use super::{App, Error, auth, mcp_tokens, repository};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

fn tools(agent: &str) -> Vec<Value> {
    let empty = json!({"type":"object","properties":{},"additionalProperties":false});
    let identity = json!({"name":format!("{agent}_identity"),"description":"Identité LDAP du propriétaire de la connexion MCP, vérifiée par ARCenal Équipe.","inputSchema":empty,"annotations":{"readOnlyHint":true}});
    if agent != "elise" {
        return vec![identity];
    }
    let job = json!({"type":"object","properties":{
        "title":{"type":"string","minLength":1,"maxLength":300},
        "location":{"type":"string"},"contract":{"type":"string"},
        "description":{"type":"string"},"context":{"type":"string"},
        "objective":{"type":"string"},"missions":{"type":"string"},"skills":{"type":"string"},
        "salary_min":{"type":"number","minimum":0},"salary_max":{"type":"number","minimum":0},
        "status":{"type":"string","enum":["draft"]}
    },"required":["title"],"additionalProperties":false});
    let candidate = json!({"type":"object","properties":{
        "first_name":{"type":"string"},"last_name":{"type":"string"},"email":{"type":"string"},
        "phone":{"type":"string"},"position":{"type":"string"},"location":{"type":"string"},
        "experience":{"type":"string"},"motivation":{"type":"string"},"notes":{"type":"string"},
        "source":{"type":"string"},"mobility":{"type":"string"},"salary_expectation":{"type":"string"},
        "status":{"type":"string","enum":repository::STATUSES},"in_pool":{"type":"integer","enum":[0,1]}
    },"anyOf":[{"required":["first_name"]},{"required":["last_name"]}],"additionalProperties":false});
    vec![
        identity,
        json!({"name":"elise_candidates_list","description":"Lister les candidats et leurs étapes dans le recrutement ARCenal Équipe.","inputSchema":empty,"annotations":{"readOnlyHint":true}}),
        json!({"name":"elise_jobs_list","description":"Lister les offres ARCenal Équipe, y compris les brouillons.","inputSchema":empty,"annotations":{"readOnlyHint":true}}),
        json!({"name":"elise_job_draft_create","description":"Créer une offre brouillon. Ne publie pas l’offre.","inputSchema":{"type":"object","properties":{"data":job},"required":["data"],"additionalProperties":false},"annotations":{"readOnlyHint":false,"destructiveHint":false}}),
        json!({"name":"elise_candidate_create","description":"Créer un dossier candidat dans ARCenal Équipe. Fournir au moins un nom ou un prénom.","inputSchema":{"type":"object","properties":{"data":candidate},"required":["data"],"additionalProperties":false},"annotations":{"readOnlyHint":false,"destructiveHint":false}}),
    ]
}

async fn execute(
    app: &App,
    user: &auth::User,
    agent: &str,
    name: &str,
    args: &Value,
) -> Result<Value, Error> {
    mcp_tokens::permitted(user)?;
    match name {
        "elise_candidates_list" => Ok(json!(repository::list(&app.pool, "candidates").await?)),
        "elise_jobs_list" => Ok(json!(repository::list(&app.pool, "jobs").await?)),
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
                user,
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
            .await
        }
        _ => Ok(json!({"uid":user.uid,"first_name":user.name,"agent":agent})),
    }
}

pub async fn call(
    State(app): State<App>,
    Path(agent): Path<String>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Response, Error> {
    mcp_tokens::agent_exists(&agent)?;
    if headers
        .get("origin")
        .is_some_and(|v| v.as_bytes() != app.config.origin.as_bytes())
    {
        return Err(Error::forbidden("Origine MCP refusée."));
    }
    let credential = match mcp_tokens::bearer(&headers) {
        Ok(token) => token,
        Err(_) => return Ok(challenge("Jeton MCP requis.")),
    };
    let uid = match mcp_tokens::owner(&app.pool, credential, &agent).await {
        Ok(uid) => uid,
        Err(error) if error.0 == StatusCode::UNAUTHORIZED => {
            return Ok(challenge("Jeton MCP invalide, révoqué ou expiré."));
        }
        Err(error) => return Err(error),
    };
    let user = match auth::directory(&uid, &app.config.app_id).await {
        Ok(user) => user,
        Err(error) if error.0 == StatusCode::UNAUTHORIZED => {
            return Ok(challenge("Utilisateur LDAP indisponible."));
        }
        Err(error) => return Err(error),
    };
    mcp_tokens::permitted(&user)?;
    dispatch(&app, &agent, &user, request).await
}

fn challenge(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [("www-authenticate", "Bearer realm=\"arcenal-mcp\"")],
        Json(json!({"error":message})),
    )
        .into_response()
}

