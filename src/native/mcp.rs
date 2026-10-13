use super::{App, Error, auth, repository};
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::{Value, json};

pub async fn call(
    State(app): State<App>,
    Path(agent): Path<String>,
    Json(request): Json<Value>,
) -> Result<Json<Value>, Error> {
    if !crate::AGENTS.iter().any(|v| v.id == agent) {
        return Err(Error::missing("Agent inconnu."));
    }
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    if request["jsonrpc"] != "2.0" {
        return Ok(Json(
            json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":"Requête JSON-RPC invalide"}}),
        ));
    }
    let method = request["method"].as_str().unwrap_or("");
    let tools = if agent == "elise" {
        vec![
            "elise_identity",
            "elise_candidates_list",
            "elise_jobs_list",
            "elise_job_draft_create",
            "elise_candidate_create",
        ]
    } else if agent == "victoire" {
        vec!["victoire_identity"]
    } else {
        vec!["andrea_identity"]
    };
    let result = match method {
        "initialize" => {
            json!({"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":format!("arcenal-{agent}"),"version":"0.2.0"}})
        }
        "notifications/initialized" | "ping" => json!({}),
        "tools/list" => {
            json!({"tools":tools.iter().map(|name|json!({"name":name,"description":"Outil ARCenal avec identité LDAP déléguée et vérifiée côté serveur","inputSchema":{"type":"object","properties":{"_arc_context":{"type":"string","description":"Contexte signé fourni exclusivement par ARCenal"},"data":{"type":"object"}},"required":["_arc_context"],"additionalProperties":false}})).collect::<Vec<_>>()})
        }
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            if !tools.contains(&name) {
                return Ok(Json(
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Outil indisponible pour cet agent"}}),
                ));
            }
            let args = &request["params"]["arguments"];
            if args.as_object().is_none_or(|m| {
                m.keys()
                    .any(|k| !matches!(k.as_str(), "_arc_context" | "data"))
            }) {
                return Err(Error::invalid("Arguments MCP non autorisés."));
            }
            let claims = auth::verify(
                &app.config.signing_key,
                args["_arc_context"].as_str().unwrap_or(""),
                &agent,
            )?;
            let user = auth::directory(&claims.uid, &app.config.app_id).await?;
            if !user.permission && !user.admin {
                return Err(Error::forbidden(
                    "Permission YunoHost de l’application requise.",
                ));
            }
            let value = match name {
                "elise_candidates_list" => json!(repository::list(&app.pool, "candidates").await?),
                "elise_jobs_list" => json!(repository::list(&app.pool, "jobs").await?),
                "elise_job_draft_create" | "elise_candidate_create" => {
                    let mut data = args["data"].clone();
                    if !data.is_object() {
                        return Err(Error::invalid("Données métier requises."));
                    }
                    if name == "elise_job_draft_create" {
                        if data.get("status").is_some_and(|v| v != "draft") {
                            return Err(Error::forbidden(
                                "Un agent ne publie pas sans confirmation humaine.",
                            ));
                        }
                        data["status"] = json!("draft");
                    }
                    repository::mutate(
                        &app.pool,
                        &user,
                        repository::Mutation {
                            action: if name == "elise_candidate_create" {
                                "candidate-save".into()
                            } else {
                                "job-save".into()
                            },
                            id: None,
                            data,
                            version: None,
                            confirmation: None,
                        },
                    )
                    .await?
                }
                _ => json!({"uid":user.uid,"first_name":user.name,"agent":agent}),
            };
            json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
        }
        _ => {
            return Ok(Json(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Méthode inconnue"}}),
            ));
        }
    };
    Ok(Json(json!({"jsonrpc":"2.0","id":id,"result":result})))
}
