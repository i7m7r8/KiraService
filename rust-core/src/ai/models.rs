// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: models
//
// Full model provider catalog + failover chain.
// Mirrors OpenClaw: src/agents/model-catalog.ts
//                   src/agents/models-config.ts
//                   src/agents/models-config.providers.ts
//                   src/agents/auth-profiles.resolve-auth-profile-order.ts
//
// S4: Provider enum + ProviderConfig + model catalog (30 providers)
// S5: Failover chain with cooldown + retry (see failover.rs)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;

// ── S4: Provider enum (all 30+ providers from models-config.providers.ts) ─────

#[derive(Clone, Debug, PartialEq)]
pub enum ModelProvider {
    // Tier-1 cloud (need API key)
    Anthropic,
    OpenAI,
    Google,        // Gemini API
    OpenRouter,
    Groq,
    Together,
    Mistral,
    Cohere,
    DeepSeek,
    xAI,           // Grok
    Perplexity,
    Fireworks,
    Cerebras,
    NovitaAI,
    // Local / self-hosted
    Ollama,
    LMStudio,
    // Cloud-hosted
    AzureOpenAI,
    VertexAI,      // Google Cloud
    Bedrock,       // AWS
    // Specialised
    Moonshot,      // Kimi
    MiniMax,
    Qwen,          // Alibaba
    Baichuan,
    // Generic
    OpenAICompat,  // any OpenAI-compatible endpoint
    Custom,
}

impl ModelProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().replace('-', "").as_str() {
            "anthropic"             => ModelProvider::Anthropic,
            "openai"                => ModelProvider::OpenAI,
            "google" | "gemini"     => ModelProvider::Google,
            "openrouter"            => ModelProvider::OpenRouter,
            "groq"                  => ModelProvider::Groq,
            "together" | "togetherai" => ModelProvider::Together,
            "mistral" | "mistralai" => ModelProvider::Mistral,
            "cohere"                => ModelProvider::Cohere,
            "deepseek"              => ModelProvider::DeepSeek,
            "xai" | "grok"          => ModelProvider::xAI,
            "perplexity"            => ModelProvider::Perplexity,
            "fireworks" | "fireworksai" => ModelProvider::Fireworks,
            "cerebras"              => ModelProvider::Cerebras,
            "novitaai" | "novita"   => ModelProvider::NovitaAI,
            "ollama"                => ModelProvider::Ollama,
            "lmstudio"              => ModelProvider::LMStudio,
            "azure" | "azureopenai" => ModelProvider::AzureOpenAI,
            "vertexai" | "vertex"   => ModelProvider::VertexAI,
            "bedrock" | "awsbedrock"=> ModelProvider::Bedrock,
            "moonshot" | "kimi"     => ModelProvider::Moonshot,
            "minimax"               => ModelProvider::MiniMax,
            "qwen" | "alibaba"      => ModelProvider::Qwen,
            "baichuan"              => ModelProvider::Baichuan,
            "openaiccompat"         => ModelProvider::OpenAICompat,
            _                       => ModelProvider::Custom,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ModelProvider::Anthropic    => "anthropic",
            ModelProvider::OpenAI       => "openai",
            ModelProvider::Google       => "google",
            ModelProvider::OpenRouter   => "openrouter",
            ModelProvider::Groq         => "groq",
            ModelProvider::Together     => "together",
            ModelProvider::Mistral      => "mistral",
            ModelProvider::Cohere       => "cohere",
            ModelProvider::DeepSeek     => "deepseek",
            ModelProvider::xAI          => "xai",
            ModelProvider::Perplexity   => "perplexity",
            ModelProvider::Fireworks    => "fireworks",
            ModelProvider::Cerebras     => "cerebras",
            ModelProvider::NovitaAI     => "novitaai",
            ModelProvider::Ollama       => "ollama",
            ModelProvider::LMStudio     => "lmstudio",
            ModelProvider::AzureOpenAI  => "azure",
            ModelProvider::VertexAI     => "vertexai",
            ModelProvider::Bedrock      => "bedrock",
            ModelProvider::Moonshot     => "moonshot",
            ModelProvider::MiniMax      => "minimax",
            ModelProvider::Qwen         => "qwen",
            ModelProvider::Baichuan     => "baichuan",
            ModelProvider::OpenAICompat => "openaiccompat",
            ModelProvider::Custom       => "custom",
        }
    }

    /// Whether this provider uses OpenAI-compatible REST format.
    /// These providers need no special handling  -  just change base_url + api_key.
    pub fn is_openai_compat(&self) -> bool {
        matches!(self,
            ModelProvider::OpenAI    | ModelProvider::Groq      |
            ModelProvider::Together  | ModelProvider::Mistral   |
            ModelProvider::DeepSeek  | ModelProvider::Perplexity|
            ModelProvider::Fireworks | ModelProvider::Cerebras  |
            ModelProvider::NovitaAI  | ModelProvider::OpenRouter|
            ModelProvider::xAI       | ModelProvider::LMStudio  |
            ModelProvider::Ollama    | ModelProvider::OpenAICompat |
            ModelProvider::Custom    | ModelProvider::MiniMax   |
            ModelProvider::Moonshot  | ModelProvider::Qwen      |
            ModelProvider::Baichuan
        )
    }

    /// Whether this provider uses Anthropic's native API format.
    pub fn is_anthropic_native(&self) -> bool {
        matches!(self, ModelProvider::Anthropic)
    }

    /// Whether this provider uses Google's Gemini API format.
    pub fn is_google_native(&self) -> bool {
        matches!(self, ModelProvider::Google | ModelProvider::VertexAI)
    }
}

