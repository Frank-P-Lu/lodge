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
