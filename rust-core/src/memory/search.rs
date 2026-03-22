// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: memory :: search
//
// MMR (Maximal Marginal Relevance) re-ranking for memory search results.
// Mirrors OpenClaw: src/memory/mmr.ts
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::index::cosine_similarity;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub id:       String,
    pub content:  String,
    pub score:    f32,
    pub tags:     Vec<String>,
}

impl SearchResult {
    pub fn to_json(&self) -> String {
        let tags_json: Vec<String> = self.tags.iter()
            .map(|t| format!("\"{}\"", t.replace('"', "\\\"")))
            .collect();
        format!(
            r#"{{"id":"{}","content":"{}","score":{:.4},"tags":[{}]}}"#,
            self.id,
            self.content.replace('"', "\\\"").replace('\n', "\\n"),
            self.score,
            tags_json.join(",")
        )
    }
}

/// MMR re-ranking: balance relevance vs. diversity.
/// Mirrors OpenClaw src/memory/mmr.ts mmr()
///
/// lambda=1.0 → pure relevance
/// lambda=0.0 → pure diversity
/// lambda=0.5 → balanced (default)
pub fn mmr_rerank(
    candidates: &[(String, String, Vec<f32>, f32)], // (id, content, embedding, score)
    query_embedding: &[f32],
    k: usize,
    lambda: f32,
) -> Vec<usize> {
    if candidates.is_empty() || k == 0 { return vec![]; }

    let n = candidates.len();
    let mut selected: Vec<usize> = Vec::with_capacity(k);
    let mut remaining: Vec<usize> = (0..n).collect();

    // Query similarities (relevance scores)
    let query_sims: Vec<f32> = candidates.iter()
        .map(|(_, _, emb, _)| cosine_similarity(emb, query_embedding))
        .collect();

    for _ in 0..k.min(n) {
        let best = remaining.iter().enumerate().max_by(|&(_, &a), &(_, &b)| {
            let rel_a = query_sims[a];
            let rel_b = query_sims[b];

            // Redundancy: max similarity to already-selected
            let red_a = selected.iter()
                .map(|&s| cosine_similarity(
                    &candidates[a].2,
                    &candidates[s].2
                ))
                .fold(0.0f32, f32::max);
            let red_b = selected.iter()
                .map(|&s| cosine_similarity(
                    &candidates[b].2,
                    &candidates[s].2
                ))
                .fold(0.0f32, f32::max);

            let score_a = lambda * rel_a - (1.0 - lambda) * red_a;
            let score_b = lambda * rel_b - (1.0 - lambda) * red_b;
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        match best {
            Some((pos, &idx)) => {
                selected.push(idx);
                remaining.remove(pos);
            }
            None => break,
        }
    }

    selected
}

/// Simple keyword-based search result scoring (pre-embedding fallback)
pub fn keyword_score(content: &str, tags: &[String], query: &str) -> f32 {
    let q = query.to_lowercase();
    let haystack = format!("{} {}", content.to_lowercase(), tags.join(" ").to_lowercase());
    let terms: Vec<&str> = q.split_whitespace().collect();
    if terms.is_empty() { return 0.0; }
    let matches = terms.iter().filter(|&&t| haystack.contains(t)).count();
    matches as f32 / terms.len() as f32
}
