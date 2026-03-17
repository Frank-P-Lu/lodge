mod common;

use predicates::prelude::*;

#[test]
fn alter_adds_new_field() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--add-fields", "status:text"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status:text"));
}

#[test]
fn alter_existing_data_gets_null_for_new_field() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "old task"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--add-fields", "status:text"])
        .assert()
        .success();

    // Query and check that status is null for the old record
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json[0]["status"].is_null());
}

#[test]
fn alter_then_add_with_new_field() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--add-fields", "status:text"])
        .assert()
        .success();

    // Add a record using the new field
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "new task", "--status", "open"])
        .assert()
        .success()
        .stdout(predicate::str::contains("open"));
}

#[test]
fn alter_nonexistent_collection_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["alter", "nope", "--add-fields", "x:text"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
