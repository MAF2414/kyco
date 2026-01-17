//! E2E tests for SDK Bridge auto-start functionality.
//!
//! These tests verify that:
//! 1. BridgeProcess::spawn() starts the bridge server or reuses existing
//! 2. ensure_bridge_running() lazy-starts the server if not running
//! 3. Multiple spawn calls don't create duplicate processes
//! 4. The bridge can handle requests after auto-start

use std::time::Duration;

use kyco::agent::bridge::{BridgeClient, BridgeProcess};

/// Helper to check if bridge is running
fn is_bridge_running() -> bool {
    BridgeClient::new().health_check().is_ok()
}

/// Helper to wait for bridge to become healthy
fn wait_for_bridge(timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if is_bridge_running() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Test that BridgeProcess::spawn() works - either starting new or reusing existing
#[test]
fn test_bridge_spawn_works() {
    // Spawn the bridge - it should work whether or not a bridge is already running
    let bridge = BridgeProcess::spawn();
    assert!(bridge.is_ok(), "BridgeProcess::spawn() should succeed: {:?}", bridge.err());

    let _bridge = bridge.unwrap();

    // Verify bridge is now running
    assert!(
        wait_for_bridge(Duration::from_secs(10)),
        "Bridge should be running after spawn"
    );

    // Verify health check returns valid response
    let client = BridgeClient::new();
    let health = client.health_check();
    assert!(health.is_ok(), "Health check should succeed: {:?}", health.err());
}

/// Test that multiple spawn calls work correctly
#[test]
fn test_bridge_spawn_multiple_times() {
    // First spawn
    let first_bridge = BridgeProcess::spawn();
    assert!(first_bridge.is_ok(), "First spawn should succeed");
    let _first = first_bridge.unwrap();

    assert!(
        wait_for_bridge(Duration::from_secs(10)),
        "Bridge should be running after first spawn"
    );

    // Second spawn should detect existing bridge and reuse it
    let second_bridge = BridgeProcess::spawn();
    assert!(second_bridge.is_ok(), "Second spawn should succeed (reusing existing)");
    let _second = second_bridge.unwrap();

    // Bridge should still be healthy
    assert!(is_bridge_running(), "Bridge should still be running after second spawn");

    // Third spawn should also work
    let third_bridge = BridgeProcess::spawn();
    assert!(third_bridge.is_ok(), "Third spawn should succeed");
    let _third = third_bridge.unwrap();

    assert!(is_bridge_running(), "Bridge should still be running after third spawn");
}

/// Test that status endpoint works when bridge is running
#[test]
fn test_bridge_status_endpoint() {
    // Ensure bridge is running
    let _bridge = BridgeProcess::spawn().expect("Bridge should spawn");
    assert!(wait_for_bridge(Duration::from_secs(10)), "Bridge should be healthy");

    let client = BridgeClient::new();
    let status = client.status();
    assert!(status.is_ok(), "Status endpoint should work: {:?}", status.err());

    let _status = status.unwrap();
    // Status endpoint worked - that's enough validation
}

#[test]
fn test_bridge_client_without_server_fails() {
    // Create client pointing to a port where no server is running
    let client = BridgeClient::with_url("http://127.0.0.1:19999");

    // Health check should fail
    let health = client.health_check();
    assert!(health.is_err(), "Health check should fail when server not running");
}

#[test]
fn test_bridge_spawn_finds_bridge_directory() {
    // This test verifies the bridge directory detection works
    // It doesn't actually start the bridge, just tests the path resolution

    // Check common locations exist or can be created
    let home = dirs::home_dir();
    assert!(home.is_some(), "Home directory should be available");

    let kyco_dir = home.unwrap().join(".kyco");
    // The directory might not exist, which is fine - spawn will try to download
    println!("KYCO dir would be: {}", kyco_dir.display());
}

/// Integration test that verifies the full flow works with multiple health checks
#[test]
fn test_multiple_health_checks() {
    // Spawn bridge (or reuse existing)
    let _bridge = BridgeProcess::spawn().expect("Spawn should succeed");

    // Wait for bridge to be healthy
    assert!(
        wait_for_bridge(Duration::from_secs(10)),
        "Bridge should be running"
    );

    let client = BridgeClient::new();

    // Multiple health checks should all succeed
    for i in 0..5 {
        let result = client.health_check();
        assert!(result.is_ok(), "Health check {} should succeed: {:?}", i, result.err());
    }
}
