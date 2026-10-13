use super::{App, Error};
use axum::http::HeaderMap;
use chrono::Utc;
use hmac::{Hmac, Mac};
use ldap3::{LdapConn, Scope, SearchEntry};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[derive(Clone, Debug, Serialize)]
pub struct User {
    pub uid: String,
    pub name: String,
    pub admin: bool,
    pub permission: bool,
}

pub async fn directory(uid: &str, app_id: &str) -> Result<User, Error> {
    if !crate::valid_uid(uid) {
        return Err(Error::unauthorized());
    }
    let uid = uid.to_owned();
    let app_id = app_id.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut connection = LdapConn::with_settings(
            ldap3::LdapConnSettings::new().set_conn_timeout(std::time::Duration::from_secs(3)),
            "ldap://127.0.0.1:389",
        )
        .map_err(|_| Error::unavailable("Annuaire YunoHost indisponible."))?;
        let (rows, _) = connection
            .search(
                "ou=users,dc=yunohost,dc=org",
                Scope::Subtree,
                &format!("(&(objectClass=posixAccount)(uid={uid}))"),
                vec!["givenName", "displayName", "cn", "memberOf", "permission"],
            )
            .and_then(|r| r.success())
            .map_err(|_| Error::unavailable("Lecture LDAP impossible."))?;
        let _ = connection.unbind();
        if rows.len() != 1 {
            return Err(Error::unauthorized());
        }
        let entry = SearchEntry::construct(rows.into_iter().next().unwrap());
        let attribute = |key: &str| {
            entry
                .attrs
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let name = ["givenName", "displayName", "cn"]
            .iter()
            .flat_map(|key| attribute(key))
            .find(|v| !v.trim().is_empty())
            .unwrap_or_else(|| uid.clone());
        if name.len() > 512 || name.chars().any(char::is_control) {
            return Err(Error::unauthorized());
        }
        let admin = attribute("memberOf")
            .iter()
            .any(|v| v.starts_with("cn=admins,"));
        let permission = attribute("permission")
            .iter()
            .any(|v| v.starts_with(&format!("cn={app_id}.main,")));
        Ok(User {
            uid,
            name,
            admin,
            permission,
        })
    })
    .await
    .map_err(|_| Error::unavailable("Annuaire YunoHost indisponible."))?
}

pub async fn user(app: &App, headers: &HeaderMap) -> Result<User, Error> {
    // The service binds only a private Unix socket; SSOwat clears/replaces YNH_*.
    let uid = headers
        .get("ynh_user")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(Error::unauthorized)?;
    directory(uid, &app.config.app_id).await
}

pub fn origin(app: &App, headers: &HeaderMap) -> Result<(), Error> {
    if headers.get("origin").and_then(|v| v.to_str().ok()) != Some(&app.config.origin)
        || headers
            .get("x-arcenal-request")
            .and_then(|v| v.to_str().ok())
            != Some("native")
        || headers
            .get("sec-fetch-site")
            .is_some_and(|v| v != "same-origin")
    {
        return Err(Error::forbidden("Requête intersite refusée."));
    }
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub uid: String,
    pub agent: String,
    pub expires: i64,
    pub nonce: String,
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || text.len() > 8192 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}
pub fn sign(key: &str, claims: &Claims) -> String {
    let payload = serde_json::to_vec(claims).expect("claims are serializable");
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC accepts key");
    mac.update(&payload);
    format!("{}.{}", hex(&payload), hex(&mac.finalize().into_bytes()))
}
pub fn verify(key: &str, token: &str, agent: &str) -> Result<Claims, Error> {
    if !token.is_ascii() || token.len() > 4096 {
        return Err(Error::unauthorized());
    }
    let (payload, signature) = token.split_once('.').ok_or_else(Error::unauthorized)?;
    let payload = unhex(payload).ok_or_else(Error::unauthorized)?;
    let signature = unhex(signature).ok_or_else(Error::unauthorized)?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|_| Error::unauthorized())?;
    mac.update(&payload);
    mac.verify_slice(&signature)
        .map_err(|_| Error::unauthorized())?;
    let claims: Claims = serde_json::from_slice(&payload).map_err(|_| Error::unauthorized())?;
    if claims.agent != agent
        || !crate::valid_uid(&claims.uid)
        || claims.expires < Utc::now().timestamp()
        || claims.expires > Utc::now().timestamp() + 900
    {
        return Err(Error::unauthorized());
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delegation_cannot_change_author_agent_or_expiry() {
        let claims = Claims {
            uid: "alice.ldap".into(),
            agent: "elise".into(),
            expires: Utc::now().timestamp() + 600,
            nonce: "unique".into(),
        };
        let token = sign("test-key", &claims);
        assert!(verify("test-key", &token, "elise").is_ok());
        assert!(verify("test-key", &token, "andrea").is_err());
        assert!(verify("wrong-key", &token, "elise").is_err());
        let expired = Claims {
            expires: 0,
            ..claims
        };
        assert!(verify("test-key", &sign("test-key", &expired), "elise").is_err());
    }
}
