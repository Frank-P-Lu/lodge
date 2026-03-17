mod common;

use predicates::prelude::*;

#[test]
fn delete_removes_record() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "doomed"])
        .assert()
        .success();

    // Delete it
    common::lodge_cmd(&dir)
        .args(["tasks", "delete", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doomed"));

    // Verify it's gone
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[test]
fn delete_nonexistent_id_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "delete", "999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
