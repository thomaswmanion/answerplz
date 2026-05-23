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

const CLIPBOARD_ANSWER_PROMPT: &str =
    "The following text was copied from the clipboard. Find the question, quiz item, or problem \
     that needs an answer. Reply with ONLY the briefest possible answer — a few words, a number, \
     or the exact option letter/text to select. No explanation unless essential.";

const BRIEF_ANSWER_INSTRUCTION: &str =
    "Reply with ONLY the briefest possible answer — a few words, a number, or the exact option \
     letter/text to select. No explanation unless essential.";

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
    exec_chat(&client, config, chat_req, 64).await
}

pub async fn answer_question(config: &AppConfig, question: &str) -> Result<String, AiError> {
    let question = question.trim();
    if question.is_empty() {
        return Err(AiError::Message("Question is empty.".into()));
    }
    let user_message = format!("{BRIEF_ANSWER_INSTRUCTION}\n\nQuestion:\n{question}");
    exec_text_message(config, &user_message, 256).await
}

pub async fn answer_from_clipboard_text(
    config: &AppConfig,
    clipboard_text: &str,
) -> Result<String, AiError> {
    let text = clipboard_text.trim();
    if text.is_empty() {
        return Err(AiError::Message("Clipboard is empty or has no text.".into()));
    }
    let user_message = format!("{CLIPBOARD_ANSWER_PROMPT}\n\n{text}");
    exec_text_message(config, &user_message, 256).await
}

async fn exec_text_message(
    config: &AppConfig,
    user_message: &str,
    max_tokens: u32,
) -> Result<String, AiError> {
    let client = client_for_config(config)?;
    let chat_req = ChatRequest::default().append_message(ChatMessage::user(user_message));
    exec_chat(&client, config, chat_req, max_tokens).await
}

async fn exec_chat(
    client: &Client,
    config: &AppConfig,
    chat_req: ChatRequest,
    max_tokens: u32,
) -> Result<String, AiError> {
    let options = ChatOptions::default()
        .with_max_tokens(max_tokens)
        .with_temperature(0.0);
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
