mod common;

use predicates::prelude::*;

fn setup_with_tasks() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "tasks",
            "--fields",
            "title:text, priority:int, due:date",
        ])
        .assert()
        .success();
    dir
}

#[test]
fn add_record_returns_json_with_id() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Buy milk", "--priority", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("Buy milk"))
        .stdout(predicate::str::contains("\"created_at\""));
}

#[test]
fn add_record_validates_int() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "tasks",
            "add",
            "--title",
            "foo",
            "--priority",
            "not_a_number",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid value"));
}

#[test]
fn add_record_validates_date() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "foo", "--due", "not-a-date"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid value"));
}

#[test]
fn add_record_accepts_valid_date() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "foo", "--due", "2025-06-15"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-06-15"));
}

#[test]
fn add_record_optional_fields() {
    let dir = setup_with_tasks();
    // Only provide title, leave priority and due as NULL
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "minimal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("minimal"));
}

#[test]
fn add_multiple_records_get_incrementing_ids() {
    let dir = setup_with_tasks();
    let out1 = common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "first"])
        .output()
        .unwrap();
    let out2 = common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "second"])
        .output()
        .unwrap();

    let json1: serde_json::Value = serde_json::from_slice(&out1.stdout).unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&out2.stdout).unwrap();
    assert_eq!(json1["id"], 1);
    assert_eq!(json2["id"], 2);
}
