// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: scheduling :: webhooks
// Session 1: types.  Session 10: full impl.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Clone, Debug)]
pub struct WebhookRegistration {
    pub id:              String,
    pub token:           String,   // URL token: POST /webhook/:token
    pub secret:          String,   // HMAC secret for verification
    pub goal_template:   String,   // AI goal; {body} is replaced with payload
    pub delivery_target: Option<String>,
    pub enabled:         bool,
    pub created_ms:      u128,
    pub fire_count:      u64,
}

impl WebhookRegistration {
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"id":"{}","token":"{}","enabled":{},"fire_count":{}}}"#,
            self.id, self.token, self.enabled, self.fire_count
        )
    }
}