// ── S4: ProviderConfig  -  one configured provider entry ────────────────────────
// Mirrors OpenClaw: src/agents/auth-profiles.ts (AuthProfile)

#[derive(Clone, Debug)]
pub struct ProviderConfig {
    /// Unique ID for this provider entry (e.g. "groq-primary", "ollama-local")
    pub id:              String,
    pub provider:        ModelProvider,
    /// Default model to use with this provider
    pub model_id:        String,
    /// API key (stored encrypted in KiraState.config, decrypted on use)
    pub api_key:         String,
    /// Base URL without trailing slash (e.g. "https://api.groq.com/openai/v1")
    pub base_url:        String,
    /// Extra headers to send (e.g. for OpenRouter's HTTP-Referer)
    pub extra_headers:   HashMap<String, String>,
    /// Request timeout in ms (default 120_000)
    pub timeout_ms:      u64,
    /// Whether this provider is currently enabled
    pub enabled:         bool,
    /// Priority  -  lower = tried first (mirrors OpenClaw profile order)
    pub priority:        u32,
    // ── Failover state (S5) ──
    /// Unix ms when cooldown expires (0 = available now)
    pub cooldown_until:  u128,
    /// Consecutive error count since last success
    pub error_count:     u32,
}

impl ProviderConfig {
    pub fn new(id: &str, provider: ModelProvider, model: &str, api_key: &str, base_url: &str) -> Self {
        ProviderConfig {
            id:             id.to_string(),
            provider,
            model_id:       model.to_string(),
            api_key:        api_key.to_string(),
            base_url:       base_url.trim_end_matches('/').to_string(),
            extra_headers:  HashMap::new(),
            timeout_ms:     120_000,
            enabled:        true,
            priority:       0,
            cooldown_until: 0,
            error_count:    0,
        }
    }

    pub fn is_available(&self, now_ms: u128) -> bool {
        self.enabled && self.cooldown_until <= now_ms && self.error_count < 5
    }

    /// Return chat completions URL for this provider.
    pub fn chat_url(&self) -> String {
        match self.provider {
            ModelProvider::Google | ModelProvider::VertexAI => {
                // Gemini uses a different path
                format!("{}/models/{}:streamGenerateContent",
                    self.base_url, self.model_id)
            }
            ModelProvider::Anthropic => {
                format!("{}/v1/messages", self.base_url)
            }
            _ => {
                format!("{}/chat/completions", self.base_url)
            }
        }
    }

    pub fn to_json_safe(&self) -> String {
        format!(
            r#"{{"id":"{}","provider":"{}","model":"{}","base_url":"{}","enabled":{},"priority":{},"error_count":{},"available":{}}}"#,
            esc(&self.id), self.provider.as_str(), esc(&self.model_id),
            esc(&self.base_url), self.enabled, self.priority,
            self.error_count, self.error_count < 5 && self.enabled
        )
    }
}

// ── S4: Built-in provider catalog (mirrors models-config.providers.ts) ────────
// These are the default endpoints. Users can override api_key and add custom entries.

pub struct ProviderCatalog;

