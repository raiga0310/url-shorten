use axum_cloudflare_adapter::EnvWrapper;
use axum_cloudflare_adapter::{to_axum_request, to_worker_response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use worker::*;

use axum::{
    extract::{self, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse as _, Redirect},
    routing::{get, post},
    Router as AxumRouter,
};

use tower_service::Service;

#[derive(Clone)]
pub struct AxumState {
    pub env_wrapper: EnvWrapper,
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let mut router: AxumRouter = AxumRouter::new()
        .route("/", get(index))
        .route("/create", post(create))
        .route("/:key", get(redirect))
        .with_state(AxumState {
            env_wrapper: EnvWrapper::new(env),
        });

    let axum_request = to_axum_request(req).await.unwrap();
    let axum_response = router.call(axum_request).await.unwrap();
    let response = to_worker_response(axum_response).await.unwrap();

    Ok(response)
}

#[axum_wasm_macros::wasm_compat]
async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateKey {
    pub url: String,
}

#[axum_wasm_macros::wasm_compat]
async fn create(
    State(state): State<AxumState>,
    extract::Json(payload): extract::Json<CreateKey>,
) -> impl axum::response::IntoResponse {
    use std::ops::Deref;

    use serde_json::json;

    let env: &Env = state.env_wrapper.env.deref();
    let kv = env.kv("kv-url").unwrap();
    let url = payload.url.clone();

    let key = Uuid::new_v4().to_string();

    let _ = kv.put(&key, url).unwrap();

    let body = json!({ "key": key });

    (StatusCode::CREATED, axum::Json(body))
}

#[axum_wasm_macros::wasm_compat]
#[worker::send]
async fn redirect(
    State(state): State<AxumState>,
    Path(key): Path<String>,
) -> axum::http::Response<axum::body::Body> {
    use std::ops::Deref;

    let env: &Env = state.env_wrapper.env.deref();
    let kv = env.kv("kv-url").unwrap();

    let url = (kv.get(&key).text().await).unwrap_or_default();
    let url = match url {
        Some(url) => url,
        None => "/".to_string(),
    };
    Redirect::to(&url).into_response()
}
