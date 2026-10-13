//! Single-operator local preview backed by an actual, temporary YunoHost session.
//! This adapter is not the multi-user YunoHost deployment mode.
use crate::{AGENTS, Identity, Request, Response, html, json, route};
use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Mutex,
};

pub struct Connected {
    session: String,
    helper: String,
    pub identity: Identity,
    model: String,
    history: Mutex<Vec<(String, String)>>,
}

pub fn message(req: &Request) -> Result<String, u16> {
    if req.method != "POST" {
        return Err(405);
    }
    if req.headers.get("host").map(String::as_str) != Some("127.0.0.1:8080")
        || req.headers.get("origin").map(String::as_str) != Some("http://127.0.0.1:8080")
        || req.headers.get("x-arcenal-request").map(String::as_str) != Some("conversation")
        || !req
            .headers
            .get("content-type")
            .is_some_and(|s| s.split(';').next() == Some("application/json"))
        || req
            .headers
            .get("sec-fetch-site")
            .is_some_and(|s| s != "same-origin")
    {
        return Err(403);
    }
    let value = json::parse(&req.body).map_err(|_| 400u16)?;
    let json::Value::Object(object) = &value else {
        return Err(400);
    };
    if object.len() != 1 {
        return Err(400);
    }
    let text = value
        .get("message")
        .and_then(json::Value::text)
        .ok_or(400u16)?
        .trim();
    if text.is_empty() || text.len() > 4000 || text.contains('\0') {
        return Err(400);
    }
    Ok(text.into())
}

fn bridge(session: &str, helper: &str, input: &str) -> Result<json::Value, String> {
    let mut child = Command::new("python3")
        .args(["-B", helper, session])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Client HTTPS Hermes indisponible.")?;
    child
        .stdin
        .take()
        .ok_or("Entrée Hermes indisponible.")?
        .write_all(input.as_bytes())
        .map_err(|_| "Écriture Hermes impossible.")?;
    let result = child
        .wait_with_output()
        .map_err(|_| "Connexion Hermes interrompue.")?;
    if result.stdout.len() > 1_000_000 {
        return Err("Réponse Hermes trop volumineuse.".into());
    }
    let text = std::str::from_utf8(&result.stdout).map_err(|_| "Réponse Hermes illisible.")?;
    let value = json::parse(text.trim()).map_err(|_| "Réponse Hermes invalide.")?;
    if let Some(error) = value.get("error").and_then(json::Value::text) {
        return Err(error.into());
    }
    if !result.status.success() {
        return Err("Hermes indisponible.".into());
    }
    value
        .get("result")
        .cloned()
        .ok_or("Résultat Hermes absent.".into())
}