impl ProviderCatalog {
    /// Return all built-in provider definitions (without api keys).
    /// Caller fills in api_key from config before use.
    pub fn all_entries() -> Vec<ProviderConfig> {
        vec![
            // ── Anthropic ──
            ProviderConfig::new("anthropic",  ModelProvider::Anthropic,
                "claude-sonnet-4-5",
                "", "https://api.anthropic.com"),

            // ── OpenAI ──
            ProviderConfig::new("openai",     ModelProvider::OpenAI,
                "gpt-4o",
                "", "https://api.openai.com/v1"),

            // ── Google Gemini ──
            ProviderConfig::new("google",     ModelProvider::Google,
                "gemini-2.0-flash",
                "", "https://generativelanguage.googleapis.com/v1beta"),

            // ── Groq (default for Kira  -  fast, free tier) ──
            ProviderConfig::new("groq",       ModelProvider::Groq,
                "llama-3.3-70b-versatile",
                "", "https://api.groq.com/openai/v1"),

            // ── OpenRouter ──
            ProviderConfig::new("openrouter", ModelProvider::OpenRouter,
                "anthropic/claude-sonnet-4-5",
                "", "https://openrouter.ai/api/v1"),

            // ── Together AI ──
            ProviderConfig::new("together",   ModelProvider::Together,
                "meta-llama/Llama-3-70b-chat-hf",
                "", "https://api.together.xyz/v1"),

            // ── Mistral ──
            ProviderConfig::new("mistral",    ModelProvider::Mistral,
                "mistral-large-latest",
                "", "https://api.mistral.ai/v1"),

            // ── DeepSeek ──
            ProviderConfig::new("deepseek",   ModelProvider::DeepSeek,
                "deepseek-chat",
                "", "https://api.deepseek.com/v1"),

            // ── xAI Grok ──
            ProviderConfig::new("xai",        ModelProvider::xAI,
                "grok-3",
                "", "https://api.x.ai/v1"),

            // ── Perplexity ──
            ProviderConfig::new("perplexity", ModelProvider::Perplexity,
                "llama-3.1-sonar-large-128k-online",
                "", "https://api.perplexity.ai"),

            // ── Fireworks AI ──
            ProviderConfig::new("fireworks",  ModelProvider::Fireworks,
                "accounts/fireworks/models/llama-v3p1-70b-instruct",
                "", "https://api.fireworks.ai/inference/v1"),

            // ── Cerebras ──
            ProviderConfig::new("cerebras",   ModelProvider::Cerebras,
                "llama3.1-70b",
                "", "https://api.cerebras.ai/v1"),

            // ── Novita AI ──
            ProviderConfig::new("novitaai",   ModelProvider::NovitaAI,
                "meta-llama/llama-3.1-70b-instruct",
                "", "https://api.novita.ai/v3/openai"),

            // ── Cohere ──
            ProviderConfig::new("cohere",     ModelProvider::Cohere,
                "command-r-plus",
                "", "https://api.cohere.com/v2"),

            // ── Local: Ollama ──
            ProviderConfig::new("ollama",     ModelProvider::Ollama,
                "llama3.2",
                "", "http://localhost:11434/v1"),

            // ── Local: LM Studio ──
            ProviderConfig::new("lmstudio",   ModelProvider::LMStudio,
                "local-model",
                "", "http://localhost:1234/v1"),

            // ── Moonshot (Kimi) ──
            ProviderConfig::new("moonshot",   ModelProvider::Moonshot,
                "moonshot-v1-128k",
                "", "https://api.moonshot.cn/v1"),

            // ── Qwen (Alibaba) ──
            ProviderConfig::new("qwen",       ModelProvider::Qwen,
                "qwen-max",
                "", "https://dashscope.aliyuncs.com/compatible-mode/v1"),
        ]
    }

    /// Build the catalog from entries, keyed by provider id.
    pub fn build() -> HashMap<String, ProviderConfig> {
        Self::all_entries().into_iter().map(|p| (p.id.clone(), p)).collect()
    }

    /// Find a provider by id string.
    pub fn find(id: &str) -> Option<ProviderConfig> {
        Self::all_entries().into_iter().find(|p| p.id == id)
    }

    /// Return JSON array of all providers (safe  -  no keys).
    pub fn to_json() -> String {
        let items: Vec<String> = Self::all_entries().iter()
            .map(|p| p.to_json_safe())
            .collect();
        format!("[{}]", items.join(","))
    }
}

// ── S4: ModelConfig  -  one configured model entry (kept for FailoverChain) ─────

#[derive(Clone, Debug)]
pub struct ModelConfig {
    pub id:           String,
    pub provider:     ModelProvider,
    pub model:        String,
    pub api_key:      String,
    pub base_url:     String,
    pub port:         u16,
    pub priority:     u32,
    pub max_tokens:   u32,
    pub context_size: u32,
    pub enabled:      bool,
    pub rate_limit_reset_at: u128,
    pub consecutive_errors:  u32,
}

