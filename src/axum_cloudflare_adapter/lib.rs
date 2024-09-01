// ref: https://github.com/logankeenan/axum-cloudflare-adapter

pub use super::error::Error;
use axum::{
    body::Body,
    http::header::HeaderName,
    http::{Method, Request, Uri},
    response::Response,
};
use futures::TryStreamExt;
use std::sync::Arc;
use std::{ops::Deref, str::FromStr};
use worker::{Headers, Request as WorkerRequest, Response as WorkerResponse};

pub async fn to_axum_request(mut worker_request: WorkerRequest) -> Result<Request<Body>, Error> {
    let method = Method::from_bytes(worker_request.method().to_string().as_bytes())?;

    let uri = Uri::from_str(worker_request.url()?.to_string().as_str())?;

    let body = worker_request.bytes().await?;

    let mut http_request = Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::from(body))?;

    for (header_name, header_value) in worker_request.headers() {
        http_request.headers_mut().insert(
            HeaderName::from_str(header_name.as_str())?,
            header_value.parse()?,
        );
    }

    Ok(http_request)
}

pub async fn to_worker_response(response: Response<Body>) -> Result<WorkerResponse, Error> {
    let mut bytes: Vec<u8> = Vec::<u8>::new();

    let (parts, body) = response.into_parts();

    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.try_next().await? {
        bytes.extend_from_slice(&chunk);
    }

    let code = parts.status.as_u16();

    let mut worker_response = WorkerResponse::from_bytes(bytes)?;
    worker_response = worker_response.with_status(code);

    let mut headers = Headers::new();
    for (key, value) in parts.headers.iter() {
        headers.set(key.as_str(), value.to_str()?).unwrap()
    }
    worker_response = worker_response.with_headers(headers);

    Ok(worker_response)
}

#[derive(Clone)]
pub struct EnvWrapper {
    pub env: Arc<worker::Env>,
}

impl EnvWrapper {
    pub fn new(env: worker::Env) -> Self {
        Self { env: Arc::new(env) }
    }
}

unsafe impl Send for EnvWrapper {}

unsafe impl Sync for EnvWrapper {}
