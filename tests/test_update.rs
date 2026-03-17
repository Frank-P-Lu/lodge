mod common;

use predicates::prelude::*;

fn setup_with_task() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Original", "--priority", "1"])
        .assert()
        .success();
    dir
}

#[test]
fn update_changes_field() {
    let dir = setup_with_task();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--title", "Updated"])
        .output()
        .unwrap();
    let json = common::parse_json_from_output(&output.stdout);
    assert_eq!(json["title"], "Updated");
    assert_eq!(json["priority"], 1); // unchanged
}

#[test]
fn update_nonexistent_id_errors() {
    let dir = setup_with_task();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "999", "--title", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn update_validates_field_type() {
    let dir = setup_with_task();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--priority", "not_int"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid value"));
}

#[test]
fn test_update_shows_confirmation() {
    let dir = setup_with_task();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--title", "Changed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated record 1 in 'tasks'"));
}

#[test]
fn test_update_no_fields_error() {
    let dir = setup_with_task();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no fields"))
        .stderr(predicate::str::contains("Invalid fields format").not());
}

#[test]
fn clear_field_sets_null() {
    let dir = setup_with_task();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--clear-title"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = common::parse_json_from_output(&output.stdout);
    assert!(json["title"].is_null());
    assert_eq!(json["priority"], 1); // unchanged
}

#[test]
fn clear_field_and_set_same_field_conflicts() {
    let dir = setup_with_task();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--clear-title", "--title", "New"])
        .assert()
        .failure();
}

#[test]
fn clear_field_alone_succeeds() {
    let dir = setup_with_task();
    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1", "--clear-priority"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated record 1 in 'tasks'"));
}
