use crate::config::{AppConfig, Provider};
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ContentPart};
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! answering_rules {
    () => {
        "\n\nWork through the problem using your knowledge and whatever is visible (text, images, \
diagrams, choices). Reason internally before deciding — read the full question and every option.\n\
- Apply accurate domain knowledge (science, safety, navigation, history, math, etc.); do not guess \
when facts apply.\n\
- Images & diagrams: base your answer on what is actually shown (colors, labels, positions, arrows).\n\
- Ordering or procedures: use the correct real-world sequence, then give the order the question asks for.\n\
- Multiple choice: evaluate each option and pick the single best match.\n\
- If evidence is weak or ambiguous, still answer but use a lower confidence score.\n\
\n\
Output format (strict): `<confidence>%, <answer>`\n\
- `<confidence>`: integer 0–100 — how sure you are after reasoning.\n\
- `<answer>`: the final answer only (few words, number, letter, direction, or comma-separated order).\n\
No explanation, labels, or extra text."
    };
}

pub const DEFAULT_ANSWER_PROMPT: &str = concat!(
    "Look at this screenshot. Find the question, quiz item, or problem that needs an answer.",
    answering_rules!(),
);

const CLIPBOARD_ANSWER_PROMPT: &str = concat!(
    "The following text was copied from the clipboard. Find the question, quiz item, or problem \
     that needs an answer.",
    answering_rules!(),
);

const BRIEF_ANSWER_INSTRUCTION: &str = concat!(
    "Answer the question below.",
    answering_rules!(),
);

fn custom_prompt(config: &AppConfig) -> Option<&str> {
    config
        .answer_prompt
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
}

fn screenshot_prompt(config: &AppConfig) -> &str {
    custom_prompt(config).unwrap_or(DEFAULT_ANSWER_PROMPT)
}

fn text_answer_instruction(config: &AppConfig) -> &str {
    custom_prompt(config).unwrap_or(BRIEF_ANSWER_INSTRUCTION)
}

fn clipboard_answer_instruction(config: &AppConfig) -> &str {
    custom_prompt(config).unwrap_or(CLIPBOARD_ANSWER_PROMPT)
}

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
    let options = chat_options_for_model(&config.model(), 8);
    client
        .exec_chat(model_for_request(config)?, chat_req, Some(&options))
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
        ContentPart::from_text(screenshot_prompt(config)),
        ContentPart::from_binary_base64("image/jpeg", jpeg_base64, Some("screenshot.jpg".into())),
    ]));
    exec_chat(&client, config, chat_req, 160).await
}

pub async fn answer_question(config: &AppConfig, question: &str) -> Result<String, AiError> {
    let question = question.trim();
    if question.is_empty() {
        return Err(AiError::Message("Question is empty.".into()));
    }
    let user_message = format!(
        "{}\n\nQuestion:\n{question}",
        text_answer_instruction(config)
    );
    exec_text_message(config, &user_message, 320).await
}

pub async fn answer_from_clipboard_text(
    config: &AppConfig,
    clipboard_text: &str,
) -> Result<String, AiError> {
    let text = clipboard_text.trim();
    if text.is_empty() {
        return Err(AiError::Message("Clipboard is empty or has no text.".into()));
    }
    let user_message = format!("{}\n\n{text}", clipboard_answer_instruction(config));
    exec_text_message(config, &user_message, 320).await
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
    let model = model_for_request(config)?;
    let options = chat_options_for_model(&model.model_name, max_tokens);
    let response = client
        .exec_chat(model, chat_req, Some(&options))
        .await
        .map_err(ai_error)?;
    response
        .first_text()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AiError::Message("Model returned an empty response.".into()))
}

fn model_for_request(config: &AppConfig) -> Result<ModelIden, AiError> {
    let name = config.model();
    let kind = match config.provider {
        Provider::Openai => AdapterKind::from_model(&name).map_err(|e| {
            AiError::Message(format!("Unsupported OpenAI model '{name}': {e}"))
        })?,
        Provider::Anthropic => AdapterKind::Anthropic,
        Provider::Google => AdapterKind::Gemini,
        // OpenRouter exposes an OpenAI-compatible chat API.
        Provider::Openrouter => AdapterKind::OpenAI,
    };
    Ok(ModelIden::new(kind, name))
}

fn chat_options_for_model(model_name: &str, max_tokens: u32) -> ChatOptions {
    let mut options = ChatOptions::default().with_max_tokens(max_tokens);
    // Reasoning-only OpenAI models reject non-default temperature.
    if !model_name.starts_with("o1")
        && !model_name.starts_with("o3")
        && !model_name.starts_with("o4")
    {
        options = options.with_temperature(0.0);
    }
    options
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
