use arcenal_equipe::native::{
    auth::User,
    mcp_tokens,
    repository::{self, Mutation},
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn native_storage_confirmation_conflict_and_audit() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("NON_EXÉCUTÉ : TEST_DATABASE_URL requis pour le test PostgreSQL.");
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("test database available");
    sqlx::raw_sql(include_str!("../migrations/001_native.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql("TRUNCATE equipe_records,equipe_audit,equipe_confirmations,equipe_imports,equipe_files,equipe_settings,equipe_mcp_tokens").execute(&pool).await.unwrap();
    let user = User {
        uid: "test.alice".into(),
        name: "Alice".into(),
        admin: true,
        permission: true,
    };
    let job = Mutation {
        action: "job-save".into(),
        id: None,
        data: json!({"title":"Offre de test","status":"draft"}),
        version: None,
        confirmation: None,
    };
    let saved = repository::mutate(&pool, &user, job).await.unwrap();
    let id = saved["id"].as_i64().unwrap();
    let row = repository::record(&pool, "jobs", id).await.unwrap();
    assert_eq!(row["author_uid"], "test.alice");
    let mut publish = Mutation {
        action: "job-save".into(),
        id: Some(id),
        data: json!({"title":"Offre de test","status":"published"}),
        version: Some(1),
        confirmation: None,
    };
    assert!(
        repository::mutate(&pool, &user, publish.clone())
            .await
            .is_err()
    );
    let preview = repository::prepare(&pool, &user, &publish, "https://test.invalid", "/equipe")
        .await
        .unwrap();
    publish.confirmation = preview["confirmation"].as_str().map(str::to_owned);
    repository::mutate(&pool, &user, publish.clone())
        .await
        .unwrap();
    assert!(repository::mutate(&pool, &user, publish).await.is_err());
    let public: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM equipe_records WHERE kind='jobs' AND data->>'status'='published'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(public, 1);
    let archive = Mutation {
        action: "job-archive".into(),
        id: Some(id),
        data: json!({}),
        version: Some(2),
        confirmation: None,
    };
    let preview = repository::prepare(&pool, &user, &archive, "https://test.invalid", "/equipe")
        .await
        .unwrap();
    repository::mutate(
        &pool,
        &user,
        Mutation {
            confirmation: preview["confirmation"].as_str().map(str::to_owned),
            ..archive
        },
    )
    .await
    .unwrap();
    let public: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM equipe_records WHERE kind='jobs' AND data->>'status'='published'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(public, 0);
    let actors: Vec<String> = sqlx::query_scalar("SELECT DISTINCT author_uid FROM equipe_audit")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(actors, vec!["test.alice"]);
    let candidate = Mutation {
        action: "candidate-save".into(),
        id: None,
        data: json!({"first_name":"Candidat test","status":"new","in_pool":1}),
        version: None,
        confirmation: None,
    };
    repository::mutate(&pool, &user, candidate).await.unwrap();
    let data = repository::bootstrap(&pool, &user).await.unwrap();
    assert_eq!(data["source"], "arcenal");
    assert_eq!(data["candidates"].as_array().unwrap().len(), 1);
    let settings = Mutation {
        action: "settings-save".into(),
        id: None,
        data: json!({"connectors":[]}),
        version: None,
        confirmation: None,
    };
    let ordinary = User {
        admin: false,
        ..user.clone()
    };
    assert!(
        repository::mutate(&pool, &ordinary, settings)
            .await
            .is_err()
    );
    // MCP credentials belong to the authenticated LDAP user, not an argument
    // chosen by Hermes. Only hashes are persisted or returned by the listing.
    assert!(
        mcp_tokens::issue(&pool, &ordinary, "elise", "  ")
            .await
            .is_err()
    );
    let no_access = User {
        admin: false,
        permission: false,
        ..user.clone()
    };
    assert!(
        mcp_tokens::issue(&pool, &no_access, "elise", "Hermes")
            .await
            .is_err()
    );
    let connection = mcp_tokens::issue(&pool, &ordinary, "elise", "Hermes test")
        .await
        .unwrap();
    let token = connection["token"].as_str().unwrap();
    let token_id = connection["id"].as_str().unwrap();
    assert_eq!(
        mcp_tokens::owner(&pool, token, "elise").await.unwrap(),
        ordinary.uid
    );
    assert!(mcp_tokens::owner(&pool, token, "victoire").await.is_err());
    let listed = mcp_tokens::list(&pool, &ordinary.uid, "elise")
        .await
        .unwrap();
    assert_eq!(listed["connections"].as_array().unwrap().len(), 1);
    assert!(!listed.to_string().contains(token));
    assert!(listed["connections"][0].get("token_hash").is_none());
    assert_eq!(
        mcp_tokens::list(&pool, "other.ldap", "elise")
            .await
            .unwrap()["connections"],
        json!([])
    );
    assert!(
        mcp_tokens::revoke(&pool, "other.ldap", "elise", token_id)
            .await
            .is_err()
    );
    assert!(
        mcp_tokens::revoke(&pool, &ordinary.uid, "victoire", token_id)
            .await
            .is_err()
    );
    assert!(mcp_tokens::owner(&pool, token, "elise").await.is_ok());
    mcp_tokens::revoke(&pool, &ordinary.uid, "elise", token_id)
        .await
        .unwrap();
    assert!(mcp_tokens::owner(&pool, token, "elise").await.is_err());
    let expired = mcp_tokens::issue(&pool, &ordinary, "elise", "Expired test")
        .await
        .unwrap();
    sqlx::query(
        "UPDATE equipe_mcp_tokens SET expires_at=now()-interval '1 second' WHERE id=$1::text::uuid",
    )
    .bind(expired["id"].as_str().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        mcp_tokens::owner(&pool, expired["token"].as_str().unwrap(), "elise")
            .await
            .is_err()
    );
    let stored: String =
        sqlx::query_scalar("SELECT token_hash FROM equipe_mcp_tokens WHERE id=$1::text::uuid")
            .bind(token_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.len(), 64);
    assert_ne!(stored, token);
    let token_audit: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT author_uid,action,details FROM equipe_audit WHERE action LIKE 'mcp-token-%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(token_audit.len(), 3);
    assert!(
        token_audit
            .iter()
            .all(|(uid, _, details)| uid == &ordinary.uid && !details.contains(token))
    );
}
