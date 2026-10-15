use super::{App, Error, auth};
use axum::{Json, extract::State, http::HeaderMap};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};

fn endpoint(app: &App, path: &str) -> Result<reqwest::Url, Error> {
    let raw = app.config.hermes_url.as_ref().ok_or_else(|| {
        Error::unavailable("Configurer l’adresse interne de Hermes dans YunoHost.")
    })?;
    let mut url =
        reqwest::Url::parse(raw).map_err(|_| Error::invalid("Adresse Hermes invalide."))?;
    let local = url
        .host_str()
        .is_some_and(|v| matches!(v, "127.0.0.1" | "localhost" | "::1"));
    if (url.scheme() != "https" && !local)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
    {
        return Err(Error::invalid(
            "HTTPS requis pour une connexion Hermes externe.",
        ));
    }
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    url.set_path(&format!("{}{}", url.path().trim_end_matches('/'), path));
    if !query.is_empty() {
        url.set_query(Some(query));
    }
    Ok(url)
}
pub async fn status(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    let token = app.config.hermes_token.as_ref().ok_or_else(|| {
        Error::unavailable("Configurer le jeton serveur Hermes dans le fichier protégé YunoHost.")
    })?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| Error::unavailable("Client HTTPS indisponible."))?;
    let response = client
        .get(endpoint(&app, "/api/model/info?profile=elise")?)
        .header("X-Hermes-Session-Token", token)
        .send()
        .await
        .map_err(|_| Error::unavailable("Hermes ne répond pas."))?;
    if !response.status().is_success() {
        return Err(Error::unavailable(
            "Hermes refuse l’accès ; vérifier la configuration serveur.",
        ));
    }
    let data: Value = response
        .json()
        .await
        .map_err(|_| Error::unavailable("Réponse Hermes invalide."))?;
    Ok(Json(
        json!({"connected":true,"uid":user.uid,"profile":"elise","model":data["model"],"provider":data["provider"]}),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chat {
    message: String,
}
pub async fn chat(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Chat>,
) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    auth::origin(&app, &headers)?;
    if input.message.trim().is_empty() || input.message.len() > 4000 {
        return Err(Error::invalid("Message requis, 4000 octets maximum."));
    }
    let token = app
        .config
        .hermes_token
        .as_ref()
        .ok_or_else(|| Error::unavailable("Hermes n’est pas configuré."))?;
    let mut url = endpoint(&app, "/api/ws")?;
    url.set_scheme(if url.scheme() == "https" { "wss" } else { "ws" })
        .map_err(|_| Error::invalid("Adresse WebSocket invalide."))?;
    url.query_pairs_mut().append_pair("token", token);
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|_| Error::invalid("Requête Hermes invalide."))?;
    let origin = endpoint(&app, "")?;
    request.headers_mut().insert(
        "Origin",
        format!("{}://{}", origin.scheme(), origin.authority())
            .parse()
            .map_err(|_| Error::invalid("Origine Hermes invalide."))?,
    );
    let transport = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        tokio_tungstenite::connect_async(request),
    )
    .await
    .map_err(|_| Error::unavailable("Délai de connexion Hermes dépassé."))?
    .map_err(|_| Error::unavailable("Connexion Hermes refusée."))?;
    let mut stream = transport.0;
    let instructions = format!(
        "Tu es Élise, assistante de recrutement. Tu échanges avec {}. Auteur LDAP validé : {}. Fournis des brouillons utiles en français. Ne prétends pas publier, envoyer, supprimer ou consulter des dossiers sans outil confirmé. Les actions critiques exigent une confirmation humaine dans ARCenal.",
        user.name, user.uid
    );
    // Text-only inference cannot execute undeclared built-in Hermes tools.
    stream.send(Message::Text(json!({"jsonrpc":"2.0","id":"arcenal-chat","method":"llm.oneshot","params":{"profile":"elise","instructions":instructions,"input":input.message,"max_tokens":1500}}).to_string().into())).await.map_err(|_|Error::unavailable("Envoi à Hermes impossible."))?;
    let response = tokio::time::timeout(std::time::Duration::from_secs(100), async {
        while let Some(frame) = stream.next().await {
            let frame =
                frame.map_err(|_| Error::unavailable("Conversation Hermes interrompue."))?;
            if let Message::Text(text) = frame {
                if text.len() > 1000000 {
                    return Err(Error::unavailable("Réponse Hermes trop longue."));
                }
                let value: Value = serde_json::from_str(&text)
                    .map_err(|_| Error::unavailable("Réponse Hermes invalide."))?;
                if value["id"] != "arcenal-chat" {
                    continue;
                }
                if value.get("error").is_some() {
                    return Err(Error::unavailable(
                        "Le modèle Hermes a refusé la génération ou dépassé son délai.",
                    ));
                }
                let text = value["result"]["text"]
                    .as_str()
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| Error::unavailable("Hermes n’a pas renvoyé de texte."))?;
                return Ok(text.to_owned());
            }
        }
        Err(Error::unavailable("Hermes a fermé la conversation."))
    })
    .await
    .map_err(|_| Error::unavailable("Délai de réponse Hermes dépassé."))??;
    let _ = stream.close(None).await;
    Ok(Json(
        json!({"text":response,"source":"hermes","uid":user.uid,"profile":"elise"}),
    ))
}
