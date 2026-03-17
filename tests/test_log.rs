mod common;

use predicates::prelude::*;

fn setup_with_tasks() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "tasks",
            "--fields",
            "title:text, done:bool, due:date",
        ])
        .assert()
        .success();
    dir
}

#[test]
fn log_empty_by_default() {
    let dir = setup_with_tasks();
    let out = common::lodge_cmd(&dir)
        .args(["log"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    assert_eq!(json, serde_json::json!([]));
}

#[test]
fn log_records_add() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test", "--done", "false", "--due", "2026-03-18"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["operation"], "add");
    assert_eq!(entries[0]["success"], true);
    assert_eq!(entries[0]["collection"], "tasks");
    assert_eq!(entries[0]["record_id"], 1);
    assert!(entries[0]["before"].is_null());
    assert_eq!(entries[0]["after"]["title"], "Test");
}

#[test]
fn log_records_update() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test", "--done", "false"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--done", "true"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log"])
        .output()
        .unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    // Most recent first: update, then add
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["operation"], "update");
    assert_eq!(entries[0]["success"], true);
    assert_eq!(entries[0]["before"]["done"], false);
    assert_eq!(entries[0]["after"]["done"], true);
}

#[test]
fn log_records_delete() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Bye"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "delete", "1"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log"])
        .output()
        .unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert_eq!(entries[0]["operation"], "delete");
    assert_eq!(entries[0]["success"], true);
    assert_eq!(entries[0]["before"]["title"], "Bye");
    assert!(entries[0]["after"].is_null());
}

#[test]
fn log_records_failed_add() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Bad", "--due", "not-a-date"])
        .assert()
        .failure();

    let out = common::lodge_cmd(&dir)
        .args(["log"])
        .output()
        .unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["operation"], "add");
    assert_eq!(entries[0]["success"], false);
    assert!(entries[0]["record_id"].is_null());
    assert!(entries[0]["error"].as_str().unwrap().contains("Invalid value"));
}

#[test]
fn log_records_failed_update() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test", "--done", "false"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--due", "not-a-date"])
        .assert()
        .failure();

    let out = common::lodge_cmd(&dir)
        .args(["log"])
        .output()
        .unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    // Most recent first: failed update, then successful add
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["operation"], "update");
    assert_eq!(entries[0]["success"], false);
    assert_eq!(entries[0]["record_id"], 1);
    assert!(entries[0]["error"].as_str().unwrap().contains("Invalid value"));
}

#[test]
fn log_filters_by_collection() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "body:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Task1"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--body", "Note1"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log", "tasks"])
        .output()
        .unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["collection"], "tasks");
}

#[test]
fn log_respects_limit() {
    let dir = setup_with_tasks();
    for i in 1..=5 {
        common::lodge_cmd(&dir)
            .args(["tasks", "add", "--title", &format!("Task{i}")])
            .assert()
            .success();
    }

    let out = common::lodge_cmd(&dir)
        .args(["log", "--limit", "2"])
        .output()
        .unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn log_format_flag() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["log", "--format", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("operation"))
        .stdout(predicate::str::contains("add"));
}

#[test]
fn log_reserved_name() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "log", "--fields", "msg:text"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Reserved name"));
}
