//! HTTP token generation utilities

/// Generate a random token for authenticating IDE extension requests.
///
/// The token is hex-encoded and safe to embed in `config.toml`.
pub fn generate_http_token() -> String {
    let mut bytes = [0u8; 32];
    if getrandom::getrandom(&mut bytes).is_ok() {
        return hex_encode(&bytes);
    }

    // Fallback: best-effort token if OS RNG is unavailable.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    let mixed = nanos ^ (pid.rotate_left(17));
    hex_encode(&mixed.to_le_bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
