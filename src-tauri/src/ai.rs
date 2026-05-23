use crate::config::{AppConfig, Provider};
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ContentPart};
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ANSWER_PROMPT: &str =
    "Look at this screenshot. Find the question, quiz item, or problem that needs an answer. \
     Reply with ONLY the briefest possible answer — a few words, a number, or the exact option \
     letter/text to select. No explanation, no punctuation unless part of the answer.";

#[derive(Debug, Error)]
pub enum AiError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub ok: bool,
    pub message: String,
}

pub async fn validate_config(config: &AppConfig) -> ValidationResult {
    match validate_config_inner(config).await {
        Ok(()) => ValidationResult {
            ok: true,
            message: "API key is valid.".into(),
        },
        Err(e) => ValidationResult {
            ok: false,
            message: e.to_string(),
        },
    }
}

async fn validate_config_inner(config: &AppConfig) -> Result<(), AiError> {
    let client = client_for_config(config)?;
    let chat_req = ChatRequest::default().append_message(ChatMessage::user("Reply with only: ok"));
    let options = ChatOptions::default().with_max_tokens(8).with_temperature(0.0);
    client
        .exec_chat(&config.model(), chat_req, Some(&options))
        .await
        .map_err(ai_error)?;
    Ok(())
}

pub async fn answer_from_screenshot(
    config: &AppConfig,
    jpeg_base64: &str,
) -> Result<String, AiError> {
    let client = client_for_config(config)?;
    let chat_req = ChatRequest::default().append_message(ChatMessage::user(vec![
        ContentPart::from_text(ANSWER_PROMPT),
        ContentPart::from_binary_base64("image/jpeg", jpeg_base64, Some("screenshot.jpg".into())),
    ]));
    let options = ChatOptions::default().with_max_tokens(64).with_temperature(0.0);
    let response = client
        .exec_chat(&config.model(), chat_req, Some(&options))
        .await
        .map_err(ai_error)?;
    response
        .first_text()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AiError::Message("Model returned an empty response.".into()))
}

fn client_for_config(config: &AppConfig) -> Result<Client, AiError> {
    let api_key = config.api_key.clone();
    let auth_resolver =
        AuthResolver::from_resolver_fn(move |_model: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
            Ok(Some(AuthData::from_single(api_key.clone())))
        });

    let mut builder = Client::builder().with_auth_resolver(auth_resolver);

    if config.provider == Provider::Openrouter || config.base_url.is_some() {
        let endpoint_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| {
                if config.provider == Provider::Openrouter {
                    "https://openrouter.ai/api/v1/".to_string()
                } else {
                    "https://api.openai.com/v1/".to_string()
                }
            });
        let key = config.api_key.clone();
        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            move |target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                Ok(ServiceTarget {
                    endpoint: Endpoint::from_owned(endpoint_url.clone()),
                    auth: AuthData::from_single(key.clone()),
                    model: ModelIden::new(AdapterKind::OpenAI, target.model.model_name),
                })
            },
        );
        builder = builder.with_service_target_resolver(target_resolver);
    }

    Ok(builder.build())
}

fn ai_error(err: genai::Error) -> AiError {
    AiError::Message(err.to_string())
}
