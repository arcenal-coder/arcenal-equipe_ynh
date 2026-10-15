pub mod auth;
mod hermes;
mod mcp;
pub mod mcp_tokens;
pub mod repository;
use axum::{
    Json, Router,
    extract::Request,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, path::PathBuf, sync::Arc};
use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct Config {
    pub app_id: String,
    pub origin: String,
    pub base: String,
    pub socket: String,
    pub data_dir: PathBuf,
    pub signing_key: String,
    pub proxy_key: String,
    pub hermes_url: Option<String>,
    pub hermes_token: Option<String>,
}
#[derive(Clone)]
pub struct App {
    pub config: Arc<Config>,
    pub pool: PgPool,
}
#[derive(Debug)]
pub struct Error(pub StatusCode, pub &'static str);
impl Error {
    pub fn unauthorized() -> Self {
        Self(StatusCode::UNAUTHORIZED, "Identité YunoHost requise.")
    }
    pub fn forbidden(message: &'static str) -> Self {
        Self(StatusCode::FORBIDDEN, message)
    }
    pub fn invalid(message: &'static str) -> Self {
        Self(StatusCode::BAD_REQUEST, message)
    }
    pub fn missing(message: &'static str) -> Self {
        Self(StatusCode::NOT_FOUND, message)
    }
    pub fn conflict(message: &'static str) -> Self {
        Self(StatusCode::CONFLICT, message)
    }
    pub fn unavailable(message: &'static str) -> Self {
        Self(StatusCode::SERVICE_UNAVAILABLE, message)
    }
}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::unavailable(
            "Base de données indisponible. Aucune modification annoncée comme réussie.",
        )
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config {
        app_id: env::var("ARCENAL_APP_ID").unwrap_or_else(|_| "arcenal_equipe".into()),
        origin: env::var("ARCENAL_ORIGIN")?.trim_end_matches('/').into(),
        base: env::var("ARCENAL_BASE_PATH")
            .unwrap_or_default()
            .trim_end_matches('/')
            .into(),
        socket: env::var("ARCENAL_SOCKET")?,
        data_dir: env::var("ARCENAL_DATA_DIR")?.into(),
        signing_key: env::var("ARCENAL_SIGNING_KEY")?,
        proxy_key: env::var("ARCENAL_PROXY_KEY")?,
        hermes_url: env::var("HERMES_URL").ok().filter(|v| !v.is_empty()),
        hermes_token: env::var("HERMES_TOKEN").ok().filter(|v| !v.is_empty()),
    };
    if config.signing_key.len() < 32
        || config.proxy_key.len() < 32
        || (!config.base.is_empty()
            && (!config.base.starts_with('/')
                || !config
                    .base
                    .bytes()
                    .all(|v| v.is_ascii_alphanumeric() || b"/_-".contains(&v))))
    {
        return Err("Configuration de sécurité invalide.".into());
    }
    let origin = reqwest::Url::parse(&config.origin)?;
    if origin.scheme() != "https"
        || origin.path() != "/"
        || origin.query().is_some()
        || !origin.username().is_empty()
    {
        return Err("Origine HTTPS requise.".into());
    }
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_with(if let Ok(url) = env::var("DATABASE_URL") {
            url.parse()
                .map_err(|_| "Configuration PostgreSQL invalide.")?
        } else {
            sqlx::postgres::PgConnectOptions::new()
                .host("127.0.0.1")
                .username(&env::var("PGUSER")?)
                .password(&env::var("PGPASSWORD")?)
                .database(&env::var("PGDATABASE")?)
        })
        .await
        .map_err(|_| "Connexion PostgreSQL impossible.")?;
    sqlx::raw_sql(include_str!("../../migrations/001_native.sql"))
        .execute(&pool)
        .await
        .map_err(|_| "Migration PostgreSQL impossible.")?;
    tokio::fs::create_dir_all(config.data_dir.join("files")).await?;
    let state = App {
        config: Arc::new(config),
        pool,
    };
    let routes = Router::new()
        .route("/", get(home))
        .route("/agents/{agent}", get(agent_page))
        .route("/static/{file}", get(asset))
        .route("/api/recruitment", get(bootstrap).post(mutate))
        .route("/api/prepare", post(prepare))
        .route("/api/import", post(import))
        .route("/api/import/prepare", post(import_prepare))
        .route("/api/export", get(export))
        .route("/api/files", post(upload))
        .route("/api/files/{id}", get(file))
        .route("/api/hermes/status", get(hermes::status))
        .route("/api/chat/elise", post(hermes::chat))
        .route("/mcp/{agent}", post(mcp::call))
        .route(
            "/api/mcp/{agent}/connections",
            get(mcp_tokens::connections).post(mcp_tokens::create),
        )
        .route(
            "/api/mcp/{agent}/connections/{id}",
            axum::routing::delete(mcp_tokens::remove),
        )
        .route("/recrutement", get(public_recruitment))
        .route("/api/public/jobs", get(public_jobs))
        .route("/profiles/{name}", get(profile))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), guard));
    let router = mount(&state.config.base, routes).with_state(state.clone());
    let listener = tokio::net::UnixListener::bind(&state.config.socket)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&state.config.socket, std::fs::Permissions::from_mode(0o660))?;
    eprintln!("ARCenal Équipe native : PostgreSQL, socket privé, SSO YunoHost.");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
