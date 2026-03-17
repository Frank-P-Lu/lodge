mod common;

use predicates::prelude::*;

fn setup_with_data() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();
    for (title, priority) in &[("Alpha", "3"), ("Beta", "1"), ("Gamma", "2")] {
        common::lodge_cmd(&dir)
            .args(["tasks", "add", "--title", title, "--priority", priority])
            .assert()
            .success();
    }
    dir
}

#[test]
fn query_returns_all_rows_as_json() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
}

#[test]
fn query_with_where_clause() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--where", "priority > 1"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn query_with_sort() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--sort", "priority ASC"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["title"], "Beta");
    assert_eq!(arr[2]["title"], "Alpha");
}

#[test]
fn query_with_limit() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--limit", "2"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn query_empty_result_returns_empty_array() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[test]
fn query_with_format_table() {
    let dir = setup_with_data();
    common::lodge_cmd(&dir)
        .args(["tasks", "query", "--format", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("id"))
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("Alpha"));
}

#[test]
fn query_with_format_csv() {
    let dir = setup_with_data();
    common::lodge_cmd(&dir)
        .args(["tasks", "query", "--format", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("id"))
        .stdout(predicate::str::contains("title"));
}

#[test]
fn query_with_fields_projects_columns() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--fields", "id,title"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    // Should have only id and title, not priority or other fields
    let first = arr[0].as_object().unwrap();
    assert!(first.contains_key("id"));
    assert!(first.contains_key("title"));
    assert!(!first.contains_key("priority"));
    assert!(!first.contains_key("created_at"));
}

#[test]
fn query_with_fields_and_where() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--fields", "title,priority", "--where", "priority > 1"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    let first = arr[0].as_object().unwrap();
    assert!(first.contains_key("title"));
    assert!(first.contains_key("priority"));
    assert!(!first.contains_key("id"));
}

#[test]
fn query_with_fields_invalid_field_errors() {
    let dir = setup_with_data();
    common::lodge_cmd(&dir)
        .args(["tasks", "query", "--fields", "id,nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown field 'nonexistent'"));
}
