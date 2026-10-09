use hyper::Method;

use crate::generic_api::{ChatCompletionOutput, GenericApi, InferError};

// re-export
pub use crate::generic_api::{Streaming, TextOnly};
pub use crate::generic_models::*;

pub struct Ollama {
    api: GenericApi,
}

impl Ollama {
    /// A local Ollama instance.
    ///
    /// The URI is usually `http://localhost:11434/v1`
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
        request: ChatCompletionRequest,
    ) -> Result<Token::Output, InferError> {
        let response = self
            .api
            .request(Method::POST, "chat/completions", &request)
            .await?;
        Token::transform_response(response).await
    }
}
