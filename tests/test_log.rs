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
    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    assert_eq!(json, serde_json::json!([]));
}

#[test]
fn log_records_add() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "tasks",
            "add",
            "--title",
            "Test",
            "--done",
            "false",
            "--due",
            "2026-03-18",
        ])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log", "--verbose"])
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
        .args(["log", "--verbose"])
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
        .args(["log", "--verbose"])
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

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["operation"], "add");
    assert_eq!(entries[0]["success"], false);
    assert!(entries[0]["record_id"].is_null());
    assert!(entries[0]["error"]
        .as_str()
        .unwrap()
        .contains("Invalid value"));
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

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    // Most recent first: failed update, then successful add
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["operation"], "update");
    assert_eq!(entries[0]["success"], false);
    assert_eq!(entries[0]["record_id"], 1);
    assert!(entries[0]["error"]
        .as_str()
        .unwrap()
        .contains("Invalid value"));
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
fn log_table_omits_before_after() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log", "--format", "table"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Table format should not include before/after columns (too wide with inline JSON)
    assert!(
        !stdout.contains("before"),
        "table format should omit 'before' column"
    );
    assert!(
        !stdout.contains("after"),
        "table format should omit 'after' column"
    );
    // But should still have the key columns
    assert!(stdout.contains("operation"));
    assert!(stdout.contains("collection"));
}

#[test]
fn log_csv_omits_before_after() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log", "--format", "csv"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("before"),
        "csv format should omit 'before' column"
    );
    assert!(
        !stdout.contains("after"),
        "csv format should omit 'after' column"
    );
    assert!(stdout.contains("operation"));
}

#[test]
fn log_default_slim_output() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test", "--done", "false"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    // Slim output should have summary but NOT before/after
    assert!(
        entries[0].get("summary").is_some(),
        "default log should include 'summary' field"
    );
    assert!(
        entries[0].get("before").is_none(),
        "default log should not include 'before' field"
    );
    assert!(
        entries[0].get("after").is_none(),
        "default log should not include 'after' field"
    );
}

#[test]
fn log_verbose_includes_before_after() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test", "--done", "false"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["log", "--verbose"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    assert!(
        entries[0].get("before").is_some(),
        "verbose log should include 'before' field"
    );
    assert!(
        entries[0].get("after").is_some(),
        "verbose log should include 'after' field"
    );
}

#[test]
fn log_summary_add() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test", "--done", "false"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    let summary = entries[0]["summary"].as_str().unwrap();
    assert_eq!(summary, "added tasks: Test");
}

#[test]
fn log_summary_add_no_text() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "counts", "--fields", "val:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["counts", "add", "--val", "42"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    let summary = entries[0]["summary"].as_str().unwrap();
    assert_eq!(summary, "added counts #1");
}

#[test]
fn log_summary_update() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Test", "--done", "false"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--done", "true"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    // Most recent first
    let summary = entries[0]["summary"].as_str().unwrap();
    assert!(
        summary.contains("updated tasks #1"),
        "summary should mention updated collection and id: {summary}"
    );
    assert!(
        summary.contains("done"),
        "summary should mention changed field: {summary}"
    );
}

#[test]
fn log_summary_delete() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Bye"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "delete", "1"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    let summary = entries[0]["summary"].as_str().unwrap();
    assert_eq!(summary, "deleted tasks #1: Bye");
}

#[test]
fn log_summary_failed() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Bad", "--due", "not-a-date"])
        .assert()
        .failure();

    let out = common::lodge_cmd(&dir).args(["log"]).output().unwrap();
    let json = common::parse_json_from_output(&out.stdout);
    let entries = json.as_array().unwrap();
    let summary = entries[0]["summary"].as_str().unwrap();
    assert!(
        summary.starts_with("failed add on tasks:"),
        "failed summary should start with 'failed add on tasks:': {summary}"
    );
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
