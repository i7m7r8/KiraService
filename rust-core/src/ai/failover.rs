// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: failover
//
// Provider failover chain with cooldown + retry logic.
// Mirrors OpenClaw: src/agents/auth-profiles/usage.ts
//                   src/agents/auth-profiles/order.ts
//
// S5: FailoverState — pick(), mark_failure(), mark_success(), next_cooldown_ms()
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;
use super::models::ProviderConfig;

// ── Failure kinds (mirrors OpenClaw AuthProfileFailureReason) ─────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum FailureKind {
    /// 401 / invalid API key — permanent cooldown for this session
    AuthPermanent,
    /// 402 / billing limit — long cooldown (hours)
    Billing,
    /// 429 / rate limited — medium cooldown (minutes, exponential)
    RateLimit,
    /// 500-503 / server overloaded — short cooldown (seconds)
    Overloaded,
    /// Network timeout — short cooldown
    Timeout,
    /// Model not found / deprecated — permanent disable
    ModelNotFound,
    /// Any other error
    Unknown,
}

impl FailureKind {
    /// Parse from HTTP status code.
    pub fn from_http_status(status: u16) -> Self {
        match status {
            401 | 403 => FailureKind::AuthPermanent,
            402        => FailureKind::Billing,
            429        => FailureKind::RateLimit,
            404        => FailureKind::ModelNotFound,
            500..=503  => FailureKind::Overloaded,
            _          => FailureKind::Unknown,
        }
    }

    /// Parse from error message string.
    pub fn from_message(msg: &str) -> Self {
        let m = msg.to_lowercase();
        if m.contains("rate limit") || m.contains("rate_limit") || m.contains("429") {
            FailureKind::RateLimit
        } else if m.contains("billing") || m.contains("quota") || m.contains("payment") {
            FailureKind::Billing
        } else if m.contains("unauthorized") || m.contains("invalid api key") || m.contains("401") {
            FailureKind::AuthPermanent
        } else if m.contains("overloaded") || m.contains("503") || m.contains("capacity") {
            FailureKind::Overloaded
        } else if m.contains("timeout") || m.contains("timed out") {
            FailureKind::Timeout
        } else if m.contains("not found") || m.contains("model") || m.contains("404") {
            FailureKind::ModelNotFound
        } else {
            FailureKind::Unknown
        }
    }

    /// Whether this failure kind permanently disables the profile.
    pub fn is_permanent(&self) -> bool {
        matches!(self, FailureKind::AuthPermanent | FailureKind::ModelNotFound)
    }
}

// ── Per-profile usage stats ────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
struct ProfileStats {
    /// Error count since last success (used for exponential backoff)
    error_count:     u32,
    /// Unix ms when cooldown expires (0 = available)
    cooldown_until:  u128,
    /// Whether this profile is permanently disabled
    disabled:        bool,
    /// Last successfully used timestamp (unix ms)
    last_used_ms:    u128,
    /// Reason for last failure
    last_failure:    Option<String>,
}

impl ProfileStats {
    fn is_available(&self, now_ms: u128) -> bool {
        !self.disabled && self.cooldown_until <= now_ms
    }

    fn soonest_available(&self, now_ms: u128) -> u128 {
        if self.disabled { u128::MAX }
        else { self.cooldown_until.max(now_ms) }
    }
}

// ── Cooldown duration calculation ─────────────────────────────────────────────
// Mirrors OpenClaw: calculateAuthProfileCooldownMs()
//   error_count=1 → 60s, 2 → 300s, 3 → 1500s, 4+ → 3600s (1h max)
//   Formula: min(1h, 60s × 5^min(error_count-1, 3))

fn cooldown_ms_for_error(error_count: u32, kind: &FailureKind) -> u128 {
    match kind {
        FailureKind::Billing => {
            // 5h base, doubles each billing failure, max 24h
            let base: u128 = 5 * 60 * 60 * 1000;
            let exponent = (error_count.saturating_sub(1)).min(10) as u32;
            (base * 2u128.pow(exponent)).min(24 * 60 * 60 * 1000)
        }
        FailureKind::RateLimit | FailureKind::Overloaded => {
            // Exponential: 60s, 300s, 1500s, 3600s cap
            let n = error_count.max(1) as u32;
            let exponent = n.saturating_sub(1).min(3);
            let raw = 60_000u128 * 5u128.pow(exponent);
            raw.min(60 * 60 * 1000) // 1 hour max
        }
        FailureKind::Timeout | FailureKind::Unknown => {
            // Short: 10s, 20s, 40s, up to 5 min
            let n = error_count.max(1) as u32;
            let exponent = n.saturating_sub(1).min(5);
            let raw = 10_000u128 * 2u128.pow(exponent);
            raw.min(5 * 60 * 1000)
        }
        FailureKind::AuthPermanent | FailureKind::ModelNotFound => {
            u128::MAX // permanent
        }
    }
}

// ── FailoverState — the main struct ──────────────────────────────────────────

/// Manages provider selection + failure tracking across multiple profiles.
/// Mirrors OpenClaw: resolveAuthProfileOrder() + markAuthProfileCooldown()
pub struct FailoverState {
    /// Ordered provider configs (priority = ascending order)
    pub profiles:    Vec<ProviderConfig>,
    /// Per-profile usage stats, keyed by profile id
    stats:           HashMap<String, ProfileStats>,
    /// The last profile that succeeded (may be preferred on next pick)
    pub last_good:   Option<String>,
}