async fn dispatch(
    app: &App,
    agent: &str,
    user: &auth::User,
    request: Value,
) -> Result<Response, Error> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    if request["jsonrpc"] != "2.0"
        || request["method"].as_str().is_none()
        || !(id.is_null() || id.is_string() || id.is_i64() || id.is_u64())
    {
        return Ok(Json(
            json!({"jsonrpc":"2.0","id":id,"error":{"code":-32600,"message":"Requête JSON-RPC invalide"}}),
        ).into_response());
    }
    // Streamable HTTP notifications must be acknowledged without a JSON-RPC body.
    if request.get("id").is_none() {
        return Ok(StatusCode::ACCEPTED.into_response());
    }
    let method = request["method"].as_str().unwrap_or("");
    let tools = tools(agent);
    let result = match method {
        "initialize" => {
            let protocol = match request["params"]["protocolVersion"].as_str() {
                Some("2024-11-05") => "2024-11-05",
                _ => "2025-03-26",
            };
            json!({"protocolVersion":protocol,"capabilities":{"tools":{}},"serverInfo":{"name":format!("arcenal-{agent}"),"version":env!("CARGO_PKG_VERSION")}})
        }
        "notifications/initialized" | "ping" => json!({}),
        "tools/list" => {
            json!({"tools":tools})
        }
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            if !tools.iter().any(|tool| tool["name"] == name) {
                return Ok(Json(
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Outil indisponible pour cet agent"}}),
                ).into_response());
            }
            let empty = json!({});
            let args = request["params"].get("arguments").unwrap_or(&empty);
            let write = matches!(name, "elise_job_draft_create" | "elise_candidate_create");
            if args
                .as_object()
                .is_none_or(|m| m.keys().any(|k| k != "data" || !write))
            {
                return Ok(Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Arguments MCP non autorisés"}})).into_response());
            }
            match execute(app, user, agent, name, args).await {
                Ok(value) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                }
                Err(error) => json!({"content":[{"type":"text","text":error.1}],"isError":true}),
            }
        }
        _ => {
            return Ok(Json(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Méthode inconnue"}}),
            ).into_response());
        }
    };
    Ok(Json(json!({"jsonrpc":"2.0","id":id,"result":result})).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::Config;
    use std::sync::Arc;

    fn test_app() -> App {
        App {
            config: Arc::new(Config {
                app_id: "arcenal_equipe".into(),
                origin: "https://example.test".into(),
                base: "/equipe".into(),
                socket: String::new(),
                data_dir: Default::default(),
                signing_key: "test-key".into(),
                proxy_key: "test-proxy".into(),
                hermes_url: None,
                hermes_token: None,
            }),
            pool: sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://localhost/unused")
                .unwrap(),
        }
    }

    #[tokio::test]
    async fn http_discovery_notifications_and_bearer_refusal() {
        let app = test_app();
        let user = auth::User {
            uid: "alice.ldap".into(),
            name: "Alice".into(),
            admin: false,
            permission: true,
        };
        for method in ["notifications/initialized", "notifications/cancelled"] {
            let response = dispatch(
                &app,
                "elise",
                &user,
                json!({"jsonrpc":"2.0","method":method}),
            )
            .await
            .unwrap();
            assert_eq!(response.status(), StatusCode::ACCEPTED);
            assert!(
                axum::body::to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
        for method in ["initialize", "tools/list", "ping", "unknown"] {
            let response = dispatch(
                &app,
                "elise",
                &user,
                json!({"jsonrpc":"2.0","id":7,"method":method}),
            )
            .await
            .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), 16384)
                .await
                .unwrap();
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["id"], 7);
            if method == "unknown" {
                assert_eq!(body["error"]["code"], -32601);
            } else {
                assert!(body.get("result").is_some());
            }
        }
        for method in ["initialize", "tools/list", "tools/call"] {
            let response = call(
                State(app.clone()), Path("elise".into()), HeaderMap::new(),
                Json(json!({"jsonrpc":"2.0","id":8,"method":method,"params":{"name":"elise_candidates_list","arguments":{}}})),
            ).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(response.headers().contains_key("www-authenticate"));
        }
        for args in [
            json!({"_arc_context":"forged"}),
            json!({"uid":"admin"}),
            json!({"data":{}}),
        ] {
            let response = dispatch(
                &app, "elise", &user,
                json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"elise_identity","arguments":args}}),
            ).await.unwrap();
            let bytes = axum::body::to_bytes(response.into_body(), 16384)
                .await
                .unwrap();
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["error"]["code"], -32602);
        }
        let identity = execute(&app, &user, "elise", "elise_identity", &json!({}))
            .await
            .unwrap();
        assert_eq!(identity["uid"], "alice.ldap");
        let denied = auth::User {
            permission: false,
            ..user.clone()
        };
        assert!(
            execute(&app, &denied, "elise", "elise_identity", &json!({}))
                .await
                .is_err()
        );
        assert!(
            execute(
                &app,
                &user,
                "elise",
                "elise_job_draft_create",
                &json!({"data":{"title":"Test","status":"published"}})
            )
            .await
            .is_err()
        );
        assert!(
            execute(
                &app,
                &user,
                "elise",
                "elise_candidate_create",
                &json!({"data":{"first_name":"Test","uid":"admin"}})
            )
            .await
            .is_err()
        );
        for agent in ["elise", "victoire", "andrea"] {
            assert!(!json!(tools(agent)).to_string().contains("_arc_context"));
        }
        let response = dispatch(&app, "elise", &user, json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"elise_identity","arguments":{}}})).await.unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), 16384)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["result"]["isError"], false);
        let identity: Value =
            serde_json::from_str(body["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(identity["uid"], user.uid);
    }

    #[tokio::test]
    async fn postgres_mcp_tools_attribute_writes_and_refuse_publication() {
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            eprintln!("NON_EXÉCUTÉ : TEST_DATABASE_URL requis pour les outils MCP PostgreSQL.");
            return;
        };
        let mut app = test_app();
        app.pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/001_native.sql"))
            .execute(&app.pool)
            .await
            .unwrap();
        let user = auth::User {
            uid: "test.mcp.owner".into(),
            name: "MCP test".into(),
            admin: false,
            permission: true,
        };
        let connection = mcp_tokens::issue(&app.pool, &user, "elise", "MCP test")
            .await
            .unwrap();
        assert_eq!(
            mcp_tokens::owner(&app.pool, connection["token"].as_str().unwrap(), "elise")
                .await
                .unwrap(),
            user.uid
        );
        for (name, data, kind) in [
            (
                "elise_job_draft_create",
                json!({"title":"MCP integration test"}),
                "jobs",
            ),
            (
                "elise_candidate_create",
                json!({"first_name":"MCP integration test"}),
                "candidates",
            ),
        ] {
            let response = dispatch(&app, "elise", &user, json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":{"data":data}}})).await.unwrap();
            let bytes = axum::body::to_bytes(response.into_body(), 16384)
                .await
                .unwrap();
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["result"]["isError"], false);
            let saved: Value =
                serde_json::from_str(body["result"]["content"][0]["text"].as_str().unwrap())
                    .unwrap();
            let row = repository::record(&app.pool, kind, saved["id"].as_i64().unwrap())
                .await
                .unwrap();
            assert_eq!(row["author_uid"], user.uid);
            if kind == "jobs" {
                assert_eq!(row["status"], "draft");
            }
        }
        let response = dispatch(&app, "elise", &user, json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"elise_job_draft_create","arguments":{"data":{"title":"Must not publish","status":"published"}}}})).await.unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), 16384)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["result"]["isError"], true);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM equipe_records WHERE author_uid=$1 AND data->>'status'='published'").bind(&user.uid).fetch_one(&app.pool).await.unwrap();
        assert_eq!(count, 0);
    }
}
