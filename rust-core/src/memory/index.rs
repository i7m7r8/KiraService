// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: memory :: index
//
// In-memory vector store with LZ4-compressed persistence.
// Session 1: types + cosine similarity.
// Session 5: embedding integration.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A single memory entry with optional embedding vector
#[derive(Clone, Debug)]
pub struct MemoryEntry {
    pub id:           String,
    pub content:      String,
    pub tags:         Vec<String>,
    pub session:      Option<String>,
    pub ts:           u128,
    pub access_count: u32,
    pub relevance:    f32,
    /// Embedding vector  -  None until embeddings are computed (Session 5)
    pub embedding:    Option<Vec<f32>>,
}

impl MemoryEntry {
    pub fn new(id: &str, content: &str, tags: Vec<String>, session: Option<String>, ts: u128) -> Self {
        MemoryEntry {
            id: id.to_string(),
            content: content.to_string(),
            tags,
            session,
            ts,
            access_count: 0,
            relevance: 1.0,
            embedding: None,
        }
    }

    pub fn to_json(&self) -> String {
        let tags_json: Vec<String> = self.tags.iter()
            .map(|t| format!("\"{}\"", t.replace('"', "\\\"")))
            .collect();
        let session_json = self.session.as_deref()
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|| "null".to_string());
        format!(
            r#"{{"id":"{}","content":"{}","tags":[{}],"session":{},"ts":{},"access_count":{},"relevance":{:.3}}}"#,
            self.id,
            self.content.replace('"', "\\\"").replace('\n', "\\n"),
            tags_json.join(","),
            session_json,
            self.ts,
            self.access_count,
            self.relevance,
        )
    }

    /// Temporal decay score  -  entries accessed recently + created recently score higher.
    /// Mirrors OpenClaw: src/memory/temporal-decay.ts
    pub fn decayed_score(&self, now_ms: u128, base_score: f32) -> f32 {
        let age_days = (now_ms.saturating_sub(self.ts)) as f64 / (1000.0 * 86400.0);
        let decay = (-0.05 * age_days).exp() as f32;  // half-life ~14 days
        let access_boost = (1.0 + self.access_count as f32 * 0.1).min(2.0);
        base_score * decay * access_boost
    }
}

/// In-process vector memory store
pub struct MemoryStore {
    entries: Vec<MemoryEntry>,
    max_entries: usize,
}

impl MemoryStore {
    pub fn new(max_entries: usize) -> Self {
        MemoryStore { entries: Vec::new(), max_entries }
    }

    pub fn add(&mut self, entry: MemoryEntry) {
        // Remove duplicate by id if exists
        self.entries.retain(|e| e.id != entry.id);
        self.entries.push(entry);
        // Evict oldest if over limit
        if self.entries.len() > self.max_entries {
            self.entries.sort_by_key(|e| e.ts);
            self.entries.remove(0);
        }
    }

    pub fn get(&self, id: &str) -> Option<&MemoryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() < before
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    pub fn all(&self) -> &[MemoryEntry] { &self.entries }

    /// Keyword search  -  find entries whose content/tags contain query terms.
    /// Returns indices sorted by match score.
    pub fn keyword_search(&self, query: &str, limit: usize) -> Vec<usize> {
        let terms: Vec<&str> = query.split_whitespace().collect();
        let mut scored: Vec<(usize, u32)> = self.entries.iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let haystack = format!(
                    "{} {}",
                    e.content.to_lowercase(),
                    e.tags.join(" ").to_lowercase()
                );
                let score: u32 = terms.iter()
                    .filter(|&&t| haystack.contains(&t.to_lowercase()))
                    .count() as u32;
                if score > 0 { Some((i, score)) } else { None }
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().take(limit).map(|(i, _)| i).collect()
    }

    /// Tag filter
    pub fn by_tag(&self, tag: &str) -> Vec<&MemoryEntry> {
        self.entries.iter()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .collect()
    }

    pub fn list_json(&self) -> String {
        let items: Vec<String> = self.entries.iter().map(|e| e.to_json()).collect();
        format!("[{}]", items.join(","))
    }

    /// Access an entry (increments access_count for temporal scoring)
    pub fn touch(&mut self, id: &str) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.access_count += 1;
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self { Self::new(10_000) }
}

// ── Cosine similarity (used for vector search in Session 5) ─────────────────

/// Cosine similarity between two equal-length vectors.
/// Returns value in [-1.0, 1.0]. Higher = more similar.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cosine_identical() {
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }
    #[test]
    fn cosine_orthogonal() {
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }
}
