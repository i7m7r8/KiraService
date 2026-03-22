// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: gateway :: persistence  (Sessions 4 + 5 + 6 + 9 + 10)
//
// Disk persistence for sessions, memory, skills, cron jobs, webhooks.
// All stored under /data/data/com.kira.service/ on Android.
// Uses LZ4 compression for transcripts, plain JSON for metadata.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use std::path::{Path, PathBuf};
use std::io::{Read, Write};

pub const DATA_DIR: &str = "/data/data/com.kira.service";

fn data_path(sub: &str) -> PathBuf {
    Path::new(DATA_DIR).join(sub)
}

fn ensure_dir(path: &Path) {
    if !path.exists() { let _ = std::fs::create_dir_all(path); }
}

// ── Session persistence ───────────────────────────────────────────────────────

/// Save a session transcript to disk (LZ4 compressed JSON).
/// Path: /data/data/com.kira.service/sessions/<session_id>.lz4
pub fn save_session_transcript(session_id: &str, turns_json: &str) -> bool {
    let dir = data_path("sessions");
    ensure_dir(&dir);
    let safe_id = session_id.replace(['/', '\\', '.', ':'], "_");
    let path = dir.join(format!("{}.lz4", safe_id));
    let compressed = compress_prepend_size(turns_json.as_bytes());
    std::fs::write(&path, &compressed).is_ok()
}

/// Load a session transcript from disk.
pub fn load_session_transcript(session_id: &str) -> Option<String> {
    let safe_id = session_id.replace(['/', '\\', '.', ':'], "_");
    let path = data_path("sessions").join(format!("{}.lz4", safe_id));
    let bytes = std::fs::read(&path).ok()?;
    let decompressed = decompress_size_prepended(&bytes).ok()?;
    String::from_utf8(decompressed).ok()
}

/// Delete a session from disk.
pub fn delete_session(session_id: &str) -> bool {
    let safe_id = session_id.replace(['/', '\\', '.', ':'], "_");
    let path = data_path("sessions").join(format!("{}.lz4", safe_id));
    std::fs::remove_file(&path).is_ok()
}

/// List all saved session IDs.
pub fn list_session_ids() -> Vec<String> {
    let dir = data_path("sessions");
    if !dir.exists() { return vec![]; }
    std::fs::read_dir(&dir).ok()
        .map(|entries| entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_suffix(".lz4").map(|s| s.to_string())
            })
            .collect())
        .unwrap_or_default()
}

// ── Memory persistence ────────────────────────────────────────────────────────

/// Save memory index to disk as LZ4-compressed JSON.
/// Path: /data/data/com.kira.service/memory/index.lz4
pub fn save_memory_index(json: &str) -> bool {
    let dir = data_path("memory");
    ensure_dir(&dir);
    let path = dir.join("index.lz4");
    let compressed = compress_prepend_size(json.as_bytes());
    std::fs::write(&path, &compressed).is_ok()
}

pub fn load_memory_index() -> Option<String> {
    let path = data_path("memory").join("index.lz4");
    let bytes = std::fs::read(&path).ok()?;
    let dec = decompress_size_prepended(&bytes).ok()?;
    String::from_utf8(dec).ok()
}

/// Save per-session memory embeddings (Vec<f32> as raw bytes).
pub fn save_embeddings(session_id: &str, data: &[u8]) -> bool {
    let dir = data_path("memory");
    ensure_dir(&dir);
    let safe = session_id.replace(['/', '\\', '.', ':'], "_");
    std::fs::write(dir.join(format!("{}.emb", safe)), data).is_ok()
}

pub fn load_embeddings(session_id: &str) -> Option<Vec<u8>> {
    let safe = session_id.replace(['/', '\\', '.', ':'], "_");
    std::fs::read(data_path("memory").join(format!("{}.emb", safe))).ok()
}

// ── Skills persistence ────────────────────────────────────────────────────────

/// Load all .md skill files from /data/data/com.kira.service/skills/
pub fn load_skill_files() -> Vec<(String, String)> {
    // Returns Vec<(filename_stem, content)>
    let dir = data_path("skills");
    if !dir.exists() { return vec![]; }
    std::fs::read_dir(&dir).ok()
        .map(|entries| entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension()
                .and_then(|x| x.to_str()) == Some("md"))
            .filter_map(|e| {
                let stem = e.path().file_stem()?.to_string_lossy().to_string();
                let content = std::fs::read_to_string(e.path()).ok()?;
                Some((stem, content))
            })
            .collect())
        .unwrap_or_default()
}

/// Save a skill file.
pub fn save_skill_file(name: &str, content: &str) -> bool {
    let dir = data_path("skills");
    ensure_dir(&dir);
    let safe = name.replace(['/', '\\', ':'], "_");
    std::fs::write(dir.join(format!("{}.md", safe)), content).is_ok()
}

pub fn delete_skill_file(name: &str) -> bool {
    let safe = name.replace(['/', '\\', ':'], "_");
    std::fs::remove_file(data_path("skills").join(format!("{}.md", safe))).is_ok()
}

// ── Cron persistence ──────────────────────────────────────────────────────────

pub fn save_cron_jobs(json: &str) -> bool {
    let dir = data_path("scheduling");
    ensure_dir(&dir);
    std::fs::write(dir.join("cron_jobs.json"), json).is_ok()
}

pub fn load_cron_jobs() -> Option<String> {
    std::fs::read_to_string(data_path("scheduling").join("cron_jobs.json")).ok()
}

// ── Webhook persistence ───────────────────────────────────────────────────────

pub fn save_webhooks(json: &str) -> bool {
    let dir = data_path("scheduling");
    ensure_dir(&dir);
    std::fs::write(dir.join("webhooks.json"), json).is_ok()
}

pub fn load_webhooks() -> Option<String> {
    std::fs::read_to_string(data_path("scheduling").join("webhooks.json")).ok()
}

// ── Cron run log ──────────────────────────────────────────────────────────────

pub fn append_cron_run_log(job_id: &str, result: &str, ts: u128) {
    let dir = data_path("scheduling");
    ensure_dir(&dir);
    let path = dir.join("cron_run_log.jsonl");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let line = format!(
            r#"{{"job_id":"{}","result":"{}","ts":{}}}{}"#,
            job_id.replace('"',"\\\""),
            result.replace('"',"\\\"").replace('\n',"\\n"),
            ts, "\n"
        );
        let _ = f.write_all(line.as_bytes());
    }
    // Keep log under 500 lines
    trim_jsonl_file(&path, 500);
}

fn trim_jsonl_file(path: &Path, max_lines: usize) {
    if let Ok(content) = std::fs::read_to_string(path) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > max_lines {
            let trimmed = lines[lines.len()-max_lines..].join("\n") + "\n";
            let _ = std::fs::write(path, trimmed);
        }
    }
}
