mod common;

use predicates::prelude::*;

fn setup_with_tasks() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();
    dir
}

#[test]
fn test_add_format_table() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "tasks",
            "add",
            "--title",
            "hello",
            "--priority",
            "1",
            "--format",
            "table",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"));
}

#[test]
fn test_add_format_csv() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "tasks",
            "add",
            "--title",
            "hello",
            "--priority",
            "1",
            "--format",
            "csv",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "id,title,priority,created_at,updated_at",
        ));
}

#[test]
fn test_update_format_table() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "hello", "--priority", "1"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args([
            "tasks", "update", "1", "--title", "updated", "--format", "table",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"));
}

#[test]
fn test_delete_format_table() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "doomed", "--priority", "1"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["tasks", "delete", "1", "--format", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"));
}

#[test]
fn test_add_default_json() {
    let dir = setup_with_tasks();
    let out = common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "hello", "--priority", "1"])
        .output()
        .unwrap();
    assert!(out.status.success());
    // Should contain valid JSON (parse it)
    let json = common::parse_json_from_output(&out.stdout);
    assert_eq!(json["title"], "hello");
}
