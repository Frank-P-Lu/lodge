mod common;

use predicates::prelude::*;

#[test]
fn list_empty() {
    let dir = common::setup();
    let output = common::lodge_cmd(&dir)
        .args(["list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(parsed, serde_json::json!([]));
}

#[test]
fn list_shows_collections() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, done:bool"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["create", "logs", "--fields", "msg:text, level:text"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Find tasks and logs by name
    let names: Vec<&str> = arr.iter().map(|v| v["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"tasks"));
    assert!(names.contains(&"logs"));

    // Verify fields structure for tasks
    let tasks = arr.iter().find(|v| v["name"] == "tasks").unwrap();
    let fields = tasks["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0]["name"], "title");
    assert_eq!(fields[0]["type"], "text");
    assert_eq!(fields[1]["name"], "done");
    assert_eq!(fields[1]["type"], "bool");
}

#[test]
fn list_format_table() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "name:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["list", "--format", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name"))
        .stdout(predicate::str::contains("items"));
}
