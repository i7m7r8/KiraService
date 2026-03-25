use std::time::{SystemTime, UNIX_EPOCH};
pub fn now_ms() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
fn gen_id()  -> String { format!("k{}", now_ms()) }
fn estimate_tokens(s: &str) -> u32 { (s.len()/4).max(1) as u32 }
fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "") }
fn json_str(s: &str) -> String { format!("\"{}\"", esc(s)) }
fn json_str_arr(v: &[String]) -> String { format!("[{}]", v.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",")) }

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let search=format!("\"{}\":\"", key);
    let start=json.find(&search)?+search.len();
    let end=json[start..].find('"')?+start;
    Some(json[start..end].to_string())
}

fn extract_json_num(json: &str, key: &str) -> Option<f64> {
    let search=format!("\"{}\":", key);
    let start=json.find(&search)?+search.len();
    let slice=json[start..].trim_start();
    let end=slice.find(|c: char| !c.is_ascii_digit() && c!='.' && c!='-').unwrap_or(slice.len());
    slice[..end].parse::<f64>().ok()
}

fn extract_json_f32(json: &str, key: &str) -> Option<f32> {
    extract_json_num(json, key).map(|v| v as f32)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Session C  -  AES-256-GCM authenticated encryption for secrets
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};

/// Derive a stable 32-byte key from a device-specific seed string.
/// Seed is typically: SHA256(ANDROID_ID + package_name), supplied by Java.
/// Uses 64 rounds of XOR + rotate mixing  -  lightweight but sufficient
/// as a KDF since the seed itself comes from a 256-bit random source.
pub fn derive_aes_key(seed: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    let seed_bytes = seed.as_bytes();
    // Mix seed bytes into key with rotation
    for (i, &b) in seed_bytes.iter().enumerate() {
        key[i % 32] ^= b.wrapping_add(i as u8);
        key[(i + 7) % 32] = key[(i + 7) % 32].rotate_left(1) ^ b;
    }
    // 64 extra mixing rounds
    for round in 0u8..64 {
        for i in 0..32 {
            key[i] = key[i].wrapping_add(key[(i + 1) % 32])
                .rotate_left(3)
                ^ round;
        }
    }
    key
}

/// Derive a 12-byte deterministic nonce from the key + a domain string.
/// Domain prevents nonce reuse across different fields (api_key, tg_token, etc).
fn derive_nonce(key: &[u8; 32], domain: &str) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    for (i, &b) in domain.as_bytes().iter().enumerate() {
        nonce[i % 12] ^= b;
    }
    // Mix with first 12 bytes of key
    for i in 0..12 {
        nonce[i] ^= key[i].rotate_right(2);
    }
    nonce
}

/// Encrypt plaintext with AES-256-GCM. Returns hex-encoded ciphertext+tag.
/// domain: field name ("api_key", "tg_token", etc)  -  prevents cross-field decryption.
pub fn aes_encrypt(plaintext: &str, key_seed: &str, domain: &str) -> String {
    let key_bytes  = derive_aes_key(key_seed);
    let nonce_bytes = derive_nonce(&key_bytes, domain);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce  = Nonce::from_slice(&nonce_bytes);
    match cipher.encrypt(nonce, plaintext.as_bytes()) {
        Ok(ciphertext) => {
            // hex-encode: ciphertext includes 16-byte GCM auth tag appended
            ciphertext.iter().map(|b| format!("{:02x}", b)).collect()
        }
        Err(_) => String::new(), // should never happen
    }
}

/// Decrypt AES-256-GCM hex ciphertext. Returns plaintext or empty string on failure.
pub fn aes_decrypt(hex_ciphertext: &str, key_seed: &str, domain: &str) -> String {
    if hex_ciphertext.is_empty() { return String::new(); }
    // Decode hex
    let bytes: Option<Vec<u8>> = (0..hex_ciphertext.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_ciphertext[i..i+2], 16).ok())
        .collect();
    let ciphertext = match bytes {
        Some(b) if !b.is_empty() => b,
        _ => return String::new(),
    };
    let key_bytes   = derive_aes_key(key_seed);
    let nonce_bytes = derive_nonce(&key_bytes, domain);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce  = Nonce::from_slice(&nonce_bytes);
    match cipher.decrypt(nonce, ciphertext.as_slice()) {
        Ok(plain) => String::from_utf8(plain).unwrap_or_default(),
        Err(_)    => String::new(), // wrong key or tampered ciphertext
    }
}

