use super::{Error, auth::User};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};

pub const CANDIDATE_FIELDS: &[&str] = &[
    "first_name",
    "last_name",
    "email",
    "phone",
    "position",
    "location",
    "experience",
    "motivation",
    "status",
    "source",
    "notes",
    "assigned_to",
    "in_pool",
    "tags",
    "mobility",
    "salary_expectation",
    "job_id",
    "newsletter_opt_in",
];
pub const JOB_FIELDS: &[&str] = &[
    "title",
    "location",
    "contract",
    "description",
    "context",
    "objective",
    "missions",
    "skills",
    "status",
    "salary_min",
    "salary_max",
    "apec_location_code",
    "apec_travel_zone",
    "apec_status_job",
    "apec_remote_work",
    "apec_duration_months",
    "dolibarr_ref",
    "dolibarr_type",
    "contacts",
];
pub const STATUSES: [&str; 6] = [
    "new",
    "screening",
    "interview",
    "offer",
    "hired",
    "rejected",
];
pub fn statuses() -> Value {
    json!({"new":"Nouveau","screening":"À qualifier","interview":"Entretien","offer":"Proposition","hired":"Recruté","rejected":"Non retenu"})
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub action: String,
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub confirmation: Option<String>,
}
pub fn validate(m: &Mutation) -> Result<(), Error> {
    if m.id.is_some_and(|id| id <= 0) {
        return Err(Error::invalid("Identifiant invalide."));
    }
    match m.action.as_str() {
        "candidate-save" | "job-save" => {
            let map = m
                .data
                .as_object()
                .ok_or_else(|| Error::invalid("Données invalides."))?;
            let fields = if m.action == "candidate-save" {
                CANDIDATE_FIELDS
            } else {
                JOB_FIELDS
            };
            if map.keys().any(|k| !fields.contains(&k.as_str())) || m.data.to_string().len() > 40000
            {
                return Err(Error::invalid(
                    "Champ non autorisé ou données trop longues.",
                ));
            }
            if m.action == "job-save" {
                if m.data["title"]
                    .as_str()
                    .is_none_or(|s| s.trim().is_empty() || s.len() > 300)
                {
                    return Err(Error::invalid("Intitulé requis."));
                }
                if !["draft", "published", "closed", "archived"]
                    .contains(&m.data["status"].as_str().unwrap_or("draft"))
                {
                    return Err(Error::invalid("Statut d’offre invalide."));
                }
                for key in ["salary_min", "salary_max"] {
                    if let Some(v) = m.data.get(key).filter(|v| !v.is_null())
                        && v.as_f64().is_none_or(|v| !v.is_finite() || v < 0.0)
                    {
                        return Err(Error::invalid("Salaire invalide."));
                    }
                }
            } else {
                if ["first_name", "last_name"]
                    .iter()
                    .all(|k| m.data[k].as_str().is_none_or(|s| s.trim().is_empty()))
                {
                    return Err(Error::invalid("Nom ou prénom requis."));
                }
                if !STATUSES.contains(&m.data["status"].as_str().unwrap_or("new")) {
                    return Err(Error::invalid("Étape invalide."));
                }
            }
        }
        "candidate-delete" | "candidate-archive" | "job-delete" | "job-archive" => {
            if m.id.is_none() {
                return Err(Error::invalid("Cible requise."));
            }
        }
        "settings-save" => {
            if !m.data.is_object() || m.data.to_string().len() > 100000 || contains_secret(&m.data)
            {
                return Err(Error::invalid(
                    "Réglages invalides : aucun secret brut n’est accepté.",
                ));
            }
        }
        _ => return Err(Error::invalid("Action inconnue.")),
    }
    Ok(())
}
fn contains_secret(v: &Value) -> bool {
    match v {
        Value::Object(m) => m.iter().any(|(k, v)| {
            ["password", "token", "api_key", "secret"].contains(&k.as_str()) || contains_secret(v)
        }),
        Value::Array(a) => a.iter().any(contains_secret),
        _ => false,
    }
}
pub fn kind(action: &str) -> &'static str {
    if action.starts_with("candidate-") {
        "candidates"
    } else {
        "jobs"
    }
}
pub async fn record(pool: &PgPool, kind: &str, id: i64) -> Result<Value, Error> {
    let value=sqlx::query_scalar::<_,Value>("SELECT data || jsonb_build_object('id',id,'version',version,'author_uid',author_uid,'updated_at',updated_at,'created_at',COALESCE(data->>'created_at',created_at::text)) FROM equipe_records WHERE kind=$1 AND id=$2")
        .bind(kind).bind(id).fetch_optional(pool).await?;
    value.ok_or_else(|| Error::missing("Dossier introuvable."))
}
pub async fn list(pool: &PgPool, kind: &str) -> Result<Vec<Value>, Error> {
    Ok(sqlx::query_scalar("SELECT data || jsonb_build_object('id',id,'version',version,'author_uid',author_uid,'updated_at',updated_at,'created_at',COALESCE(data->>'created_at',created_at::text)) FROM equipe_records WHERE kind=$1 ORDER BY id DESC").bind(kind).fetch_all(pool).await?)
}
pub async fn settings(pool: &PgPool) -> Result<Value, Error> {
    Ok(sqlx::query_scalar::<_,Value>("SELECT data FROM equipe_settings WHERE singleton=true").fetch_optional(pool).await?.unwrap_or_else(||json!({"connectors":[],"rejectionTemplate":{},"newsletterTemplate":{},"eliseTraining":{}})))
}
pub async fn bootstrap(pool: &PgPool, user: &User) -> Result<Value, Error> {
    let activity:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('actor',author_uid,'action',action,'details',details,'created_at',COALESCE(legacy_created_at,created_at::text),'target',target) FROM equipe_audit ORDER BY id DESC LIMIT 100").fetch_all(pool).await?;
    let mut config = settings(pool).await?;
    for key in [
        "connectors",
        "rejectionTemplate",
        "newsletterTemplate",
        "eliseTraining",
    ] {
        if config.get(key).is_none() {
            config[key] = if key == "connectors" {
                json!([])
            } else {
                json!({})
            };
        }
    }
    if let Some(connectors) = config.get_mut("connectors").and_then(Value::as_array_mut) {
        for connector in connectors {
            if let Some(object) = connector.as_object_mut() {
                object.remove("secret_ref");
                object.remove("endpoint");
            }
        }
    }
    Ok(
        json!({"source":"arcenal","uid":user.uid,"editable":true,"admin":user.admin,"statuses":statuses(),"candidates":list(pool,"candidates").await?,"jobs":list(pool,"jobs").await?,"settings":config,"activity":activity}),
    )
}
fn digest(m: &Mutation, uid: &str, version: i64) -> String {
    let mut value = m.clone();
    value.confirmation = None;
    super::auth::hex(&Sha256::digest(format!(
        "{uid}:{version}:{}",
        serde_json::to_string(&value).unwrap()
    )))
}
pub async fn prepare(
    pool: &PgPool,
    user: &User,
    m: &Mutation,
    origin: &str,
    base: &str,
) -> Result<Value, Error> {
    validate(m)?;
    let current = if let Some(id) = m.id {
        record(pool, kind(&m.action), id).await?
    } else {
        json!({"version":0})
    };
    let nonce = uuid::Uuid::new_v4().to_string();
    let hash = digest(m, &user.uid, current["version"].as_i64().unwrap_or(0));
    sqlx::query("INSERT INTO equipe_confirmations(nonce,author_uid,request_hash,expires_at) VALUES($1,$2,$3,now()+interval '2 minutes')").bind(&nonce).bind(&user.uid).bind(hash).execute(pool).await?;
    Ok(
        json!({"confirmation":nonce,"action":m.action,"target":m.id,"destination":format!("{origin}{base}/recrutement"),"title":m.data.get("title").or(current.get("title")),"content":m.data,"expires_in":120}),
    )
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    user: &User,
    action: &str,
    target: &str,
    details: &str,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO equipe_audit(author_uid,action,target,details) VALUES($1,$2,$3,$4)")
        .bind(&user.uid)
        .bind(action)
        .bind(target)
        .bind(details)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn mutate(pool: &PgPool, user: &User, m: Mutation) -> Result<Value, Error> {
    validate(&m)?;
    if m.action == "settings-save" && !user.admin {
        return Err(Error::forbidden(
            "Les réglages nécessitent un administrateur YunoHost.",
        ));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(172386)")
        .execute(&mut *tx)
        .await?;
    let existing = if let Some(id) = m.id {
        sqlx::query("SELECT data,version FROM equipe_records WHERE kind=$1 AND id=$2 FOR UPDATE")
            .bind(kind(&m.action))
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| Error::missing("Cible introuvable."))?
            .into()
    } else {
        None
    };
    let version = existing
        .as_ref()
        .map(|r: &sqlx::postgres::PgRow| r.get::<i64, _>("version"))
        .unwrap_or(0);
    if m.id.is_some() && m.version != Some(version) {
        return Err(Error::conflict(
            "Le dossier a changé. Actualisez avant d’enregistrer.",
        ));
    }
    let old = existing
        .as_ref()
        .map(|r| r.get::<Value, _>("data"))
        .unwrap_or_else(|| json!({}));
    let critical = m.action.ends_with("delete")
        || m.action.ends_with("archive")
        || (m.action == "job-save"
            && (m.data["status"] == "published"
                || old["status"] == "published"
                || m.data["status"] == "archived"));
    if critical {
        let token = m
            .confirmation
            .as_ref()
            .ok_or_else(|| Error::conflict("Confirmation humaine requise."))?;
        let consumed=sqlx::query("UPDATE equipe_confirmations SET used_at=now() WHERE nonce=$1 AND author_uid=$2 AND request_hash=$3 AND expires_at>now() AND used_at IS NULL").bind(token).bind(&user.uid).bind(digest(&m,&user.uid,version)).execute(&mut *tx).await?;
        if consumed.rows_affected() != 1 {
            return Err(Error::forbidden(
                "Confirmation invalide, expirée ou déjà utilisée.",
            ));
        }
    }
    let mut id = m.id;
    if m.action == "settings-save" {
        sqlx::query("INSERT INTO equipe_settings(singleton,data,author_uid) VALUES(true,$1,$2) ON CONFLICT(singleton) DO UPDATE SET data=excluded.data,author_uid=excluded.author_uid,updated_at=now()").bind(&m.data).bind(&user.uid).execute(&mut *tx).await?;
    } else if m.action.ends_with("delete") {
        sqlx::query("DELETE FROM equipe_records WHERE kind=$1 AND id=$2")
            .bind(kind(&m.action))
            .bind(id)
            .execute(&mut *tx)
            .await?;
    } else {
        let mut value = old;
        if m.action.ends_with("archive") {
            if kind(&m.action) == "jobs" {
                value["status"] = json!("archived");
            } else {
                value["archived_at"] = json!(Utc::now().to_rfc3339());
            }
        } else {
            for (k, v) in m.data.as_object().unwrap() {
                value[k] = v.clone();
            }
            if value.get("status").is_none_or(Value::is_null) {
                value["status"] = json!(if kind(&m.action) == "jobs" {
                    "draft"
                } else {
                    "new"
                });
            }
        }
        if let Some(record_id) = id {
            sqlx::query("UPDATE equipe_records SET data=$1,author_uid=$2,version=version+1,updated_at=now() WHERE kind=$3 AND id=$4").bind(value).bind(&user.uid).bind(kind(&m.action)).bind(record_id).execute(&mut *tx).await?;
        } else {
            id=Some(sqlx::query_scalar::<_,i64>("INSERT INTO equipe_records(kind,data,author_uid) VALUES($1,$2,$3) RETURNING id").bind(kind(&m.action)).bind(value).bind(&user.uid).fetch_one(&mut *tx).await?);
        }
    }
    let target = format!("{}:{}", kind(&m.action), id.unwrap_or(0));
    audit(
        &mut tx,
        user,
        &m.action,
        &target,
        "Modification native confirmée par l’application",
    )
    .await?;
    tx.commit().await?;
    Ok(json!({"ok":true,"id":id,"action":m.action}))
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub candidates: Vec<Value>,
    pub jobs: Vec<Value>,
    #[serde(default)]
    pub settings: Value,
    #[serde(default)]
    pub activity: Vec<Value>,
    #[serde(default)]
    pub confirmation: Option<String>,
}
pub fn import_digest(data: &Import) -> String {
    let mut data = data.clone();
    data.confirmation = None;
    super::auth::hex(&Sha256::digest(
        serde_json::to_vec(&data).expect("import is serializable"),
    ))
}
pub async fn import(pool: &PgPool, user: &User, data: Import) -> Result<Value, Error> {
    if !user.admin {
        return Err(Error::forbidden(
            "Import réservé à un administrateur YunoHost.",
        ));
    }
    let digest = import_digest(&data);
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(172386)")
        .execute(&mut *tx)
        .await?;
    if sqlx::query_scalar::<_, i64>("SELECT count(*) FROM equipe_records")
        .fetch_one(&mut *tx)
        .await?
        != 0
    {
        return Err(Error::conflict(
            "L’import initial exige une base vide pour préserver les données déjà enregistrées.",
        ));
    }
    if contains_secret(&data.settings) {
        return Err(Error::invalid(
            "L’export contient des secrets bruts ; import refusé.",
        ));
    }
    for (kind, rows) in [("jobs", &data.jobs), ("candidates", &data.candidates)] {
        for row in rows {
            let id = row["id"]
                .as_i64()
                .filter(|id| *id > 0)
                .ok_or_else(|| Error::invalid("Identifiant d’import invalide."))?;
            if !row.is_object() {
                return Err(Error::invalid("Ligne d’import invalide."));
            }
            let mut value = row.clone();
            value.as_object_mut().unwrap().remove("id");
            value.as_object_mut().unwrap().remove("author_uid");
            value.as_object_mut().unwrap().remove("version");
            for key in ["cv_stored", "cover_letter_stored"] {
                value.as_object_mut().unwrap().remove(key);
            }
            sqlx::query("INSERT INTO equipe_records(kind,id,data,author_uid) VALUES($1,$2,$3,$4)")
                .bind(kind)
                .bind(id)
                .bind(value)
                .bind(&user.uid)
                .execute(&mut *tx)
                .await?;
        }
    }
    sqlx::query("SELECT setval('equipe_record_id',GREATEST(COALESCE((SELECT max(id) FROM equipe_records),0),1))").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO equipe_settings(singleton,data,author_uid) VALUES(true,$1,$2) ON CONFLICT(singleton) DO UPDATE SET data=excluded.data,author_uid=excluded.author_uid").bind(data.settings).bind(&user.uid).execute(&mut *tx).await?;
    for event in &data.activity {
        let actor = event["actor"].as_str().unwrap_or("historique");
        sqlx::query("INSERT INTO equipe_audit(author_uid,action,target,details,legacy_created_at) VALUES($1,$2,'import-historique',$3,$4)").bind(actor).bind(event["action"].as_str().unwrap_or("import")).bind(event["details"].as_str().unwrap_or("")).bind(event["created_at"].as_str()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO equipe_imports(digest,author_uid) VALUES($1,$2)")
        .bind(digest)
        .bind(&user.uid)
        .execute(&mut *tx)
        .await?;
    audit(
        &mut tx,
        user,
        "import",
        "initial",
        "Reprise initiale transactionnelle des métadonnées ; pièces jointes à reprendre séparément",
    )
    .await?;
    tx.commit().await?;
    Ok(json!({"ok":true,"candidates":data.candidates.len(),"jobs":data.jobs.len()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mutation_refuses_identity_injection_and_invalid_status() {
        let invalid = Mutation {
            action: "candidate-save".into(),
            id: None,
            data: json!({"first_name":"Alice","uid":"admin"}),
            version: None,
            confirmation: None,
        };
        assert!(validate(&invalid).is_err());
        let invalid = Mutation {
            data: json!({"first_name":"Alice","status":"admin"}),
            ..invalid
        };
        assert!(validate(&invalid).is_err());
    }
    #[test]
    fn confirmation_hash_binds_author_version_and_content() {
        let m = Mutation {
            action: "job-save".into(),
            id: Some(1),
            data: json!({"title":"Offre","status":"published"}),
            version: Some(1),
            confirmation: None,
        };
        assert_ne!(digest(&m, "alice", 1), digest(&m, "bob", 1));
        assert_ne!(digest(&m, "alice", 1), digest(&m, "alice", 2));
    }
}
