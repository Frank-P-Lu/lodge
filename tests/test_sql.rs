mod common;

use predicates::prelude::*;
use serde_json::Value;

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

#[test]
fn sql_bool_fields_return_true_false() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "flags", "--fields", "active:bool"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["flags", "add", "--active", "true"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT * FROM flags"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let records: Vec<Value> = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(records[0]["active"], Value::Bool(true));
}

#[test]
fn sql_bool_false_returns_false() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "flags", "--fields", "active:bool"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["flags", "add", "--active", "false"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT * FROM flags"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let records: Vec<Value> = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(records[0]["active"], Value::Bool(false));
}

#[test]
fn sql_mixed_collections_bool_fix() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "flags", "--fields", "active:bool"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["create", "counts", "--fields", "quantity:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["flags", "add", "--active", "true"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["counts", "add", "--quantity", "1"])
        .assert()
        .success();

    // Bool field should be true, not 1
    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT * FROM flags"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let records: Vec<Value> = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(records[0]["active"], Value::Bool(true));

    // Int field should remain a number, not become a bool
    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT * FROM counts"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let records: Vec<Value> = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(records[0]["quantity"], Value::Number(1.into()));
}