impl FailoverState {
    pub fn new(profiles: Vec<ProviderConfig>) -> Self {
        let mut ordered = profiles;
        ordered.sort_by_key(|p| p.priority);
        FailoverState {
            profiles: ordered,
            stats:    HashMap::new(),
            last_good: None,
        }
    }

    // ── Pick ─────────────────────────────────────────────────────────────────

    /// Select the next available profile.
    /// Priority:
    ///   1. last_good (if still available + no errors)
    ///   2. First available by priority order (no cooldown)
    ///   3. First profile whose cooldown expires soonest (all in cooldown)
    /// Mirrors: resolveAuthProfileOrder() available-first, then soonest-cooldown
    pub fn next_profile(&self, now_ms: u128) -> Option<&ProviderConfig> {
        // Expire stale cooldowns first
        let available: Vec<&ProviderConfig> = self.profiles.iter()
            .filter(|p| p.enabled)
            .filter(|p| {
                let stats = self.stats.get(&p.id);
                stats.map(|s| s.is_available(now_ms)).unwrap_or(true)
            })
            .collect();

        if available.is_empty() {
            // All in cooldown — return the one whose cooldown expires soonest
            return self.profiles.iter()
                .filter(|p| p.enabled)
                .filter(|p| !self.stats.get(&p.id).map(|s| s.disabled).unwrap_or(false))
                .min_by_key(|p| {
                    self.stats.get(&p.id)
                        .map(|s| s.soonest_available(now_ms))
                        .unwrap_or(now_ms)
                });
        }

        // Prefer last_good if it's in the available set and has no errors
        if let Some(ref last) = self.last_good {
            if let Some(p) = available.iter().find(|p| &p.id == last) {
                let errors = self.stats.get(&p.id).map(|s| s.error_count).unwrap_or(0);
                if errors == 0 {
                    return Some(p);
                }
            }
        }

        // Return first available by priority
        available.into_iter().next()
    }

    // ── Mark failure ──────────────────────────────────────────────────────────

    /// Record a failure for a profile and apply appropriate cooldown.
    /// Mirrors: markAuthProfileCooldown()
    pub fn mark_failure(&mut self, profile_id: &str, kind: FailureKind, now_ms: u128) {
        let stats = self.stats.entry(profile_id.to_string()).or_default();
        stats.error_count += 1;
        stats.last_failure = Some(format!("{:?}", kind));

        if kind.is_permanent() {
            stats.disabled = true;
            stats.cooldown_until = u128::MAX;
        } else {
            let cooldown = cooldown_ms_for_error(stats.error_count, &kind);
            stats.cooldown_until = now_ms + cooldown;
        }

        // Also update the ProviderConfig error_count for FailoverChain compatibility
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == profile_id) {
            p.error_count  = stats.error_count;
            p.cooldown_until = stats.cooldown_until;
            if kind.is_permanent() { p.enabled = false; }
        }
    }

    // ── Mark success ──────────────────────────────────────────────────────────

    /// Record a successful call — reset error count and cooldown.
    pub fn mark_success(&mut self, profile_id: &str, now_ms: u128) {
        let stats = self.stats.entry(profile_id.to_string()).or_default();
        stats.error_count    = 0;
        stats.cooldown_until = 0;
        stats.last_used_ms   = now_ms;
        self.last_good       = Some(profile_id.to_string());

        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == profile_id) {
            p.error_count    = 0;
            p.cooldown_until = 0;
        }
    }

    // ── Expire stale cooldowns ────────────────────────────────────────────────
    /// Mirrors: clearExpiredCooldowns() — resets profiles whose cooldown window passed.
    /// Call this at the start of each pick cycle.
    pub fn clear_expired_cooldowns(&mut self, now_ms: u128) {
        for stats in self.stats.values_mut() {
            if !stats.disabled && stats.cooldown_until > 0 && stats.cooldown_until <= now_ms {
                // Cooldown window passed — reset error count for a fresh start
                stats.error_count    = 0;
                stats.cooldown_until = 0;
            }
        }
        for p in self.profiles.iter_mut() {
            if p.cooldown_until > 0 && p.cooldown_until <= now_ms {
                p.error_count    = 0;
                p.cooldown_until = 0;
            }
        }
    }

    /// Return ms until the soonest cooldown expires (0 if any profile is available).
    pub fn next_cooldown_ms(&self, now_ms: u128) -> u128 {
        let available = self.profiles.iter().any(|p| {
            p.enabled && self.stats.get(&p.id).map(|s| s.is_available(now_ms)).unwrap_or(true)
        });
        if available { return 0; }
        self.stats.values()
            .filter(|s| !s.disabled && s.cooldown_until > now_ms)
            .map(|s| s.cooldown_until - now_ms)
            .min()
            .unwrap_or(0)
    }

    /// JSON status for /ai/failover endpoint.
    pub fn to_json(&self, now_ms: u128) -> String {
        let profiles: Vec<String> = self.profiles.iter().map(|p| {
            let stats = self.stats.get(&p.id);
            let available = stats.map(|s| s.is_available(now_ms)).unwrap_or(true);
            let err_count = stats.map(|s| s.error_count).unwrap_or(0);
            let cooldown  = stats.map(|s| s.cooldown_until).unwrap_or(0);
            let is_last   = self.last_good.as_deref() == Some(&p.id);
            format!(
                r#"{{"id":"{}","provider":"{}","enabled":{},"available":{},"error_count":{},"cooldown_until":{},"last_good":{}}}"#,
                esc(&p.id), p.provider.as_str(), p.enabled,
                available, err_count, cooldown, is_last
            )
        }).collect();
        format!("[{}]", profiles.join(","))
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
