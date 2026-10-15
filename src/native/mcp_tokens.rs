//! Standard MCP bearer credentials. Only their hashes are persisted.
use super::{App, Error, auth};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn agent_exists(agent: &str) -> Result<(), Error> {
    if !crate::AGENTS.iter().any(|entry| entry.id == agent) {
        return Err(Error::missing("Agent inconnu."));
    }
    Ok(())
}

pub fn permitted(user: &auth::User) -> Result<(), Error> {
    if !user.permission && !user.admin {
        return Err(Error::forbidden(
            "Permission YunoHost de l’application requise.",
        ));
    }
    Ok(())
}

fn digest(token: &str) -> String {
    auth::hex(&Sha256::digest(token.as_bytes()))
}

pub fn bearer(headers: &HeaderMap) -> Result<&str, Error> {
    let mut values = headers.get_all("authorization").iter();
    let value = values
        .next()
        .and_then(|v| v.to_str().ok())
        .ok_or_else(Error::unauthorized)?;
    if values.next().is_some() {
        return Err(Error::unauthorized());
    }
    let (scheme, token) = value.split_once(' ').ok_or_else(Error::unauthorized)?;
    if !scheme.eq_ignore_ascii_case("bearer")
        || !token.starts_with("arcmcp_")
        || token.len() != 71
        || !token[7..].bytes().all(|v| v.is_ascii_hexdigit())
    {
        return Err(Error::unauthorized());
    }
    Ok(token)
}

pub async fn issue(
    pool: &PgPool,
    user: &auth::User,
    agent: &str,
    label: &str,
) -> Result<Value, Error> {
    agent_exists(agent)?;
    permitted(user)?;
    let label = label.trim();
    if label.is_empty() || label.len() > 80 || label.chars().any(char::is_control) {
        return Err(Error::invalid(
            "Nom de connexion requis (80 caractères maximum).",
        ));
    }
    let token = format!(
        "arcmcp_{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let id = Uuid::new_v4();
    let expires = Utc::now() + chrono::Duration::days(90);
    let mut tx = pool.begin().await?;
    // UUID is cast as text to avoid requiring a new SQLx feature.
    sqlx::query("INSERT INTO equipe_mcp_tokens(id,token_hash,author_uid,agent,label,expires_at) VALUES ($1::text::uuid,$2,$3,$4,$5,$6)")
        .bind(id.to_string()).bind(digest(&token)).bind(&user.uid).bind(agent).bind(label).bind(expires)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO equipe_audit(author_uid,action,target,details) VALUES ($1,'mcp-token-create',$2,$3)")
        .bind(&user.uid).bind(id.to_string()).bind(format!("MCP {agent}"))
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(json!({"id":id,"token":token,"label":label,"agent":agent,"expires_at":expires}))
}

pub async fn owner(pool: &PgPool, token: &str, agent: &str) -> Result<String, Error> {
    let uid = sqlx::query_scalar("SELECT author_uid FROM equipe_mcp_tokens WHERE token_hash=$1 AND agent=$2 AND revoked_at IS NULL AND expires_at>now()")
        .bind(digest(token)).bind(agent).fetch_optional(pool).await?;
    uid.ok_or_else(Error::unauthorized)
}

pub async fn list(pool: &PgPool, uid: &str, agent: &str) -> Result<Value, Error> {
    let rows = sqlx::query("SELECT id::text,label,created_at,expires_at,revoked_at FROM equipe_mcp_tokens WHERE author_uid=$1 AND agent=$2 ORDER BY created_at DESC")
        .bind(uid).bind(agent).fetch_all(pool).await?;
    Ok(json!({"connections":rows.iter().map(|row| json!({
        "id":row.get::<String,_>("id"),"label":row.get::<String,_>("label"),
        "created_at":row.get::<DateTime<Utc>,_>("created_at"),
        "expires_at":row.get::<DateTime<Utc>,_>("expires_at"),
        "revoked_at":row.get::<Option<DateTime<Utc>>,_>("revoked_at")
    })).collect::<Vec<_>>()}))
}

pub async fn revoke(pool: &PgPool, uid: &str, agent: &str, id: &str) -> Result<(), Error> {
    let id = Uuid::parse_str(id).map_err(|_| Error::invalid("Connexion invalide."))?;
    let mut tx = pool.begin().await?;
    let changed = sqlx::query("UPDATE equipe_mcp_tokens SET revoked_at=now() WHERE id=$1::text::uuid AND author_uid=$2 AND agent=$3 AND revoked_at IS NULL")
        .bind(id.to_string()).bind(uid).bind(agent).execute(&mut *tx).await?.rows_affected();
    if changed == 0 {
        return Err(Error::missing("Connexion active introuvable."));
    }
    sqlx::query("INSERT INTO equipe_audit(author_uid,action,target,details) VALUES ($1,'mcp-token-revoke',$2,$3)")
        .bind(uid).bind(id.to_string()).bind(format!("MCP {agent}"))
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn connections(
    State(app): State<App>,
    Path(agent): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, Error> {
    agent_exists(&agent)?;
    let user = auth::user(&app, &headers).await?;
    permitted(&user)?;
    Ok(Json(list(&app.pool, &user.uid, &agent).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewConnection {
    label: String,
}

pub async fn create(
    State(app): State<App>,
    Path(agent): Path<String>,
    headers: HeaderMap,
    Json(input): Json<NewConnection>,
) -> Result<Json<Value>, Error> {
    auth::origin(&app, &headers)?;
    let user = auth::user(&app, &headers).await?;
    Ok(Json(issue(&app.pool, &user, &agent, &input.label).await?))
}

pub async fn remove(
    State(app): State<App>,
    Path((agent, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, Error> {
    auth::origin(&app, &headers)?;
    agent_exists(&agent)?;
    let user = auth::user(&app, &headers).await?;
    permitted(&user)?;
    revoke(&app.pool, &user.uid, &agent, &id).await?;
    Ok(Json(json!({"revoked":true})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_requires_one_well_formed_credential() {
        let token = format!(
            "arcmcp_{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let mut headers = HeaderMap::new();
        assert!(bearer(&headers).is_err());
        for value in [
            format!("Basic {token}"),
            "Bearer x".into(),
            format!("Bearer {token} "),
        ] {
            headers.insert("authorization", value.parse().unwrap());
            assert!(bearer(&headers).is_err());
        }
        headers.insert("authorization", format!("bEaReR {token}").parse().unwrap());
        assert_eq!(bearer(&headers).unwrap(), token);
        headers.append("authorization", format!("Bearer {token}").parse().unwrap());
        assert!(bearer(&headers).is_err());
        assert!(
            serde_json::from_value::<NewConnection>(json!({"label":"Hermes","uid":"admin"}))
                .is_err()
        );
    }
}
