// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: models
//
// Model configuration, provider abstraction, failover chain.
// Mirrors OpenClaw: src/agents/model-fallback.ts
//                   src/agents/auth-profiles.ts
//
// Session 1: types.
// Session 17: implement failover logic.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Which AI provider hosts this model
#[derive(Clone, Debug, PartialEq)]
pub enum ModelProvider {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    Custom,
}

impl ModelProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai"    => ModelProvider::OpenAI,
            "anthropic" => ModelProvider::Anthropic,
            "gemini"    => ModelProvider::Gemini,
            "ollama"    => ModelProvider::Ollama,
            _           => ModelProvider::Custom,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ModelProvider::OpenAI    => "openai",
            ModelProvider::Anthropic => "anthropic",
            ModelProvider::Gemini    => "gemini",
            ModelProvider::Ollama    => "ollama",
            ModelProvider::Custom    => "custom",
        }
    }

    /// Default base URL for this provider
    pub fn default_base_url(&self) -> &str {
        match self {
            ModelProvider::OpenAI    => "api.openai.com",
            ModelProvider::Anthropic => "api.anthropic.com",
            ModelProvider::Gemini    => "generativelanguage.googleapis.com",
            ModelProvider::Ollama    => "127.0.0.1",
            ModelProvider::Custom    => "",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            ModelProvider::Ollama => 11434,
            _                     => 443,
        }
    }
}

/// A single model configuration entry
#[derive(Clone, Debug)]
pub struct ModelConfig {
    pub id:           String,   // unique name, e.g. "primary"
    pub provider:     ModelProvider,
    pub model:        String,   // e.g. "gpt-4o", "claude-3-5-sonnet-latest"
    pub api_key:      String,   // encrypted at rest, decrypted on use
    pub base_url:     String,
    pub port:         u16,
    pub priority:     u32,      // lower = preferred; 0 = highest
    pub max_tokens:   u32,
    pub context_size: u32,      // max context window tokens
    pub enabled:      bool,
    // Rate limit tracking
    pub rate_limit_reset_at: u128,  // unix ms
    pub consecutive_errors:  u32,
}

impl ModelConfig {
    pub fn default_openai(api_key: &str) -> Self {
        ModelConfig {
            id: "openai-primary".to_string(),
            provider: ModelProvider::OpenAI,
            model: "gpt-4o".to_string(),
            api_key: api_key.to_string(),
            base_url: "api.openai.com".to_string(),
            port: 443,
            priority: 0,
            max_tokens: 4096,
            context_size: 128_000,
            enabled: true,
            rate_limit_reset_at: 0,
            consecutive_errors: 0,
        }
    }

    pub fn to_json_safe(&self) -> String {
        // Never expose api_key in JSON output
        format!(
            r#"{{"id":"{}","provider":"{}","model":"{}","priority":{},"enabled":{},"consecutive_errors":{}}}"#,
            self.id, self.provider.as_str(), self.model,
            self.priority, self.enabled, self.consecutive_errors
        )
    }

    /// True if this model is currently usable (not rate-limited, not erroring)
    pub fn is_available(&self, now_ms: u128) -> bool {
        self.enabled
            && self.rate_limit_reset_at < now_ms
            && self.consecutive_errors < 5
    }
}

/// Ordered list of models to try. Mirrors OpenClaw model-failover.
#[derive(Clone, Debug, Default)]
pub struct FailoverChain {
    pub models: Vec<ModelConfig>,
}

impl FailoverChain {
    pub fn new(models: Vec<ModelConfig>) -> Self {
        let mut chain = FailoverChain { models };
        chain.models.sort_by_key(|m| m.priority);
        chain
    }

    /// Returns the first available model, or None if all are unavailable
    pub fn pick(&self, now_ms: u128) -> Option<&ModelConfig> {
        self.models.iter().find(|m| m.is_available(now_ms))
    }

    /// Mark a model as failed (e.g. 429 or 500 from provider)
    pub fn mark_error(&mut self, model_id: &str, rate_limit_reset_ms: Option<u128>) {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == model_id) {
            m.consecutive_errors += 1;
            if let Some(reset) = rate_limit_reset_ms {
                m.rate_limit_reset_at = reset;
            }
        }
    }

    /// Mark a model as succeeded — reset error counter
    pub fn mark_success(&mut self, model_id: &str) {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == model_id) {
            m.consecutive_errors = 0;
            m.rate_limit_reset_at = 0;
        }
    }

    pub fn to_json_safe(&self) -> String {
        let items: Vec<String> = self.models.iter()
            .map(|m| m.to_json_safe())
            .collect();
        format!("[{}]", items.join(","))
    }

    /// Parse model list from JSON array of provider objects
    pub fn from_json(json: &str) -> Self {
        // Minimal parser: look for objects in array
        let mut models = Vec::new();
        let mut pos = 0;
        let bytes = json.as_bytes();
        let mut depth = 0i32;
        let mut obj_start = None;

        while pos < bytes.len() {
            match bytes[pos] {
                b'{' => {
                    depth += 1;
                    if depth == 1 { obj_start = Some(pos); }
                }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start) = obj_start {
                            let fragment = &json[start..=pos];
                            if let Some(m) = parse_model_config(fragment) {
                                models.push(m);
                            }
                        }
                        obj_start = None;
                    }
                }
                _ => {}
            }
            pos += 1;
        }

        FailoverChain::new(models)
    }
}

fn parse_model_config(json: &str) -> Option<ModelConfig> {
    fn str_field<'a>(json: &'a str, key: &str) -> Option<String> {
        let search = format!("\"{}\":\"", key);
        let start = json.find(&search)? + search.len();
        let end = json[start..].find('"')? + start;
        Some(json[start..end].to_string())
    }
    fn u32_field(json: &str, key: &str) -> Option<u32> {
        let search = format!("\"{}\":", key);
        let start = json.find(&search)? + search.len();
        let slice = json[start..].trim_start();
        let end = slice.find(|c: char| !c.is_ascii_digit()).unwrap_or(slice.len());
        slice[..end].parse().ok()
    }

    let provider_str = str_field(json, "provider").unwrap_or_else(|| "openai".to_string());
    let provider = ModelProvider::from_str(&provider_str);
    let default_url = provider.default_base_url().to_string();
    let default_port = provider.default_port();

    Some(ModelConfig {
        id:           str_field(json, "id").unwrap_or_else(|| "model-0".to_string()),
        model:        str_field(json, "model").unwrap_or_else(|| "gpt-4o".to_string()),
        api_key:      str_field(json, "api_key").unwrap_or_default(),
        base_url:     str_field(json, "base_url").unwrap_or(default_url),
        port:         u32_field(json, "port").unwrap_or(default_port as u32) as u16,
        priority:     u32_field(json, "priority").unwrap_or(0),
        max_tokens:   u32_field(json, "max_tokens").unwrap_or(4096),
        context_size: u32_field(json, "context_size").unwrap_or(128_000),
        provider,
        enabled: true,
        rate_limit_reset_at: 0,
        consecutive_errors: 0,
    })
}
