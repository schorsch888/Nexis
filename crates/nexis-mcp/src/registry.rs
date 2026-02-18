use std::str::FromStr;

use nexis_runtime::{AIProvider, ProviderError};

use crate::{AnthropicProvider, GeminiProvider, OpenAIProvider};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Gemini,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderKind::OpenAI => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Gemini => "gemini",
        }
    }

    pub fn required_api_key_env(self) -> &'static str {
        match self {
            ProviderKind::OpenAI => "OPENAI_API_KEY",
            ProviderKind::Anthropic => "ANTHROPIC_API_KEY",
            ProviderKind::Gemini => "GEMINI_API_KEY",
        }
    }
}

impl FromStr for ProviderKind {
    type Err = ProviderError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Ok(ProviderKind::OpenAI),
            "anthropic" | "claude" => Ok(ProviderKind::Anthropic),
            "gemini" => Ok(ProviderKind::Gemini),
            _ => Err(ProviderError::Message(format!(
                "unsupported provider '{value}', expected one of: openai, anthropic, gemini"
            ))),
        }
    }
}

pub fn create_provider(kind: ProviderKind, api_key: impl Into<String>) -> Box<dyn AIProvider> {
    let key = api_key.into();

    match kind {
        ProviderKind::OpenAI => Box::new(OpenAIProvider::new(key)),
        ProviderKind::Anthropic => Box::new(AnthropicProvider::new(key)),
        ProviderKind::Gemini => Box::new(GeminiProvider::new(key)),
    }
}

pub fn create_provider_from_env() -> Result<Box<dyn AIProvider>, ProviderError> {
    let provider_name = std::env::var("NEXIS_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    let kind = ProviderKind::from_str(&provider_name)?;

    let api_key_env = kind.required_api_key_env();
    let api_key = std::env::var(api_key_env).map_err(|_| {
        ProviderError::Message(format!(
            "missing required environment variable '{api_key_env}' for provider '{}'",
            kind.as_str()
        ))
    })?;

    if api_key.trim().is_empty() {
        return Err(ProviderError::Message(format!(
            "environment variable '{api_key_env}' is empty for provider '{}'",
            kind.as_str()
        )));
    }

    Ok(create_provider(kind, api_key))
}

#[cfg(test)]
mod tests {
    use super::{create_provider, create_provider_from_env, ProviderKind};
    use std::str::FromStr;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn parses_provider_kind_aliases() {
        assert_eq!(
            ProviderKind::from_str("openai").unwrap(),
            ProviderKind::OpenAI
        );
        assert_eq!(
            ProviderKind::from_str("anthropic").unwrap(),
            ProviderKind::Anthropic
        );
        assert_eq!(
            ProviderKind::from_str("claude").unwrap(),
            ProviderKind::Anthropic
        );
        assert_eq!(
            ProviderKind::from_str("gemini").unwrap(),
            ProviderKind::Gemini
        );
    }

    #[test]
    fn create_provider_returns_expected_runtime_name() {
        assert_eq!(create_provider(ProviderKind::OpenAI, "k1").name(), "openai");
        assert_eq!(
            create_provider(ProviderKind::Anthropic, "k2").name(),
            "anthropic"
        );
        assert_eq!(create_provider(ProviderKind::Gemini, "k3").name(), "gemini");
    }

    #[test]
    fn create_provider_from_env_errors_on_missing_key() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var("NEXIS_PROVIDER", "anthropic");
        std::env::remove_var("ANTHROPIC_API_KEY");

        let err = create_provider_from_env().unwrap_err();
        assert!(err
            .to_string()
            .contains("missing required environment variable 'ANTHROPIC_API_KEY'"));

        std::env::remove_var("NEXIS_PROVIDER");
    }

    #[test]
    fn create_provider_from_env_uses_selected_provider() {
        let _guard = env_lock().lock().unwrap();
        std::env::set_var("NEXIS_PROVIDER", "gemini");
        std::env::set_var("GEMINI_API_KEY", "test-key");

        let provider = create_provider_from_env().unwrap();
        assert_eq!(provider.name(), "gemini");

        std::env::remove_var("NEXIS_PROVIDER");
        std::env::remove_var("GEMINI_API_KEY");
    }
}
