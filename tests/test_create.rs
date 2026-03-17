mod common;

use predicates::prelude::*;

#[test]
fn create_collection_succeeds() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created collection 'tasks'"));
}

#[test]
fn create_collection_table_has_correct_columns() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();

    // Check table info
    common::lodge_cmd(&dir)
        .args(["sql", "PRAGMA table_info(tasks)"])
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("priority"))
        .stdout(predicate::str::contains("id"))
        .stdout(predicate::str::contains("created_at"))
        .stdout(predicate::str::contains("updated_at"));
}

#[test]
fn create_duplicate_collection_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn create_with_invalid_type_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:banana"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid field type"));
}

#[test]
fn create_with_reserved_name_errors() {
    let dir = common::setup();
    for name in &["init", "create", "alter", "sql", "help"] {
        common::lodge_cmd(&dir)
            .args(["create", name, "--fields", "title:text"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("Reserved name"));
    }
}

#[test]
fn create_with_all_field_types() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "everything",
            "--fields",
            "name:text, count:int, score:real, active:bool, born:date, logged:datetime",
        ])
        .assert()
        .success();
}