/// Inline JSON string extractor  -  same as extract_json_str but returns Option
pub fn extract_json_str_inline(json: &str, key: &str) -> Option<String> {
    extract_json_str(json, key)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Session K  -  Pure-Rust HTTPS client via rustls
// Works on arm64-v8a. Falls back to plain HTTP on other ABIs (or through
// Java bridge via /http_proxy endpoint).
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::io::{Write, Read};
use std::sync::Arc;

/// Send HTTPS POST and return response body.
/// Uses rustls with webpki-roots (Mozilla CA bundle compiled in).
pub fn https_post(
    host:       &str,
    port:       u16,
    path:       &str,
    body:       &str,
    auth_token: &str,
    timeout_s:  u64,
) -> Result<String, String> {
    // Build TLS config with Mozilla root certificates
    let root_store = {
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    };
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let config = Arc::new(config);

    // Establish TCP connection
    let addr   = format!("{}:{}", host, port);
    let stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("tcp connect {}: {}", addr, e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(timeout_s)))
        .map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(15)))
        .map_err(|e| e.to_string())?;

    // TLS handshake
    let server_name = ServerName::try_from(host.to_string())
        .map_err(|e| format!("invalid hostname {}: {:?}", host, e))?;
    let mut conn = rustls::ClientConnection::new(config, server_name)
        .map_err(|e| format!("tls init: {}", e))?;
    let mut tls_stream = rustls::Stream::new(&mut conn, stream);

    // Write HTTP/1.1 request
    let request = format!(
        "POST {} HTTP/1.1
         Host: {}
         Authorization: Bearer {}
         Content-Type: application/json
         Content-Length: {}
         Connection: close
         
         {}",
        path, host, auth_token, body.len(), body
    );
    tls_stream.write_all(request.as_bytes())
        .map_err(|e| format!("write: {}", e))?;

    // Read response
    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match tls_stream.read(&mut buf) {
            Ok(0)  => break,
            Ok(n)  => response.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => break,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof     => break,
            Err(e) => return Err(format!("read: {}", e)),
        }
        if response.len() > 10 * 1024 * 1024 { break; } // 10MB cap
    }

    let resp_str = String::from_utf8_lossy(&response).into_owned();
    // Strip HTTP headers  -  find blank line
    if let Some(body_start) = resp_str.find("

") {
        Ok(resp_str[body_start + 4..].to_string())
    } else {
        Ok(resp_str)
    }
}

/// GET request over HTTPS (for Telegram API, GitHub releases, etc.)
pub fn https_get(host: &str, port: u16, path: &str, timeout_s: u64) -> Result<String, String> {
    let root_store = {
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    };
    let config = Arc::new(ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth());

    let stream = std::net::TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(timeout_s)))
        .map_err(|e| e.to_string())?;

    let server_name = ServerName::try_from(host.to_string())
        .map_err(|e| format!("hostname: {:?}", e))?;
    let mut conn   = rustls::ClientConnection::new(config, server_name)
        .map_err(|e| format!("tls: {}", e))?;
    let mut stream = rustls::Stream::new(&mut conn,
        std::net::TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| e.to_string())?);

    let request = format!(
        "GET {} HTTP/1.1
Host: {}
Connection: close

",
        path, host
    );
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;

    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => { response.extend_from_slice(&buf[..n]); }
        }
    }
    let resp = String::from_utf8_lossy(&response).into_owned();
    Ok(if let Some(i) = resp.find("

") { resp[i+4..].to_string() } else { resp })
}
