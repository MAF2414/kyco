//! HTTP client helpers for job control API.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::Config;

pub(super) const AUTH_HEADER: &str = "X-KYCO-Token";

fn format_http_error(code: u16, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return format!("HTTP {code}");
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return format!("HTTP {code}: {body}");
    };

    let error = value
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("http_error");
    let details = value
        .get("message")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| value.get("details").map(|v| v.to_string()));

    if let Some(details) = details {
        format!("HTTP {code} {error}: {details}")
    } else {
        format!("HTTP {code} {error}: {body}")
    }
}

/// Resolve the config path - uses global config (~/.kyco/config.toml) as default,
/// but allows override via --config flag for project-local configs.
fn resolve_config_path(work_dir: &Path, config_override: Option<&PathBuf>) -> PathBuf {
    match config_override {
        Some(p) if p.is_absolute() => p.clone(),
        Some(p) => work_dir.join(p),
        None => Config::global_config_path(),
    }
}

pub(super) fn load_gui_http_settings(
    work_dir: &Path,
    config_override: Option<&PathBuf>,
) -> (u16, Option<String>) {
    // If using default global config, use Config::load() which handles auto-init
    let config = if config_override.is_none() {
        Config::load().ok()
    } else {
        let config_path = resolve_config_path(work_dir, config_override);
        Config::from_file(&config_path).ok()
    };

    let port = config
        .as_ref()
        .map(|c| c.settings.gui.http_port)
        .unwrap_or(9876);
    let token =
        config.and_then(|c| Some(c.settings.gui.http_token).filter(|t| !t.trim().is_empty()));

    (port, token)
}

fn with_auth(mut req: ureq::Request, token: Option<&str>) -> ureq::Request {
    if let Some(token) = token.filter(|t| !t.trim().is_empty()) {
        req = req.set(AUTH_HEADER, token);
    }
    req
}

pub(super) fn http_get_json(url: &str, token: Option<&str>) -> Result<serde_json::Value> {
    let req = with_auth(ureq::get(url), token);
    let resp = req.call().map_err(|e| match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            anyhow::anyhow!(format_http_error(code, &body))
        }
        other => anyhow::anyhow!(other),
    })?;

    let body = resp.into_string().context("Failed to read response body")?;
    let json: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse JSON response")?;
    Ok(json)
}

pub(super) fn http_post_json(
    url: &str,
    token: Option<&str>,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    let req = with_auth(ureq::post(url), token).set("Content-Type", "application/json");
    let resp = req
        .send_string(&serde_json::to_string(&payload).context("Failed to serialize request JSON")?)
        .map_err(|e| match e {
            ureq::Error::Status(code, resp) => {
                let body = resp.into_string().unwrap_or_default();
                anyhow::anyhow!(format_http_error(code, &body))
            }
            other => anyhow::anyhow!(other),
        })?;

    let body = resp.into_string().context("Failed to read response body")?;
    let json: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse JSON response")?;
    Ok(json)
}
