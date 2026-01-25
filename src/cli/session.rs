//! Session management CLI commands.
//!
//! Lists and shows sessions stored in the SDK Bridge.

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};

use crate::agent::bridge::BridgeClient;

/// List stored sessions
pub fn list(session_type: Option<String>, limit: Option<usize>, json: bool) -> Result<()> {
    let client = BridgeClient::new();

    // Ensure bridge is running
    if client.health_check().is_err() {
        anyhow::bail!(
            "SDK Bridge is not running. Start the KYCo GUI or run a job to start the bridge."
        );
    }

    let sessions = client
        .list_sessions(session_type.as_deref())
        .context("Failed to list sessions")?;

    // Apply limit
    let sessions: Vec<_> = if let Some(n) = limit {
        sessions.into_iter().take(n).collect()
    } else {
        sessions
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&sessions)?);
        return Ok(());
    }

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!(
        "{:<40} {:<8} {:<20} {:<20} {:>6} {:>12}",
        "ID", "TYPE", "CREATED", "LAST ACTIVE", "TURNS", "COST"
    );
    println!("{}", "-".repeat(110));

    for session in &sessions {
        let created = Utc
            .timestamp_millis_opt(session.created_at as i64)
            .single()
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".to_string());

        let last_active = Utc
            .timestamp_millis_opt(session.last_active_at as i64)
            .single()
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "?".to_string());

        let id_short = if session.id.len() > 38 {
            format!("{}...", &session.id[..35])
        } else {
            session.id.clone()
        };

        println!(
            "{:<40} {:<8} {:<20} {:<20} {:>6} ${:>10.4}",
            id_short,
            session.session_type,
            created,
            last_active,
            session.turn_count,
            session.total_cost_usd
        );
    }

    println!();
    println!("Total: {} session(s)", sessions.len());

    Ok(())
}

/// Show details of a specific session
pub fn show(session_id: &str, json: bool) -> Result<()> {
    let client = BridgeClient::new();

    // Ensure bridge is running
    if client.health_check().is_err() {
        anyhow::bail!(
            "SDK Bridge is not running. Start the KYCo GUI or run a job to start the bridge."
        );
    }

    let session = client
        .get_session(session_id)
        .context("Failed to get session")?;

    let Some(session) = session else {
        anyhow::bail!("Session not found: {}", session_id);
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&session)?);
        return Ok(());
    }

    let created = Utc
        .timestamp_millis_opt(session.created_at as i64)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "?".to_string());

    let last_active = Utc
        .timestamp_millis_opt(session.last_active_at as i64)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "?".to_string());

    println!("Session ID:    {}", session.id);
    println!("Type:          {}", session.session_type);
    println!("Created:       {}", created);
    println!("Last Active:   {}", last_active);
    println!("Working Dir:   {}", session.cwd);
    println!("Turn Count:    {}", session.turn_count);
    println!("Total Tokens:  {}", session.total_tokens);
    println!("Total Cost:    ${:.4}", session.total_cost_usd);

    Ok(())
}
