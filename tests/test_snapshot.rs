mod common;

use predicates::prelude::*;

fn setup_with_notes() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "title:text, body:text"])
        .assert()
        .success();
    dir
}

#[test]
fn snapshot_creates_file_in_lodge_snapshots() {
    let dir = setup_with_notes();
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--title", "Hello", "--body", "World"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir).args(["snapshot"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(".lodge/snapshots/"));
    assert!(stdout.contains(".json"));

    // Verify the file actually exists
    let snapshot_dir = dir.path().join(".lodge/snapshots");
    assert!(snapshot_dir.exists());
    let entries: Vec<_> = std::fs::read_dir(&snapshot_dir).unwrap().collect();
    assert_eq!(entries.len(), 1);
}

#[test]
fn snapshot_with_custom_output_path() {
    let dir = setup_with_notes();
    let custom_path = dir.path().join("my_snapshot.json");

    common::lodge_cmd(&dir)
        .args(["snapshot", "--output", custom_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("my_snapshot.json"));

    assert!(custom_path.exists());
}

#[test]
fn snapshot_contains_all_collections_and_records() {
    let dir = setup_with_notes();
    // Create a second collection
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "name:text, priority:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--title", "Note1", "--body", "Body1"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--name", "Task1", "--priority", "5"])
        .assert()
        .success();

    let custom_path = dir.path().join("snap.json");
    common::lodge_cmd(&dir)
        .args(["snapshot", "--output", custom_path.to_str().unwrap()])
        .assert()
        .success();

    let data = std::fs::read_to_string(&custom_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&data).unwrap();

    assert_eq!(json["lodge_version"], 1);
    assert!(json["collections"]["notes"].is_object());
    assert!(json["collections"]["tasks"].is_object());
    assert_eq!(
        json["collections"]["notes"]["records"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        json["collections"]["tasks"]["records"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        json["collections"]["notes"]["records"][0]["title"]
            .as_str()
            .unwrap()
            == "Note1"
    );
}

#[test]
fn snapshot_round_trip_restore() {
    let dir = setup_with_notes();
    common::lodge_cmd(&dir)
        .args([
            "notes",
            "add",
            "--title",
            "Important",
            "--body",
            "Don't lose me",
        ])
        .assert()
        .success();

    let snap_path = dir.path().join("backup.json");
    common::lodge_cmd(&dir)
        .args(["snapshot", "--output", snap_path.to_str().unwrap()])
        .assert()
        .success();

    // Delete all records via SQL
    common::lodge_cmd(&dir)
        .args(["sql", "DELETE FROM notes"])
        .assert()
        .success();

    // Verify it's gone
    let out = common::lodge_cmd(&dir)
        .args(["notes", "query"])
        .output()
        .unwrap();
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());

    // Restore
    common::lodge_cmd(&dir)
        .args(["restore", snap_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Restored from"));

    // Verify data is back
    let out = common::lodge_cmd(&dir)
        .args(["notes", "query"])
        .output()
        .unwrap();
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "Important");
    assert_eq!(results[0]["body"], "Don't lose me");
}

#[test]
fn restore_nonexistent_file_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["restore", "/nonexistent/path.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read snapshot file"));
}

#[test]
fn restore_invalid_json_errors() {
    let dir = common::setup();
    let bad_file = dir.path().join("bad.json");
    std::fs::write(&bad_file, "not json at all").unwrap();

    common::lodge_cmd(&dir)
        .args(["restore", bad_file.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid JSON"));
}

#[test]
fn snapshot_empty_db_succeeds() {
    let dir = common::setup();
    let snap_path = dir.path().join("empty.json");
    common::lodge_cmd(&dir)
        .args(["snapshot", "--output", snap_path.to_str().unwrap()])
        .assert()
        .success();

    let data = std::fs::read_to_string(&snap_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert!(json["collections"].as_object().unwrap().is_empty());
}
