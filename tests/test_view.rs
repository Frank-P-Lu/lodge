mod common;

use predicates::prelude::*;

fn setup_with_tasks() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "tasks",
            "--fields",
            "title:text, priority:int, status:text",
        ])
        .assert()
        .success();
    for (title, priority, status) in &[
        ("Alpha", "3", "open"),
        ("Beta", "1", "closed"),
        ("Gamma", "2", "open"),
    ] {
        common::lodge_cmd(&dir)
            .args([
                "tasks",
                "add",
                "--title",
                title,
                "--priority",
                priority,
                "--status",
                status,
            ])
            .assert()
            .success();
    }
    dir
}

#[test]
fn view_create_succeeds() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "urgent",
            "--collection",
            "tasks",
            "--where",
            "priority = 1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created view 'urgent'"));
}

#[test]
fn view_create_duplicate_errors() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["view", "create", "v1", "--collection", "tasks"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["view", "create", "v1", "--collection", "tasks"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn view_create_nonexistent_collection_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["view", "create", "v1", "--collection", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn view_list_returns_views() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "urgent",
            "--collection",
            "tasks",
            "--where",
            "priority = 1",
        ])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["view", "create", "all_tasks", "--collection", "tasks"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["view", "list"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn view_list_empty() {
    let dir = common::setup();
    let output = common::lodge_cmd(&dir)
        .args(["view", "list"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[test]
fn view_run_returns_filtered_records() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "urgent",
            "--collection",
            "tasks",
            "--where",
            "priority = 1",
        ])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["view", "run", "urgent"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Beta");
}

#[test]
fn view_run_with_sort_and_limit() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "top2",
            "--collection",
            "tasks",
            "--sort",
            "priority ASC",
            "--limit",
            "2",
        ])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["view", "run", "top2"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["title"], "Beta");
}

#[test]
fn view_run_with_format() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["view", "create", "all", "--collection", "tasks"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["view", "run", "all", "--format", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("Alpha"));
}

#[test]
fn view_run_nonexistent_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["view", "run", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn view_delete_succeeds() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["view", "create", "v1", "--collection", "tasks"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["view", "delete", "v1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted view 'v1'"));

    // Verify it's gone
    let output = common::lodge_cmd(&dir)
        .args(["view", "list"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[test]
fn view_delete_nonexistent_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["view", "delete", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn view_appears_in_help() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["view", "create", "urgent", "--collection", "tasks"])
        .assert()
        .success();

    // Re-run help — the view about text should list existing views
    common::lodge_cmd(&dir)
        .args(["view", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("urgent"));
}

#[test]
fn view_show_returns_definition() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "open_tasks",
            "--collection",
            "tasks",
            "--where",
            "status = 'open'",
        ])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["view", "show", "open_tasks"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["name"], "open_tasks");
    assert_eq!(json["collection"], "tasks");
    assert_eq!(json["where"], "status = 'open'");
}

#[test]
fn view_show_nonexistent_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["view", "show", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn view_update_changes_filter() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["view", "create", "open_tasks", "--collection", "tasks"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args([
            "view",
            "update",
            "open_tasks",
            "--where",
            "status = 'closed'",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated view 'open_tasks'"));

    // Verify the filter changed
    let output = common::lodge_cmd(&dir)
        .args(["view", "run", "open_tasks"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["status"], "closed");
}

#[test]
fn view_update_nonexistent_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["view", "update", "nope", "--where", "x=1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn view_update_no_fields_errors() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["view", "create", "v1", "--collection", "tasks"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["view", "update", "v1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

#[test]
fn run_shorthand_executes_view() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "open_tasks",
            "--collection",
            "tasks",
            "--where",
            "status = 'open'",
        ])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["run", "open_tasks"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let records = json.as_array().unwrap();
    assert_eq!(records.len(), 2);
}

#[test]
fn run_shorthand_meta_flag() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "open_tasks",
            "--collection",
            "tasks",
            "--where",
            "status = 'open'",
        ])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["run", "open_tasks", "--meta"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["view"], "open_tasks");
    assert_eq!(json["collection"], "tasks");
    assert_eq!(json["records"].as_array().unwrap().len(), 2);
}

#[test]
fn run_shorthand_nonexistent_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["run", "no_such_view"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no_such_view"));
}

#[test]
fn view_run_meta_flag() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args([
            "view",
            "create",
            "open_tasks",
            "--collection",
            "tasks",
            "--where",
            "status = 'open'",
        ])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["view", "run", "open_tasks", "--meta"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["view"], "open_tasks");
    assert_eq!(json["collection"], "tasks");
    let records = json["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
}
