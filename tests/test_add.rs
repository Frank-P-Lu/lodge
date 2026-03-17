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

    let json1 = common::parse_json_from_output(&out1.stdout);
    let json2 = common::parse_json_from_output(&out2.stdout);
    assert_eq!(json1["id"], 1);
    assert_eq!(json2["id"], 2);
}

#[test]
fn test_add_no_fields_errors() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no fields provided"));
}

#[test]
fn test_add_shows_confirmation() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "confirm me"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added record 1 to 'tasks'"));
}

fn setup_with_events() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "events",
            "--fields",
            "name:text, happened_at:datetime",
        ])
        .assert()
        .success();
    dir
}

#[test]
fn add_datetime_with_z_suffix() {
    let dir = setup_with_events();
    let out = common::lodge_cmd(&dir)
        .args([
            "events",
            "add",
            "--name",
            "utc event",
            "--happened_at",
            "2026-03-18T08:30:00Z",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    assert_eq!(json["happened_at"], "2026-03-18T08:30:00");
}

#[test]
fn add_datetime_with_positive_offset() {
    let dir = setup_with_events();
    let out = common::lodge_cmd(&dir)
        .args([
            "events",
            "add",
            "--name",
            "tokyo event",
            "--happened_at",
            "2026-03-18T17:30:00+09:00",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    // 17:30 +09:00 = 08:30 UTC
    assert_eq!(json["happened_at"], "2026-03-18T08:30:00");
}

#[test]
fn add_datetime_with_negative_offset() {
    let dir = setup_with_events();
    let out = common::lodge_cmd(&dir)
        .args([
            "events",
            "add",
            "--name",
            "nyc event",
            "--happened_at",
            "2026-03-18T03:30:00-05:00",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    // 03:30 -05:00 = 08:30 UTC
    assert_eq!(json["happened_at"], "2026-03-18T08:30:00");
}
