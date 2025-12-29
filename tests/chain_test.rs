//! Tests for chain configuration and GUI integration
//!
//! This test file focuses on bugs identified in the review:
//! 1. Agent field whitespace not trimmed before storage
//! 2. Duplicate trigger/skip states not deduplicated
//! 3. Mode validation
//! 4. Edit fields not cleared after chain deletion

mod chain_test;