fn mount<S: Clone + Send + Sync + 'static>(base: &str, routes: Router<S>) -> Router<S> {
    if base.is_empty() {
        routes
    } else {
        Router::new().nest(&format!("{base}/"), routes)
    }
}

#[cfg(test)]
#[tokio::test]
async fn yunohost_subpath_matches_trailing_slash_and_api() {
    use tower::ServiceExt;
    let routes = Router::new()
        .route("/", get(|| async { "home" }))
        .route("/api/recruitment", get(|| async { "data" }));
    let router = mount("/equipe", routes);
    for uri in ["/equipe/", "/equipe/api/recruitment"] {
        let response = router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }
}
async fn guard(State(app): State<App>, request: Request, next: Next) -> Response {
    let provided = request
        .headers()
        .get("x-arcenal-proxy")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !bool::from(provided.as_bytes().ct_eq(app.config.proxy_key.as_bytes())) {
        return Error::unauthorized().into_response();
    }
    let mut response = next.run(request).await;
    for (key, value) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        (
            "content-security-policy",
            "default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'self'; form-action 'self'",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(key),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}
pub fn shell(app: &App, user: &auth::User, content: String) -> String {
    include_str!("../../templates/native_shell.html")
        .replace("{{base}}", &crate::html(&app.config.base))
        .replace("{{uid}}", &crate::html(&user.uid))
        .replace("{{content}}", &content)
}
async fn home(State(app): State<App>, headers: HeaderMap) -> Result<Html<String>, Error> {
    let user = auth::user(&app, &headers).await?;
    let cards=crate::AGENTS.iter().map(|agent|format!("<article class=\"card\"><img src=\"{}/static/{}\" alt=\"\" width=\"180\" height=\"180\"><h2>{}</h2><p>{}</p><a class=\"button\" href=\"{}/agents/{}\">Ouvrir son espace</a></article>",app.config.base,agent.avatar,agent.name,agent.role,app.config.base,agent.id)).collect::<String>();
    Ok(Html(shell(
        &app,
        &user,
        format!(
            "<h1>Bonjour {}.</h1><p>Votre équipe métier, avec vos données dans ARCenal Équipe.</p><div class=\"cards\">{cards}</div>",
            crate::html(&user.name)
        ),
    )))
}
async fn agent_page(
    State(app): State<App>,
    headers: HeaderMap,
    Path(agent): Path<String>,
) -> Result<Html<String>, Error> {
    let user = auth::user(&app, &headers).await?;
    if agent != "elise" {
        let entry = crate::AGENTS
            .iter()
            .find(|v| v.id == agent)
            .ok_or_else(|| Error::missing("Agent inconnu."))?;
        return Ok(Html(shell(
            &app,
            &user,
            format!(
                "<h1>{}</h1><p>{}</p><p>Le paquet 0.2 valide le recrutement natif. Ce domaine reste à développer après réception d’Élise.</p><a href=\"{}/agents/elise\">Ouvrir Élise</a>",
                entry.name, entry.role, app.config.base
            ),
        )));
    }
    Ok(Html(shell(
        &app,
        &user,
        include_str!("../../templates/native_elise.html")
            .replace("{{origin}}", &crate::html(&app.config.origin))
            .replace("{{base}}", &crate::html(&app.config.base))
            .replace("{{user}}", &crate::html(&user.name)),
    )))
}
async fn asset(
    State(app): State<App>,
    headers: HeaderMap,
    Path(file): Path<String>,
) -> Result<Response, Error> {
    // Neutral static resources are also needed by the public jobs page.
    let _ = headers;
    let (mime, bytes): (&str, &[u8]) = match file.as_str() {
        "elise.png" => ("image/png", include_bytes!("../../static/elise.png")),
        "victoire.svg" => ("image/svg+xml", include_bytes!("../../static/victoire.svg")),
        "andrea.svg" => ("image/svg+xml", include_bytes!("../../static/andrea.svg")),
        "theme.css" => ("text/css", include_bytes!("../../static/theme.css")),
        "connected.css" => ("text/css", include_bytes!("../../static/connected.css")),
        "recruitment.css" => ("text/css", include_bytes!("../../static/recruitment.css")),
        "app.js" => ("text/javascript", include_bytes!("../../static/app.js")),
        "recruitment.js" => (
            "text/javascript",
            include_bytes!("../../static/recruitment.js"),
        ),
        "native.js" => ("text/javascript", include_bytes!("../../static/native.js")),
        _ => return Err(Error::missing("Ressource inconnue.")),
    };
    let _ = app;
    Ok(([("content-type", mime)], bytes.to_owned()).into_response())
}
async fn bootstrap(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    Ok(Json(repository::bootstrap(&app.pool, &user).await?))
}
async fn mutate(
    State(app): State<App>,
    headers: HeaderMap,
    Json(value): Json<repository::Mutation>,
) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    auth::origin(&app, &headers)?;
    Ok(Json(repository::mutate(&app.pool, &user, value).await?))
}
async fn prepare(
    State(app): State<App>,
    headers: HeaderMap,
    Json(value): Json<repository::Mutation>,
) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    auth::origin(&app, &headers)?;
    Ok(Json(
        repository::prepare(
            &app.pool,
            &user,
            &value,
            &app.config.origin,
            &app.config.base,
        )
        .await?,
    ))
}
async fn import(
    State(app): State<App>,
    headers: HeaderMap,
    Json(value): Json<repository::Import>,
) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    auth::origin(&app, &headers)?;
    let claims = auth::verify(
        &app.config.signing_key,
        value.confirmation.as_deref().unwrap_or(""),
        "import",
    )?;
    if claims.uid != user.uid || claims.nonce != repository::import_digest(&value) {
        return Err(Error::forbidden("Confirmation d’import invalide."));
    }
    Ok(Json(repository::import(&app.pool, &user, value).await?))
}
async fn import_prepare(
    State(app): State<App>,
    headers: HeaderMap,
    Json(value): Json<repository::Import>,
) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    auth::origin(&app, &headers)?;
    if !user.admin {
        return Err(Error::forbidden(
            "Import réservé à un administrateur YunoHost.",
        ));
    }
    let claims = auth::Claims {
        uid: user.uid,
        agent: "import".into(),
        expires: chrono::Utc::now().timestamp() + 120,
        nonce: repository::import_digest(&value),
    };
    Ok(Json(
        json!({"confirmation":auth::sign(&app.config.signing_key,&claims),"destination":format!("{}{}/recrutement",app.config.origin,app.config.base),"published_jobs":value.jobs.iter().filter(|job|job["status"]=="published").collect::<Vec<_>>()}),
    ))
}
async fn export(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    if !user.admin {
        return Err(Error::forbidden(
            "Export réservé à un administrateur YunoHost.",
        ));
    }
    Ok(Json(
        json!({"candidates":repository::list(&app.pool,"candidates").await?,"jobs":repository::list(&app.pool,"jobs").await?,"settings":repository::settings(&app.pool).await?}),
    ))
}
async fn public_jobs(State(app): State<App>) -> Result<Json<Value>, Error> {
    let rows:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'title',data->'title','location',data->'location','contract',data->'contract','description',data->'description','missions',data->'missions','skills',data->'skills') FROM equipe_records WHERE kind='jobs' AND data->>'status'='published' ORDER BY id DESC").fetch_all(&app.pool).await?;
    Ok(Json(json!({"jobs":rows})))
}
async fn public_recruitment(State(app): State<App>) -> Result<Html<String>, Error> {
    let Json(data) = public_jobs(State(app.clone())).await?;
    let cards=data["jobs"].as_array().unwrap().iter().map(|job|format!("<article class=\"panel\"><h2>{}</h2><p>{} · {}</p><p>{}</p><h3>Missions</h3><pre>{}</pre></article>",crate::html(job["title"].as_str().unwrap_or("")),crate::html(job["location"].as_str().unwrap_or("")),crate::html(job["contract"].as_str().unwrap_or("")),crate::html(job["description"].as_str().unwrap_or("")),crate::html(job["missions"].as_str().unwrap_or("")))).collect::<String>();
    let empty = if cards.is_empty() {
        "<p>Aucune offre publiée pour le moment.</p>"
    } else {
        ""
    };
    Ok(Html(format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Recrutement · ARCenal Équipe</title><link rel=\"stylesheet\" href=\"{}/static/theme.css\"></head><body><main><h1>Nos offres d’emploi</h1>{empty}{cards}</main></body></html>",
        app.config.base
    )))
}
async fn profile(
    State(app): State<App>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Response, Error> {
    auth::user(&app, &headers).await?;
    let text = match name.as_str() {
        "elise.yaml" => include_str!("../../profiles/elise.yaml"),
        "victoire.yaml" => include_str!("../../profiles/victoire.yaml"),
        "andrea.yaml" => include_str!("../../profiles/andrea.yaml"),
        _ => return Err(Error::missing("Profil inconnu.")),
    };
    Ok(([("content-type", "application/yaml")], text).into_response())
}

async fn upload(
    State(app): State<App>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, Error> {
    let user = auth::user(&app, &headers).await?;
    auth::origin(&app, &headers)?;
    let mut candidate = None;
    let mut file_data = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| Error::invalid("Fichier invalide."))?
    {
        if field.name() == Some("candidate_id") {
            candidate = field.text().await.ok().and_then(|v| v.parse::<i64>().ok());
        } else if field.name() == Some("file") {
            let original = field.file_name().unwrap_or("cv.pdf").to_owned();
            let data = field
                .bytes()
                .await
                .map_err(|_| Error::invalid("Téléversement impossible."))?;
            if data.len() > 5 * 1024 * 1024 || !data.starts_with(b"%PDF-") {
                return Err(Error::invalid("CV PDF uniquement, 5 Mo maximum."));
            }
            file_data = Some((original, data));
        } else {
            return Err(Error::invalid("Champ de téléversement inconnu."));
        }
    }
    let candidate = candidate
        .filter(|v| *v > 0)
        .ok_or_else(|| Error::invalid("Candidat requis."))?;
    let (original, data) = file_data.ok_or_else(|| Error::invalid("Fichier requis."))?;
    if original.len() > 255 || original.chars().any(char::is_control) {
        return Err(Error::invalid("Nom de fichier invalide."));
    }
    let mut tx = app.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(172386)")
        .execute(&mut *tx)
        .await?;
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM equipe_records WHERE kind='candidates' AND id=$1 FOR UPDATE",
    )
    .bind(candidate)
    .fetch_optional(&mut *tx)
    .await?;
    if exists.is_none() {
        return Err(Error::missing("Candidat introuvable."));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let path = app.config.data_dir.join("files").join(&id);
    tokio::fs::write(&path, &data)
        .await
        .map_err(|_| Error::unavailable("Stockage de fichier indisponible."))?;
    let stored=async{
        sqlx::query("INSERT INTO equipe_files(id,candidate_id,original_name,size,author_uid) VALUES($1,$2,$3,$4,$5)").bind(&id).bind(candidate).bind(&original).bind(data.len() as i64).bind(&user.uid).execute(&mut *tx).await?;
        sqlx::query("UPDATE equipe_records SET data=data||jsonb_build_object('cv_original',$1::text,'cv_file_id',$2::text),author_uid=$3,version=version+1,updated_at=now() WHERE kind='candidates' AND id=$4").bind(&original).bind(&id).bind(&user.uid).bind(candidate).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO equipe_audit(author_uid,action,target,details) VALUES($1,'upload',$2,'CV PDF ajouté')").bind(&user.uid).bind(format!("candidate:{candidate}")).execute(&mut *tx).await?;
        tx.commit().await
    }.await;
    if stored.is_err() {
        let _ = tokio::fs::remove_file(path).await;
        return Err(Error::unavailable("Enregistrement du fichier impossible."));
    }
    Ok(Json(json!({"ok":true,"id":id})))
}
async fn file(
    State(app): State<App>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Error> {
    auth::user(&app, &headers).await?;
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err(Error::missing("Fichier inconnu."));
    }
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM equipe_files f JOIN equipe_records r ON r.kind='candidates' AND r.id=f.candidate_id WHERE f.id=$1)").bind(&id).fetch_one(&app.pool).await?;
    if !exists {
        return Err(Error::missing("Fichier inconnu."));
    }
    let bytes = tokio::fs::read(app.config.data_dir.join("files").join(id))
        .await
        .map_err(|_| Error::missing("Fichier indisponible."))?;
    Ok((
        [
            ("content-type", "application/pdf"),
            ("content-disposition", "attachment; filename=\"cv.pdf\""),
        ],
        bytes,
    )
        .into_response())
}
