mod common;

use predicates::prelude::*;

fn setup_with_fts_notes() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "notes",
            "--fields",
            "title:text, body:text",
            "--fts",
            "title,body",
        ])
        .assert()
        .success();
    dir
}

#[test]
fn create_with_fts_succeeds() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "notes",
            "--fields",
            "title:text, body:text",
            "--fts",
            "title,body",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created collection 'notes'"));
}

#[test]
fn search_finds_matching_records() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Meeting with Sarah",
            "--body",
            "Discussed Q3 plans",
        ])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Grocery list",
            "--body",
            "Buy apples and bananas",
        ])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "Sarah"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "Meeting with Sarah");
}

#[test]
fn search_respects_limit() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Rust programming",
            "--body",
            "Learning Rust",
        ])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Rust book review",
            "--body",
            "Great Rust book",
        ])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "Rust", "--limit", "1"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn search_respects_format() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Test note",
            "--body",
            "Some content",
        ])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["notes", "search", "Test", "--format", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("---"));
}

#[test]
fn search_no_results_returns_empty_array() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--title", "Hello", "--body", "World"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "nonexistent_xyz_term"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_on_collection_without_fts_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "title:text, body:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["notes", "search", "anything"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FTS not enabled"));
}

#[test]
fn alter_enable_fts_on_existing_data() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "title:text, body:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Pre-existing note",
            "--body",
            "Already here",
        ])
        .assert()
        .success();

    // Enable FTS after data exists
    common::lodge_cmd(&dir)
        .args(["alter", "notes", "--enable-fts", "title,body"])
        .assert()
        .success();

    // Search should find pre-existing data
    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "Already"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "search failed: {stderr}");
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["body"], "Already here");
}

#[test]
fn fts_stays_in_sync_after_update() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Original title",
            "--body",
            "Content",
        ])
        .assert()
        .success();

    // Update the title
    common::lodge_cmd(&dir)
        .args(["notes", "update", "1", "--title", "Updated title"])
        .assert()
        .success();

    // Old text should not be found
    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "Original"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());

    // New text should be found
    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "Updated"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn fts_stays_in_sync_after_delete() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Doomed note",
            "--body",
            "Will be deleted",
        ])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["notes", "delete", "1"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "Doomed"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());
}

#[test]
fn fts_on_non_text_field_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "data",
            "--fields",
            "name:text, count:int",
            "--fts",
            "count",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be text type"));
}
