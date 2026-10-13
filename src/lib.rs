pub mod connected;
pub mod json;
pub mod native;
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
};

pub const AGENTS: [Agent; 3] = [
    Agent {
        id: "elise",
        name: "Élise",
        role: "Recrutement et candidatures",
        avatar: "elise.png",
    },
    Agent {
        id: "victoire",
        name: "Victoire",
        role: "DCO, comptes rendus et documents",
        avatar: "victoire.svg",
    },
    Agent {
        id: "andrea",
        name: "Andréa",
        role: "Site internet et communication",
        avatar: "andrea.svg",
    },
];
pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    pub role: &'static str,
    pub avatar: &'static str,
}
#[derive(Clone)]
pub struct Identity {
    pub uid: String,
    pub first_name: String,
}
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: String,
}
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
    bytes: Option<&'static [u8]>,
}
impl Response {
    pub fn new(status: u16, content_type: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
            bytes: None,
        }
    }
    pub fn binary(content_type: &'static str, bytes: &'static [u8]) -> Self {
        Self {
            status: 200,
            content_type,
            body: String::new(),
            bytes: Some(bytes),
        }
    }
    pub fn write(&self, stream: &mut impl Write) -> io::Result<()> {
        let body: &[u8] = match self.bytes {
            Some(bytes) => bytes,
            None => self.body.as_bytes(),
        };
        let reason = match self.status {
            200 => "OK",
            202 => "Accepted",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            503 => "Service Unavailable",
            _ => "Error",
        };
        write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'\r\n\r\n",
            self.status,
            reason,
            self.content_type,
            body.len(),
        )?;
        stream.write_all(body)
    }
}
pub fn read_request(stream: &mut impl Read) -> Result<Request, u16> {
    let mut bytes = Vec::new();
    let mut buf = [0; 1024];
    let end = loop {
        if let Some(p) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            if p > 8192 {
                return Err(413);
            }
            break p + 4;
        }
        if bytes.len() >= 8192 {
            return Err(413);
        }
        let n = stream.read(&mut buf).map_err(|_| 400u16)?;
        if n == 0 {
            return Err(400);
        }
        bytes.extend_from_slice(&buf[..n]);
    };
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| 400u16)?;
    let mut lines = head.split("\r\n");
    let start = lines.next().ok_or(400u16)?;
    let parts = start.split(' ').collect::<Vec<_>>();
    if parts.len() != 3
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with('/')
        || parts[1].contains(['#', '\\'])
    {
        return Err(400);
    }
    let (method, path) = (
        parts[0].to_owned(),
        parts[1].split('?').next().unwrap().to_owned(),
    );
    let mut headers = BTreeMap::new();
    for line in lines.filter(|s| !s.is_empty()) {
        let (key, value) = line.split_once(':').ok_or(400u16)?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || value.chars().any(|c| c.is_control() && c != '\t')
        {
            return Err(400);
        }
        if headers
            .insert(key.to_ascii_lowercase(), value.trim().to_owned())
            .is_some()
        {
            return Err(400);
        }
    }
    if !headers.contains_key("host")
        || headers.contains_key("transfer-encoding")
        || headers.contains_key("expect")
    {
        return Err(400);
    }
    let length = match headers.get("content-length") {
        Some(s) if s.bytes().all(|b| b.is_ascii_digit()) => {
            s.parse::<usize>().map_err(|_| 400u16)?
        }
        Some(_) => return Err(400),
        None => 0,
    };
    if length > 16384 {
        return Err(413);
    }
    while bytes.len() < end + length {
        let n = stream.read(&mut buf).map_err(|_| 400u16)?;
        if n == 0 {
            return Err(400);
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    if bytes.len() != end + length {
        return Err(400);
    }
    let body = String::from_utf8(bytes[end..].to_vec()).map_err(|_| 400u16)?;
    Ok(Request {
        method,
        path,
        headers,
        body,
    })
}

pub fn valid_uid(uid: &str) -> bool {
    !uid.is_empty()
        && uid.len() <= 64
        && uid
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
pub fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn page(title: &str, content: String, base: &str, demo: bool) -> String {
    let banner = if demo {
        "Démonstration locale · identité fictive · aucun envoi à Hermes"
    } else {
        "POC socle · identité YunoHost · outils de démonstration uniquement"
    };
    format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{} · ARCenal Équipe</title><link rel=\"stylesheet\" href=\"{base}/static/theme.css\"><script src=\"{base}/static/app.js\" defer></script></head><body><a class=\"skip\" href=\"#main\">Aller au contenu</a><header><a class=\"brand\" href=\"{base}/\">ARCenal Équipe</a><label>Apparence <select id=\"theme\"><option value=\"system\">Système</option><option value=\"light\">Clair</option><option value=\"dark\">Sombre</option></select></label></header><p class=\"banner\">{banner}</p><main id=\"main\">{content}</main><footer>POC 0.1 · Avatars provisoires · Aucune fonction métier activée</footer></body></html>",
        html(title)
    )
}

pub fn route(req: &Request, identity: Option<&Identity>, base: &str, demo: bool) -> Response {
    let Some(user) = identity else {
        return Response::new(
            401,
            "text/plain; charset=utf-8",
            "Identité YunoHost requise.",
        );
    };
    let path = if base.is_empty() {
        req.path.as_str()
    } else {
        match req.path.strip_prefix(base) {
            Some(p) if p.starts_with('/') => p,
            Some("") => "/",
            _ => return Response::new(404, "text/plain", "Introuvable"),
        }
    };
    if let Some(id) = path.strip_prefix("/mcp/") {
        let Some(agent) = AGENTS.iter().find(|a| a.id == id) else {
            return Response::new(404, "text/plain", "Agent inconnu");
        };
        if req.method != "POST" {
            return Response::new(405, "text/plain", "Utiliser POST JSON-RPC");
        }
        // Cross-site browser requests are refused. MCP clients must have a trusted SSO identity too.
        if req
            .headers
            .get("sec-fetch-site")
            .is_some_and(|s| !matches!(s.as_str(), "same-origin" | "none"))
        {
            return Response::new(403, "text/plain", "Origine refusée");
        }
        if let Some(origin) = req.headers.get("origin") {
            let expected = format!(
                "{}://{}",
                if demo { "http" } else { "https" },
                req.headers.get("host").map(String::as_str).unwrap_or("")
            );
            if origin != &expected {
                return Response::new(403, "text/plain", "Origine refusée");
            }
        }
        if !req
            .headers
            .get("content-type")
            .is_some_and(|s| s.split(';').next() == Some("application/json"))
        {
            return Response::new(400, "text/plain", "JSON requis");
        }
        return mcp(&req.body, agent, user);
    }
    if req.method != "GET" {
        return Response::new(405, "text/plain", "Lecture uniquement dans ce POC");
    }
    let (ctype, body) = match path {
        "/" => {
            let cards = AGENTS.iter().map(|a|format!("<article class=\"card\"><img src=\"{base}/static/{}\" alt=\"\" width=\"180\" height=\"180\"><h2>{}</h2><p>{}</p><a class=\"button\" href=\"{base}/agents/{}\">Ouvrir l’espace de {}</a></article>",a.avatar,a.name,a.role,a.id,a.name)).collect::<String>();
            (
                "text/html; charset=utf-8",
                page(
                    "Accueil",
                    format!(
                        "<h1>Bonjour {}.</h1><p>Choisissez votre assistant.</p><div class=\"cards\">{cards}</div>",
                        html(&user.first_name)
                    ),
                    base,
                    demo,
                ),
            )
        }
        "/static/theme.css" => (
            "text/css; charset=utf-8",
            include_str!("../static/theme.css").into(),
        ),
        "/static/connected.css" => (
            "text/css; charset=utf-8",
            include_str!("../static/connected.css").into(),
        ),
        "/static/recruitment.css" => (
            "text/css; charset=utf-8",
            include_str!("../static/recruitment.css").into(),
        ),
        "/static/recruitment.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../static/recruitment.js").into(),
        ),
        "/static/app.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../static/app.js").into(),
        ),
        "/static/elise.png" => {
            return Response::binary("image/png", include_bytes!("../static/elise.png"));
        }
        "/static/victoire.svg" => (
            "image/svg+xml",
            include_str!("../static/victoire.svg").into(),
        ),
        "/static/andrea.svg" => ("image/svg+xml", include_str!("../static/andrea.svg").into()),
        "/health" => (
            "application/json",
            "{\"status\":\"ok\",\"stage\":\"poc\"}".into(),
        ),
        _ => {
            if let Some(agent) = path
                .strip_prefix("/agents/")
                .and_then(|id| AGENTS.iter().find(|a| a.id == id))
            {
                let content = format!(
                    "<a href=\"{base}/\">← Les trois agents</a><section class=\"agent\"><img src=\"{base}/static/{}\" alt=\"\" width=\"180\" height=\"180\"><div><h1>{}</h1><p>{}</p></div></section><section class=\"panel\"><h2>Conversation de démonstration</h2><p>Bonjour {}, je suis {}. Mon espace est prêt pour la validation du socle.</p><p>La conversation avec Hermes n’est pas encore configurée.</p></section><section class=\"panel\"><h2>Vérifier le MCP</h2><p>Ce test appelle mon outil de démonstration et vérifie l’identité attribuée par le serveur.</p><button id=\"mcp-test\" data-endpoint=\"{base}/mcp/{}\" data-tool=\"{}_demo_identity\">Tester la connexion MCP</button><p id=\"test-status\" role=\"status\" aria-live=\"polite\">Prêt pour le test.</p><pre id=\"test-result\" hidden></pre></section><section class=\"panel\"><h2>Profil Hermes</h2><a class=\"button\" href=\"{base}/profiles/{}.yaml\" download>Télécharger le profil de démonstration</a><p>Profil indicatif : import Hermes et accès authentifié à vérifier avant connexion réelle.</p></section><section class=\"panel\"><h2>Éléments métier</h2><p>Aucun document ni donnée métier pour le moment.</p></section>",
                    agent.avatar,
                    agent.name,
                    agent.role,
                    html(&user.first_name),
                    agent.name,
                    agent.id,
                    agent.id,
                    agent.id
                );
                (
                    "text/html; charset=utf-8",
                    page(agent.name, content, base, demo),
                )
            } else if let Some(id) = path
                .strip_prefix("/profiles/")
                .and_then(|s| s.strip_suffix(".yaml"))
            {
                let profile = match id {
                    "elise" => include_str!("../profiles/elise.yaml"),
                    "victoire" => include_str!("../profiles/victoire.yaml"),
                    "andrea" => include_str!("../profiles/andrea.yaml"),
                    _ => return Response::new(404, "text/plain", "Profil inconnu"),
                };
                ("application/yaml; charset=utf-8", profile.into())
            } else {
                return Response::new(404, "text/plain; charset=utf-8", "Page introuvable");
            }
        }
    };
    Response::new(200, ctype, body)
}

