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

#[test]
fn alter_rename_field() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text,status:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--rename-field", "status:state"])
        .assert()
        .success()
        .stdout(predicate::str::contains("state:text"));

    // The renamed field should appear in queries
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "test", "--state", "open"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["state"], "open");
}

#[test]
fn alter_rename_preserves_data() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text,status:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "mytask", "--status", "done"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--rename-field", "status:state"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["state"], "done");
}

#[test]
fn alter_rename_nonexistent_field_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--rename-field", "nope:something"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn alter_rename_to_existing_name_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text,status:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--rename-field", "status:title"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn alter_rename_protected_field_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    for field in &["id", "created_at", "updated_at"] {
        common::lodge_cmd(&dir)
            .args(["alter", "tasks", "--rename-field", &format!("{field}:newname")])
            .assert()
            .failure()
            .stderr(predicate::str::contains("protected"));
    }
}

#[test]
fn alter_drop_field() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text,priority:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "test", "--priority", "1"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--drop-fields", "priority"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json[0].get("priority").is_none());
    assert_eq!(json[0]["title"], "test");
}

#[test]
fn alter_drop_nonexistent_field_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--drop-fields", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn alter_drop_protected_field_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    for field in &["id", "created_at", "updated_at"] {
        common::lodge_cmd(&dir)
            .args(["alter", "tasks", "--drop-fields", field])
            .assert()
            .failure()
            .stderr(predicate::str::contains("protected"));
    }
}

#[test]
fn alter_drop_rebuilds_fts() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text,notes:text,priority:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "searchable task", "--notes", "some notes", "--priority", "1"])
        .assert()
        .success();

    // Drop notes (a text field) — FTS should be rebuilt with just title
    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--drop-fields", "notes"])
        .assert()
        .success();

    // Search should still work on remaining text field
    common::lodge_cmd(&dir)
        .args(["tasks", "search", "searchable"])
        .assert()
        .success()
        .stdout(predicate::str::contains("searchable task"));
}

#[test]
fn alter_requires_at_least_one_flag() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks"])
        .assert()
        .failure();
}
