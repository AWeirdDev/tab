use http_body_util::{BodyExt, Full};
use hyper::{
    Method, Request, Response, StatusCode, Uri,
    body::{Bytes, Incoming},
    header,
};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use serde::Serialize;

type HttpClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

#[derive(Debug, thiserror::Error)]
pub enum InferError {
    #[error("invalid api url: {0}")]
    InvalidUri(#[from] hyper::http::uri::InvalidUri),

    #[error("invalid request: {0}")]
    Http(#[from] hyper::http::Error),

    #[error("request failed: {0}")]
    Client(#[from] hyper_util::client::legacy::Error),

    #[error("failed to read response body: {0}")]
    Body(#[from] hyper::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("unexpected status {status}: {body}")]
    Status { status: StatusCode, body: String },
}

#[derive(Clone)]
pub struct GenericApi {
    pub(crate) client: HttpClient,
    pub(crate) api_key: Box<str>,
    pub(crate) api_url: Box<str>,
}

impl GenericApi {
    pub fn new(api_key: impl Into<Box<str>>, api_url: impl AsRef<str>) -> Result<Self, InferError> {
        let api_url = api_url
            .as_ref()
            .trim_end_matches('/')
            .to_owned()
            .into_boxed_str();
        api_url.parse::<Uri>()?;

        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();

        Ok(Self {
            client: Client::builder(TokioExecutor::new()).build(connector),
            api_key: api_key.into(),
            api_url,
        })
    }

    pub(crate) async fn request<B>(
        &self,
        method: Method,
        route: impl AsRef<str>,
        body: &B,
    ) -> Result<Response<Incoming>, InferError>
    where
        B: Serialize + ?Sized,
    {
        let uri = format!(
            "{}/{}",
            self.api_url.trim_end_matches('/'),
            route.as_ref().trim_start_matches('/')
        );
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .body(Full::new(Bytes::from(serde_json::to_vec(&body)?)))?;

        let response = self.client.request(request).await?;

        let status = response.status();

        if !status.is_success() {
            let bytes = response.into_body().collect().await?.to_bytes();
            return Err(InferError::Status {
                status,
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }

        Ok(response)
    }
}
