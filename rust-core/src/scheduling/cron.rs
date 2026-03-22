// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: scheduling :: cron
//
// Cron job types and expression parser.
// Session 1: types + next_run_ms().
// Session 9: full scheduler service.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Idle,
    Running,
    Disabled,
}

impl JobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            JobStatus::Idle     => "idle",
            JobStatus::Running  => "running",
            JobStatus::Disabled => "disabled",
        }
    }
}

/// A cron job definition
#[derive(Clone, Debug)]
pub struct CronJob {
    pub id:              String,
    pub name:            String,
    pub schedule:        CronSchedule,
    pub goal:            String,   // AI goal to run on each fire
    pub delivery_target: Option<String>, // channel:chat_id to reply to
    pub agent_id:        Option<String>, // which agent config to use
    pub enabled:         bool,
    pub last_run_ms:     u128,
    pub next_run_ms:     u128,
    pub run_count:       u64,
    pub status:          JobStatus,
    pub created_ms:      u128,
}

impl CronJob {
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"id":"{}","name":"{}","expression":"{}","goal":"{}","enabled":{},"last_run_ms":{},"next_run_ms":{},"run_count":{},"status":"{}"}}"#,
            self.id,
            self.name.replace('"', "\\\""),
            self.schedule.expression.replace('"', "\\\""),
            self.goal.replace('"', "\\\""),
            self.enabled,
            self.last_run_ms,
            self.next_run_ms,
            self.run_count,
            self.status.as_str(),
        )
    }
}

/// Parsed cron schedule
#[derive(Clone, Debug)]
pub struct CronSchedule {
    pub expression: String,
    /// Pre-computed interval for simple "every N minutes" schedules
    pub interval_ms: Option<u64>,
}

impl CronSchedule {
    /// Parse a cron expression or shorthand.
    /// Supports:
    ///   "every 5m"  / "every 1h"  / "every 30s"  — interval
    ///   "daily"     / "hourly"    / "weekly"       — shorthand
    ///   "HH:MM"                                   — daily at time
    ///   Standard 5-field cron: "* * * * *"        — (Session 9: full parser)
    pub fn parse(expr: &str) -> Self {
        let e = expr.trim().to_lowercase();
        let interval_ms = if let Some(rest) = e.strip_prefix("every ") {
            parse_interval(rest)
        } else {
            match e.as_str() {
                "hourly"  => Some(3_600_000),
                "daily"   => Some(86_400_000),
                "weekly"  => Some(604_800_000),
                _         => None,
            }
        };
        CronSchedule { expression: expr.to_string(), interval_ms }
    }

    /// Compute next fire time after `after_ms`.
    /// For interval-based schedules only (full cron parser in Session 9).
    pub fn next_after(&self, after_ms: u128) -> Option<u128> {
        self.interval_ms.map(|ms| after_ms + ms as u128)
    }

    pub fn is_due(&self, last_run_ms: u128, now_ms: u128) -> bool {
        match self.interval_ms {
            Some(ms) => now_ms >= last_run_ms + ms as u128,
            None     => false, // full cron: Session 9
        }
    }
}

fn parse_interval(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('s') {
        n.parse::<u64>().ok().map(|v| v * 1_000)
    } else if let Some(n) = s.strip_suffix('m') {
        n.parse::<u64>().ok().map(|v| v * 60_000)
    } else if let Some(n) = s.strip_suffix('h') {
        n.parse::<u64>().ok().map(|v| v * 3_600_000)
    } else if let Some(n) = s.strip_suffix('d') {
        n.parse::<u64>().ok().map(|v| v * 86_400_000)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_every_5m() {
        let s = CronSchedule::parse("every 5m");
        assert_eq!(s.interval_ms, Some(300_000));
    }
    #[test]
    fn parse_hourly() {
        let s = CronSchedule::parse("hourly");
        assert_eq!(s.interval_ms, Some(3_600_000));
    }
    #[test]
    fn is_due_check() {
        let s = CronSchedule::parse("every 1h");
        assert!(s.is_due(0, 3_600_001));
        assert!(!s.is_due(0, 3_599_999));
    }
}
