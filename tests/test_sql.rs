mod common;

use predicates::prelude::*;

#[test]
fn sql_select_returns_results() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "hello"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["sql", "SELECT title FROM tasks"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn sql_bad_query_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["sql", "SELECT * FROM nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn sql_with_format_table() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "name:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["items", "add", "--name", "widget"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["sql", "SELECT name FROM items", "--format", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("widget"))
        .stdout(predicate::str::contains("name"));
}
