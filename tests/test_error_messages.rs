mod common;

use predicates::prelude::*;

#[test]
fn test_export_no_args_error() {
    let dir = common::setup();
    let out = common::lodge_cmd(&dir).args(["export"]).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("Invalid fields format"),
        "Should not say 'Invalid fields format': {stderr}"
    );
}

#[test]
fn test_update_no_fields_error() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "test"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["tasks", "update", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no fields"))
        .stderr(predicate::str::contains("Invalid fields format").not());
}

#[test]
fn test_alter_no_flags_error() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid fields format").not());
}