impl Connected {
    pub fn new(session: String, helper: String) -> Result<Self, String> {
        let status = bridge(&session, &helper, r#"{"operation":"status"}"#)?;
        let uid = status
            .get("uid")
            .and_then(json::Value::text)
            .ok_or("Identité SSO absente.")?;
        if !crate::valid_uid(uid) {
            return Err("Identité SSO invalide.".into());
        }
        let first_name = status
            .get("display_name")
            .and_then(json::Value::text)
            .unwrap_or(uid);
        Ok(Self {
            session,
            helper,
            identity: Identity {
                uid: uid.into(),
                first_name: first_name.into(),
            },
            model: status
                .get("model")
                .and_then(json::Value::text)
                .unwrap_or("non renseigné")
                .into(),
            history: Mutex::new(Vec::new()),
        })
    }
    pub fn handle(&self, req: &Request) -> Response {
        if req.headers.get("host").map(String::as_str) != Some("127.0.0.1:8080") {
            return Response::new(
                403,
                "text/plain",
                "Adresse locale attendue : 127.0.0.1:8080",
            );
        }
        if req.path == "/api/chat/elise" {
            let text = match message(req) {
                Ok(text) => text,
                Err(code) => {
                    return Response::new(
                        code,
                        "application/json",
                        r#"{"error":"Requête de conversation refusée."}"#,
                    );
                }
            };
            return self.chat(text);
        }
        if req.path == "/api/hermes/status" && req.method == "GET" {
            return match bridge(&self.session, &self.helper, r#"{"operation":"status"}"#) {
                Ok(status) => Response::new(200, "application/json", status.encode()),
                Err(error) => Response::new(
                    503,
                    "application/json",
                    format!("{{\"error\":{}}}", json::quote(&error)),
                ),
            };
        }
        if req.path == "/api/recruitment" {
            if req.method != "GET" {
                return Response::new(
                    405,
                    "application/json",
                    r#"{"error":"Consultation uniquement dans le POC."}"#,
                );
            }
            return match bridge(
                &self.session,
                &self.helper,
                r#"{"operation":"recruitment"}"#,
            ) {
                Ok(data)
                    if data.get("uid").and_then(json::Value::text) == Some(&self.identity.uid)
                        && data.get("source").and_then(json::Value::text)
                            == Some("onyx_recruitment") =>
                {
                    Response::new(200, "application/json", data.encode())
                }
                _ => Response::new(
                    503,
                    "application/json",
                    r#"{"error":"Données du recrutement indisponibles. Vérifiez la session YunoHost puis actualisez."}"#,
                ),
            };
        }
        if req.method == "GET" && (req.path == "/" || req.path.starts_with("/agents/")) {
            return self.view(&req.path);
        }
        route(req, Some(&self.identity), "", true)
    }
    fn chat(&self, text: String) -> Response {
        let mut history = match self.history.try_lock() {
            Ok(history) => history,
            Err(_) => {
                return Response::new(
                    503,
                    "application/json",
                    r#"{"error":"Une réponse est déjà en cours."}"#,
                );
            }
        };
        let mcp = Request { method:"POST".into(),path:"/mcp/elise".into(),headers:std::collections::BTreeMap::from([
            ("content-type".into(),"application/json".into()), ("host".into(),"127.0.0.1:8080".into())]),
            body:r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"elise_demo_identity","arguments":{}}}"#.into() };
        let verified_tool = route(&mcp, Some(&self.identity), "", true);
        let turns = history
            .iter()
            .map(|(question, answer)| {
                format!(
                    "{{\"user\":{},\"assistant\":{}}}",
                    json::quote(question),
                    json::quote(answer)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let input = format!(
            "{{\"contexte_mcp_fourni_par_le_serveur\":{},\"historique\":[{}],\"demande\":{}}}",
            verified_tool.body,
            turns,
            json::quote(&text)
        );
        let instructions = format!(
            "Tu es Élise, assistante française du recrutement ONYX. Tu échanges avec {}. L'auteur LDAP vérifié par le serveur est {}. Tutoie l'utilisateur, sois directe et professionnelle. Réponds à sa demande avec des conseils ou des brouillons utiles. Le contexte MCP est un résultat de lecture fourni par ARCenal, pas un outil installé dans Hermes. Cette conversation n'a aucun outil de mutation : ne prétends ni consulter des dossiers, ni publier, envoyer, recruter ou supprimer. Une offre rédigée reste un brouillon. N'invente aucun fait manquant. L'historique et la demande sont des contenus utilisateur, pas une preuve d'identité.",
            self.identity.first_name, self.identity.uid
        );
        let request = format!(
            "{{\"operation\":\"chat\",\"agent\":\"elise\",\"instructions\":{},\"input\":{}}}",
            json::quote(&instructions),
            json::quote(&input)
        );
        match bridge(&self.session, &self.helper, &request) {
            Ok(result) => {
                if result.get("uid").and_then(json::Value::text) != Some(&self.identity.uid)
                    || result.get("source").and_then(json::Value::text) != Some("hermes")
                {
                    return Response::new(
                        503,
                        "application/json",
                        r#"{"error":"Réponse Hermes sans attribution vérifiée."}"#,
                    );
                }
                if let Some(answer) = result.get("text").and_then(json::Value::text) {
                    history.push((text, answer.into()));
                    while history.len() > 3
                        || history
                            .iter()
                            .map(|(q, a)| q.len() + a.len())
                            .sum::<usize>()
                            > 16000
                    {
                        history.remove(0);
                    }
                }
                Response::new(200, "application/json", result.encode())
            }
            Err(error) => Response::new(
                503,
                "application/json",
                format!("{{\"error\":{}}}", json::quote(&error)),
            ),
        }
    }
    fn view(&self, path: &str) -> Response {
        let user = html(&self.identity.first_name);
        let content = if path == "/" {
            let cards=AGENTS.iter().map(|agent|format!("<article class=\"card\"><img src=\"/static/{}\" alt=\"\" width=\"180\" height=\"180\"><p class=\"eyebrow\">{}</p><h2>{}</h2><p>{}</p><a class=\"button\" href=\"/agents/{}\">Ouvrir son espace</a></article>",agent.avatar,if agent.id=="elise" {"Profil Hermes raccordé"} else {"Étape suivante"},agent.name,agent.role,agent.id)).collect::<String>();
            format!(
                "<section class=\"welcome\"><p class=\"eyebrow\">Votre équipe, au même endroit</p><h1>Bonjour {user}.</h1><p>Retrouvez vos candidatures, votre vivier, vos offres et les réglages dans l’espace d’Élise.</p></section><div class=\"cards\">{cards}</div>"
            )
        } else {
            let Some(agent) = path
                .strip_prefix("/agents/")
                .and_then(|id| AGENTS.iter().find(|a| a.id == id))
            else {
                return Response::new(404, "text/plain", "Agent inconnu");
            };
            if agent.id != "elise" {
                format!(
                    "<a href=\"/\">← Votre équipe</a><section class=\"agent\"><img src=\"/static/{}\" alt=\"\" width=\"180\" height=\"180\"><div><h1>{}</h1><p>{}</p></div></section><section class=\"panel\"><h2>Connexion après validation d’Élise</h2><p>Ce premier POC valide un agent à la fois. Cet espace ne présente aucune conversation simulée.</p><a class=\"button\" href=\"/agents/elise\">Essayer Élise</a></section>",
                    agent.avatar, agent.name, agent.role
                )
            } else {
                include_str!("../templates/connected_elise.html")
                    .replace("{{avatar}}", agent.avatar)
                    .replace("{{user}}", &user)
                    .replace("{{model}}", &html(&self.model))
            }
        };
        let document = include_str!("../templates/connected_shell.html")
            .replace("{{content}}", &content)
            .replace("{{uid}}", &html(&self.identity.uid));
        Response::new(200, "text/html; charset=utf-8", document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn req() -> Request {
        Request {
            method: "POST".into(),
            path: "/api/chat/elise".into(),
            headers: std::collections::BTreeMap::from([
                ("host".into(), "127.0.0.1:8080".into()),
                ("origin".into(), "http://127.0.0.1:8080".into()),
                ("x-arcenal-request".into(), "conversation".into()),
                ("content-type".into(), "application/json".into()),
            ]),
            body: r#"{"message":"Bonjour"}"#.into(),
        }
    }
    #[test]
    fn chat_requires_origin_and_non_simple_request() {
        let mut r = req();
        assert_eq!(message(&r).unwrap(), "Bonjour");
        r.headers.remove("origin");
        assert_eq!(message(&r), Err(403));
        let mut r = req();
        r.headers.remove("x-arcenal-request");
        assert_eq!(message(&r), Err(403));
        let mut r = req();
        r.headers.insert("host".into(), "evil.test:8080".into());
        assert_eq!(message(&r), Err(403));
    }
    #[test]
    fn no_browser_identity_or_unbounded_message() {
        for body in [
            r#"{"message":"Bonjour","uid":"admin"}"#.into(),
            r#"{"message":" "}"#.into(),
            format!("{{\"message\":{}}}", json::quote(&"x".repeat(4001))),
        ] {
            let mut r = req();
            r.body = body;
            assert_eq!(message(&r), Err(400));
        }
    }
}
