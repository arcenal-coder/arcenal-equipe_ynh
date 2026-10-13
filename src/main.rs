use arcenal_equipe::{Identity, Request, Response, read_request, route, valid_uid};
use std::{
    env,
    io::{Read, Write},
    net::TcpListener,
    os::unix::net::{UnixListener, UnixStream},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone)]
struct Config {
    demo: bool,
    base: String,
    identity_helper: String,
    connected: Option<Arc<arcenal_equipe::connected::Connected>>,
}
fn serve(mut stream: impl Read + Write, config: &Config) {
    let request = read_request(&mut stream);
    let malformed = request.is_err();
    let response = match request {
        Err(code) => Response::new(code, "text/plain", "Requête invalide ou trop volumineuse"),
        Ok(req) => {
            if let Some(connected) = &config.connected {
                let _ = connected.handle(&req).write(&mut stream);
                return;
            }
            let identity = resolve_identity(&req, config);
            match identity {
                Ok(user) => route(&req, user.as_ref(), &config.base, config.demo),
                Err(()) => Response::new(
                    503,
                    "text/plain; charset=utf-8",
                    "Annuaire LDAP indisponible ; accès refusé.",
                ),
            }
        }
    };
    let _ = response.write(&mut stream);
    if malformed {
        // Drain a bounded tail after sending the error. Closing TCP with unread
        // upload bytes can otherwise reset the connection before 413 is received.
        let _ = std::io::copy(&mut Read::take(&mut stream, 32768), &mut std::io::sink());
    }
}
fn resolve_identity(req: &Request, config: &Config) -> Result<Option<Identity>, ()> {
    if config.demo {
        return Ok(Some(Identity {
            uid: "demo.local".into(),
            first_name: "Camille".into(),
        }));
    }
    // Only reachable via the permission-protected Unix socket. SSOwat overwrites YNH_*.
    let Some(uid) = req.headers.get("ynh_user") else {
        return Ok(None);
    };
    if !valid_uid(uid) {
        return Ok(None);
    }
    let output = Command::new("/usr/bin/timeout")
        .args(["5", "/usr/bin/python3", &config.identity_helper, uid])
        .output()
        .map_err(|_| ())?;
    if !output.status.success() || output.stdout.len() > 4096 {
        return Err(());
    }
    let first_name = String::from_utf8(output.stdout)
        .map_err(|_| ())?
        .trim()
        .to_owned();
    if first_name.is_empty() || first_name.chars().any(char::is_control) {
        return Err(());
    }
    Ok(Some(Identity {
        uid: uid.clone(),
        first_name,
    }))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args == ["serve"] {
        return arcenal_equipe::native::run().await;
    }
    let demo = args == ["--demo"];
    let connected_mode = args == ["--connected"];
    if !args.is_empty() && !demo && !connected_mode {
        return Err("Usage : arcenal-equipe [--demo | --connected]".into());
    }
    let base = env::var("ARCENAL_BASE_PATH")
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_owned();
    if !base.is_empty()
        && (!base.starts_with('/')
            || !base
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/-_".contains(&b)))
    {
        return Err("ARCENAL_BASE_PATH invalide".into());
    }
    let connected = if connected_mode {
        if !base.is_empty() {
            return Err("Le POC connecté local attend un chemin racine.".into());
        }
        Some(Arc::new(arcenal_equipe::connected::Connected::new(
            env::var("ARCENAL_SESSION_FILE")?,
            env::var("ARCENAL_HERMES_HELPER").unwrap_or_else(|_| "lib/hermes_bridge.py".into()),
        )?))
    } else {
        None
    };
    let config = Arc::new(Config {
        demo,
        base,
        identity_helper: env::var("ARCENAL_IDENTITY_HELPER")
            .unwrap_or_else(|_| "/var/www/arcenal_equipe/lib/identity.py".into()),
        connected,
    });
    let active = Arc::new(AtomicUsize::new(0));
    if demo || connected_mode {
        let listener = TcpListener::bind("127.0.0.1:8080")?;
        eprintln!(
            "POC local : http://127.0.0.1:8080{}/ — {}",
            config.base,
            if connected_mode {
                "session YunoHost vérifiée, Hermes réel"
            } else {
                "identité fictive Camille"
            }
        );
        for stream in listener.incoming() {
            let mut stream = stream?;
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            if active.load(Ordering::Relaxed) >= 16 {
                let _ = Response::new(503, "text/plain", "Serveur occupé").write(&mut stream);
                continue;
            }
            active.fetch_add(1, Ordering::Relaxed);
            let count = active.clone();
            let conf = config.clone();
            std::thread::spawn(move || {
                serve(stream, &conf);
                count.fetch_sub(1, Ordering::Relaxed);
            });
        }
    } else {
        let path =
            env::var("ARCENAL_SOCKET").unwrap_or_else(|_| "/run/arcenal_equipe/app.sock".into());
        // Refuse an existing socket; systemd owns and cleans RuntimeDirectory.
        let listener = UnixListener::bind(&path)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o660))?;
        eprintln!("POC YunoHost : socket privé {path}");
        for stream in listener.incoming() {
            let mut stream: UnixStream = stream?;
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            if active.load(Ordering::Relaxed) >= 16 {
                let _ = Response::new(503, "text/plain", "Serveur occupé").write(&mut stream);
                continue;
            }
            active.fetch_add(1, Ordering::Relaxed);
            let count = active.clone();
            let conf = config.clone();
            std::thread::spawn(move || {
                serve(stream, &conf);
                count.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }
    Ok(())
}