fn rpc_error(id: &str, code: i32, message: &str) -> Response {
    Response::new(
        200,
        "application/json",
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":{}}}}}",
            json::quote(message)
        ),
    )
}
fn mcp(body: &str, agent: &Agent, user: &Identity) -> Response {
    use json::Value;
    let value = match json::parse(body) {
        Ok(v) => v,
        Err(_) => return rpc_error("null", -32700, "Invalid JSON"),
    };
    let id = match value.get("id") {
        Some(v @ (Value::String(_) | Value::Number(_) | Value::Null)) => Some(v.encode()),
        None => None,
        _ => return rpc_error("null", -32600, "Invalid id"),
    };
    if value.get("jsonrpc").and_then(Value::text) != Some("2.0") {
        return rpc_error("null", -32600, "Invalid JSON-RPC request");
    }
    let Some(method) = value.get("method").and_then(Value::text) else {
        return rpc_error("null", -32600, "Missing method");
    };
    let Some(id) = id else {
        return if method == "notifications/initialized" {
            Response::new(202, "application/json", "")
        } else {
            Response::new(400, "text/plain", "Unsupported notification")
        };
    };
    let tool = format!("{}_demo_identity", agent.id);
    let result = match method {
        "initialize" => {
            let version = value
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::text)
                .unwrap_or("2025-03-26");
            let version = if matches!(version, "2024-11-05" | "2025-03-26" | "2025-06-18") {
                version
            } else {
                "2025-03-26"
            };
            format!(
                "{{\"protocolVersion\":{},\"capabilities\":{{\"tools\":{{}}}},\"serverInfo\":{{\"name\":\"arcenal-{}-poc\",\"version\":\"0.1.0\"}}}}",
                json::quote(version),
                agent.id
            )
        }
        "ping" => "{}".into(),
        "tools/list" => format!(
            "{{\"tools\":[{{\"name\":{},\"description\":\"Identité validée et agent de cette façade, sans mutation\",\"inputSchema\":{{\"type\":\"object\",\"properties\":{{}},\"additionalProperties\":false}},\"annotations\":{{\"readOnlyHint\":true}}}}]}}",
            json::quote(&tool)
        ),
        "tools/call" => {
            let params = value.get("params");
            if params.and_then(|p| p.get("name")).and_then(Value::text) != Some(&tool) {
                return rpc_error(&id, -32602, "Tool not available for this agent");
            }
            if let Some(args) = params.and_then(|p| p.get("arguments"))
                && !matches!(args,Value::Object(o) if o.is_empty())
            {
                return rpc_error(&id, -32602, "This tool accepts no identity or arguments");
            }
            let text = format!(
                "Bonjour {}. Agent : {}. Auteur LDAP : {}. MCP d’identité opérationnel.",
                user.first_name, agent.name, user.uid
            );
            format!(
                "{{\"content\":[{{\"type\":\"text\",\"text\":{}}}],\"isError\":false}}",
                json::quote(&text)
            )
        }
        _ => return rpc_error(&id, -32601, "Method not found"),
    };
    Response::new(
        200,
        "application/json",
        format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_response_preserves_non_utf8_and_content_length() {
        let payload: &'static [u8] = b"\x89PNG\r\n\x1a\n\0\xff";
        let mut output = Vec::new();
        Response::binary("image/png", payload)
            .write(&mut output)
            .unwrap();
        let end = output
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .unwrap()
            + 4;
        assert_eq!(&output[end..], payload);
        let header = std::str::from_utf8(&output[..end]).unwrap();
        assert!(header.contains("Content-Type: image/png\r\n"));
        assert!(header.contains(&format!("Content-Length: {}\r\n", payload.len())));
    }
    fn user() -> Identity {
        Identity {
            uid: "alice.ldap".into(),
            first_name: "Alice <script>".into(),
        }
    }
    fn request(path: &str) -> Request {
        Request {
            method: "GET".into(),
            path: path.into(),
            headers: BTreeMap::from([("host".into(), "localhost:8080".into())]),
            body: String::new(),
        }
    }
    #[test]
    fn private_everywhere() {
        for path in [
            "/",
            "/agents/elise",
            "/static/theme.css",
            "/profiles/elise.yaml",
            "/mcp/elise",
            "/health",
        ] {
            assert_eq!(route(&request(path), None, "", false).status, 401);
        }
    }
    #[test]
    fn home_and_prefix_and_escaping() {
        let r = route(&request("/equipe/"), Some(&user()), "/equipe", true);
        assert_eq!(r.status, 200);
        assert!(r.body.contains("Alice &lt;script&gt;"));
        for a in AGENTS {
            assert!(r.body.contains(a.name));
        }
        assert_eq!(
            route(&request("/equipement/"), Some(&user()), "/equipe", true).status,
            404
        );
    }
    #[test]
    fn parser_rejects_smuggling_and_size() {
        for raw in [
            "GET / HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n",
            "GET / HTTP/1.1\r\nHost: x\r\nTransfer-Encoding: chunked\r\n\r\n",
            "GET / HTTP/1.1\r\nHost: x\r\n\r\nextra",
        ] {
            assert!(read_request(&mut raw.as_bytes()).is_err());
        }
        let raw = "POST / HTTP/1.1\r\nHost: x\r\nContent-Length: 16385\r\n\r\n";
        assert_eq!(read_request(&mut raw.as_bytes()).err(), Some(413));
    }
    #[test]
    fn mcp_isolated_and_identity_not_supplied() {
        let mut r = request("/mcp/elise");
        r.method = "POST".into();
        r.headers
            .insert("content-type".into(), "application/json".into());
        for (name, args, ok) in [
            ("elise_demo_identity", "{}", true),
            ("andrea_demo_identity", "{}", false),
            ("elise_demo_identity", r#"{"uid":"admin"}"#, false),
        ] {
            r.body = format!(
                r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{name}","arguments":{args}}}}}"#
            );
            let v = json::parse(&route(&r, Some(&user()), "", true).body).unwrap();
            assert_eq!(v.get("result").is_some(), ok);
            if ok {
                assert!(v.encode().contains("alice.ldap"));
            }
        }
        r.headers
            .insert("origin".into(), "https://evil.test".into());
        assert_eq!(route(&r, Some(&user()), "", true).status, 403);
    }
    #[test]
    fn uid_filter() {
        for s in ["", "admin)(uid=*)", "a\n", "../../etc"] {
            assert!(!valid_uid(s));
        }
        assert!(valid_uid("alice.dupont"));
    }
    #[test]
    fn only_known_profiles_and_no_mutations() {
        assert_eq!(
            route(
                &request("/profiles/../../etc/passwd.yaml"),
                Some(&user()),
                "",
                true
            )
            .status,
            404
        );
        let mut r = request("/");
        r.method = "DELETE".into();
        assert_eq!(route(&r, Some(&user()), "", true).status, 405);
    }
}
