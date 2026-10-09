use async_stream::try_stream;
use async_trait::async_trait;
use futures_core::{Stream, stream::BoxStream};
use http_body_util::BodyExt;
use hyper::{Method, Response, body::Incoming};
use serde::Serialize;

use crate::generic_api::{GenericApi, InferError};

// re-exports
pub use crate::generic_models::*;

/// API compatible with ClosedAI.
pub struct OpenAiCompat {
    api: GenericApi,
}

impl OpenAiCompat {
    /// A local instance.
    ///
    /// For Ollama, the URI is usually `http://localhost:11434/v1`.
    pub fn new_local(uri: &str) -> Result<Self, InferError> {
        Ok(Self {
            api: GenericApi::new("x", uri)?,
        })
    }

    /// Perform a chat completion.
    ///
    /// # Params
    /// - `request`: The chat completion request.
    ///
    /// # Type Params
    /// - `Token`: The chat completion output token.
    ///
    /// To get a text-only response, use [`TextOnly`]; to get a streaming response,
    /// use [`Streaming`].
    pub async fn chat_completion<Token: ChatCompletionOutput>(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<Token::Output, InferError> {
        #[derive(Serialize)]
        struct StreamingRequest<'a> {
            #[serde(flatten)]
            request: &'a ChatCompletionRequest,
            stream: bool,
        }

        let response = self
            .api
            .request(
                Method::POST,
                "chat/completions",
                &StreamingRequest {
                    request,
                    stream: Token::STREAM,
                },
            )
            .await?;
        Token::transform_response(response).await
    }
}

#[async_trait]
pub trait ChatCompletionOutput {
    const STREAM: bool;
    type Output;
    async fn transform_response(response: Response<Incoming>) -> Result<Self::Output, InferError>;
}

pub struct TextOnlyCompletion;

#[async_trait]
impl ChatCompletionOutput for TextOnlyCompletion {
    const STREAM: bool = false;
    type Output = ChatCompletion;

    async fn transform_response(response: Response<Incoming>) -> Result<Self::Output, InferError> {
        let bytes = response.into_body().collect().await?.to_bytes();
        Ok(serde_json::from_slice::<ChatCompletion>(&bytes)?)
    }
}

type StreamingInner<'a> = BoxStream<'a, Result<ChatCompletionChunk, InferError>>;
pub struct StreamingCompletion;

#[async_trait]
impl ChatCompletionOutput for StreamingCompletion {
    const STREAM: bool = true;
    type Output = StreamingInner<'static>;

    async fn transform_response(response: Response<Incoming>) -> Result<Self::Output, InferError> {
        let body = response.into_body();
        Ok(Box::pin(chunks(body)))
    }
}

fn chunks(
    mut body: Incoming,
) -> impl Stream<Item = Result<ChatCompletionChunk, InferError>> + Send {
    try_stream! {
        let mut buf = Vec::new();

        while let Some(frame) = body.frame().await {
            let Ok(data) = frame?.into_data() else { continue };
            buf.extend_from_slice(&data);

            while let Some(end) = buf.windows(2).position(|w| w == b"\n\n") {
                let event: Vec<u8> = buf.drain(..end + 2).collect();
                let event = String::from_utf8_lossy(&event);

                for line in event.lines() {
                    let Some(payload) = line.strip_prefix("data:").map(str::trim) else {
                        continue;
                    };

                    if payload == "[DONE]" {
                        return;
                    }

                    yield serde_json::from_str(payload)?;
                }
            }
        }
    }
}
