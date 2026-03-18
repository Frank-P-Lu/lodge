mod common;

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
fn query_tracking_table_exists() {
    let dir = common::setup();
    let output = common::lodge_cmd(&dir)
        .args([
            "sql",
            "SELECT name FROM sqlite_master WHERE type='table' AND name='_lodge_query_log'",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "_lodge_query_log");
}

#[test]
fn query_increments_call_count() {
    let dir = setup_with_tasks();
    // Run same query 3 times
    for _ in 0..3 {
        common::lodge_cmd(&dir)
            .args(["tasks", "query", "--where", "priority = 1"])
            .assert()
            .success();
    }
    let output = common::lodge_cmd(&dir)
        .args([
            "sql",
            "SELECT call_count FROM _lodge_query_log WHERE query_type = 'query'",
        ])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["call_count"], 3);
}

#[test]
fn different_queries_tracked_separately() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "query", "--where", "priority = 1"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "query", "--where", "priority = 2"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args([
            "sql",
            "SELECT COUNT(*) as cnt FROM _lodge_query_log WHERE query_type = 'query'",
        ])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["cnt"], 2);
}

#[test]
fn search_tracked() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["tasks", "search", "Alpha"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT query_type, collection FROM _lodge_query_log"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["query_type"], "search");
    assert_eq!(arr[0]["collection"], "tasks");
}

#[test]
fn view_run_tracked() {
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
        .args(["view", "run", "urgent"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["sql", "SELECT query_type, collection FROM _lodge_query_log"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["query_type"], "view_run");
    assert_eq!(arr[0]["collection"], "urgent");
}

// Phase 3 tests: threshold and suggestions

#[test]
fn suggestion_emitted_on_threshold() {
    let dir = setup_with_tasks();
    // Run same query 3 times (default threshold)
    for _ in 0..2 {
        let output = common::lodge_cmd(&dir)
            .args(["tasks", "query", "--where", "priority = 1"])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("Hint:"),
            "Should not suggest before threshold"
        );
    }
    // 3rd run should trigger hint
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--where", "priority = 1"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Hint:"),
        "Should suggest on threshold crossing"
    );
    assert!(stderr.contains("lodge view create"));
}

#[test]
fn suggestion_not_repeated() {
    let dir = setup_with_tasks();
    // Run 3 times to trigger suggestion
    for _ in 0..3 {
        common::lodge_cmd(&dir)
            .args(["tasks", "query", "--where", "priority = 1"])
            .assert()
            .success();
    }
    // 4th run should NOT repeat
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--where", "priority = 1"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Hint:"),
        "Should not repeat suggestion after threshold"
    );
}

#[test]
fn view_run_never_suggests() {
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

    for _ in 0..5 {
        let output = common::lodge_cmd(&dir)
            .args(["view", "run", "urgent"])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("Hint:"), "View runs should never suggest");
    }
}

#[test]
fn custom_threshold() {
    let dir = setup_with_tasks();
    common::lodge_cmd(&dir)
        .args(["set", "view_suggest_threshold", "2"])
        .assert()
        .success();

    // 1st run — no hint
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--where", "priority = 1"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Hint:"));

    // 2nd run — hint (threshold=2)
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query", "--where", "priority = 1"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Hint:"),
        "Should suggest at custom threshold of 2"
    );
}
