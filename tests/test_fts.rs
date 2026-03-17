mod common;

#[allow(unused_imports)]
use predicates::prelude::*;

fn setup_with_fts_notes() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "title:text, body:text"])
        .assert()
        .success();
    dir
}

#[test]
fn create_with_text_fields_enables_fts() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "title:text, body:text"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created collection 'notes'"));

    // FTS should work immediately
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--title", "Hello", "--body", "World"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["notes", "search", "Hello"])
        .assert()
        .success();
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
fn search_on_collection_without_text_fields_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "metrics", "--fields", "value:int, score:real"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["metrics", "search", "anything"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FTS not enabled"));
}

#[test]
fn alter_adding_text_field_enables_fts_on_existing_data() {
    let dir = common::setup();
    // Create with only non-text fields — no FTS
    common::lodge_cmd(&dir)
        .args(["create", "events", "--fields", "count:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["events", "add", "--count", "5"])
        .assert()
        .success();

    // Add a text field — FTS should be created and existing rows indexed
    common::lodge_cmd(&dir)
        .args(["alter", "events", "--add-fields", "label:text"])
        .assert()
        .success();

    // Update the existing record with text data
    common::lodge_cmd(&dir)
        .args(["events", "update", "1", "--label", "important milestone"])
        .assert()
        .success();

    // Search should find it
    let out = common::lodge_cmd(&dir)
        .args(["events", "search", "milestone"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "search failed: {stderr}");
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn alter_adding_text_field_rebuilds_existing_fts() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Original note",
            "--body",
            "Some content",
        ])
        .assert()
        .success();

    // Add another text field — FTS should be rebuilt including the new field
    common::lodge_cmd(&dir)
        .args(["alter", "notes", "--add-fields", "tags:text"])
        .assert()
        .success();

    // Update with new field data
    common::lodge_cmd(&dir)
        .args(["notes", "update", "1", "--tags", "important work"])
        .assert()
        .success();

    // Search on existing field still works
    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "Original"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);

    // Search on new field also works
    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "important"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
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
fn search_empty_query_returns_empty_array() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--title", "Hello", "--body", "World"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", ""])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_short_query_returns_empty_array() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--title", "Hello", "--body", "World"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "ab"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_with_single_quotes_succeeds() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "alert('xss')",
            "--body",
            "security test",
        ])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "alert('xss')"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "alert('xss')");
}

#[test]
fn search_with_double_quotes_succeeds() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "He said \"hello\" today",
            "--body",
            "conversation",
        ])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "said \"hello\""])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn search_cjk_text() {
    let dir = setup_with_fts_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "日本語のテスト",
            "--body",
            "Japanese text test",
        ])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["notes", "search", "日本語"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "日本語のテスト");
}
