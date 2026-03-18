mod common;

use predicates::prelude::*;

#[test]
fn drop_succeeds_and_collection_is_gone() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["drop", "tasks"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Dropped collection 'tasks'"));

    // Querying the dropped collection should fail
    common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .assert()
        .failure();
}

#[test]
fn drop_removes_metadata_from_list() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["drop", "tasks"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = std::str::from_utf8(&output).unwrap();
    assert!(!text.contains("tasks"), "dropped collection should not appear in list");
}

#[test]
fn drop_cleans_up_views() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, status:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["view", "create", "open_tasks", "--collection", "tasks", "--where", "status = 'open'"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["drop", "tasks"])
        .assert()
        .success();

    // View should be gone
    common::lodge_cmd(&dir)
        .args(["view", "show", "open_tasks"])
        .assert()
        .failure();
}

#[test]
fn drop_cleans_up_fts() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "body:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["drop", "notes"])
        .assert()
        .success();

    // FTS table should be gone
    common::lodge_cmd(&dir)
        .args(["sql", "SELECT * FROM notes_fts"])
        .assert()
        .failure();
}

#[test]
fn drop_nonexistent_collection_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["drop", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Collection 'nope' not found"));
}

#[test]
fn drop_cleans_up_query_log() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    // Run a query to create a tracking entry
    common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["drop", "tasks"])
        .assert()
        .success();

    // Query log entries for 'tasks' should be gone
    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT COUNT(*) as cnt FROM _lodge_query_log WHERE collection = 'tasks'"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = std::str::from_utf8(&output).unwrap();
    assert!(text.contains("\"cnt\":0") || text.contains("\"cnt\": 0"), "query log entries should be cleaned up");
}

#[test]
fn drop_preserves_mutation_log() {
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
        .args(["drop", "tasks"])
        .assert()
        .success();

    // Mutation log should still have entries
    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT COUNT(*) as cnt FROM _lodge_log WHERE collection = 'tasks'"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = std::str::from_utf8(&output).unwrap();
    assert!(!text.contains("\"cnt\":0") && !text.contains("\"cnt\": 0"), "mutation log entries should be preserved");
}
