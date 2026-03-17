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
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
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
