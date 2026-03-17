mod common;

#[test]
fn test_json_keys_in_schema_order() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "zebra:text, alpha:int"])
        .assert()
        .success();
    let out = common::lodge_cmd(&dir)
        .args(["items", "add", "--zebra", "hello", "--alpha", "42"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    let keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        vec!["id", "zebra", "alpha", "created_at", "updated_at"]
    );
}

#[test]
fn test_query_keys_in_schema_order() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "zebra:text, alpha:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["items", "add", "--zebra", "hello", "--alpha", "42"])
        .assert()
        .success();
    let out = common::lodge_cmd(&dir)
        .args(["items", "query"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    let keys: Vec<&String> = results[0].as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        vec!["id", "zebra", "alpha", "created_at", "updated_at"]
    );
}

#[test]
fn test_table_columns_in_schema_order() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "zebra:text, alpha:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["items", "add", "--zebra", "hello", "--alpha", "42"])
        .assert()
        .success();
    let out = common::lodge_cmd(&dir)
        .args(["items", "query", "--format", "table"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let header = stdout.lines().next().unwrap();
    let cols: Vec<&str> = header.split_whitespace().collect();
    assert_eq!(
        cols,
        vec!["id", "zebra", "alpha", "created_at", "updated_at"]
    );
}

#[test]
fn test_csv_columns_in_schema_order() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "zebra:text, alpha:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["items", "add", "--zebra", "hello", "--alpha", "42"])
        .assert()
        .success();
    let out = common::lodge_cmd(&dir)
        .args(["items", "query", "--format", "csv"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let header = stdout.lines().next().unwrap();
    let cols: Vec<&str> = header.split(',').collect();
    assert_eq!(
        cols,
        vec!["id", "zebra", "alpha", "created_at", "updated_at"]
    );
}