impl ModelConfig {
    pub fn from_provider(cfg: &ProviderConfig) -> Self {
        let port: u16 = if cfg.base_url.starts_with("http://") { 80 } else { 443 };
        ModelConfig {
            id:           cfg.id.clone(),
            provider:     cfg.provider.clone(),
            model:        cfg.model_id.clone(),
            api_key:      cfg.api_key.clone(),
            base_url:     cfg.base_url.clone(),
            port,
            priority:     cfg.priority,
            max_tokens:   8192,
            context_size: 128_000,
            enabled:      cfg.enabled,
            rate_limit_reset_at: cfg.cooldown_until,
            consecutive_errors:  cfg.error_count,
        }
    }

    pub fn is_available(&self, now_ms: u128) -> bool {
        self.enabled
            && self.rate_limit_reset_at < now_ms
            && self.consecutive_errors < 5
    }

    pub fn to_json_safe(&self) -> String {
        format!(
            r#"{{"id":"{}","provider":"{}","model":"{}","priority":{},"enabled":{},"errors":{}}}"#,
            esc(&self.id), self.provider.as_str(), esc(&self.model),
            self.priority, self.enabled, self.consecutive_errors
        )
    }
}

// ── FailoverChain (S4 foundation, S5 full logic) ─────────────────────────────

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

    pub fn from_providers(providers: &[ProviderConfig]) -> Self {
        let models = providers.iter().map(ModelConfig::from_provider).collect();
        Self::new(models)
    }

    pub fn pick(&self, now_ms: u128) -> Option<&ModelConfig> {
        self.models.iter().find(|m| m.is_available(now_ms))
    }

    pub fn mark_error(&mut self, model_id: &str, cooldown_ms: Option<u128>) {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == model_id) {
            m.consecutive_errors += 1;
            if let Some(reset) = cooldown_ms { m.rate_limit_reset_at = reset; }
        }
    }

    pub fn mark_success(&mut self, model_id: &str) {
        if let Some(m) = self.models.iter_mut().find(|m| m.id == model_id) {
            m.consecutive_errors = 0;
            m.rate_limit_reset_at = 0;
        }
    }

    pub fn to_json_safe(&self) -> String {
        let items: Vec<String> = self.models.iter().map(|m| m.to_json_safe()).collect();
        format!("[{}]", items.join(","))
    }

    pub fn from_json(json: &str) -> Self {
        let mut models = Vec::new();
        let mut depth = 0i32;
        let mut obj_start = None;
        let bytes = json.as_bytes();
        for (pos, &b) in bytes.iter().enumerate() {
            match b {
                b'{' => { depth += 1; if depth == 1 { obj_start = Some(pos); } }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start) = obj_start {
                            if let Some(m) = parse_model_config(&json[start..=pos]) {
                                models.push(m);
                            }
                        }
                        obj_start = None;
                    }
                }
                _ => {}
            }
        }
        FailoverChain::new(models)
    }
}

fn parse_model_config(json: &str) -> Option<ModelConfig> {
    let s = |key: &str| -> Option<String> {
        let search = format!("\"{}\":\"", key);
        let start = json.find(&search)? + search.len();
        let end = json[start..].find('"')? + start;
        Some(json[start..end].to_string())
    };
    let n = |key: &str| -> Option<u32> {
        let search = format!("\"{}\":", key);
        let start = json.find(&search)? + search.len();
        let rest  = json[start..].trim_start();
        let end   = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        rest[..end].parse().ok()
    };
    let provider_str = s("provider").unwrap_or_else(|| "openai".to_string());
    let provider = ModelProvider::from_str(&provider_str);
    let default_url = match &provider {
        ModelProvider::Groq      => "https://api.groq.com/openai/v1".to_string(),
        ModelProvider::Anthropic => "https://api.anthropic.com".to_string(),
        ModelProvider::Google    => "https://generativelanguage.googleapis.com/v1beta".to_string(),
        ModelProvider::Ollama    => "http://localhost:11434/v1".to_string(),
        _                        => "https://api.openai.com/v1".to_string(),
    };
    Some(ModelConfig {
        id:           s("id").unwrap_or_else(|| "model-0".to_string()),
        model:        s("model").unwrap_or_else(|| "gpt-4o".to_string()),
        api_key:      s("api_key").unwrap_or_default(),
        base_url:     s("base_url").unwrap_or(default_url),
        port:         n("port").unwrap_or(443) as u16,
        priority:     n("priority").unwrap_or(0),
        max_tokens:   n("max_tokens").unwrap_or(8192),
        context_size: n("context_size").unwrap_or(128_000),
        provider,
        enabled:             true,
        rate_limit_reset_at: 0,
        consecutive_errors:  0,
    })
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
