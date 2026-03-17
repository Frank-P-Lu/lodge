mod common;

#[test]
fn test_add_bool_returns_true() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "flags", "--fields", "active:bool, name:text"])
        .assert()
        .success();
    let out = common::lodge_cmd(&dir)
        .args(["flags", "add", "--active", "true", "--name", "test"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    assert_eq!(json["active"], serde_json::Value::Bool(true));
}

#[test]
fn test_add_bool_returns_false() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "flags", "--fields", "active:bool, name:text"])
        .assert()
        .success();
    let out = common::lodge_cmd(&dir)
        .args(["flags", "add", "--active", "false", "--name", "test"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    assert_eq!(json["active"], serde_json::Value::Bool(false));
}

#[test]
fn test_query_bool_fields_are_booleans() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "flags", "--fields", "active:bool"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["flags", "add", "--active", "true"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["flags", "query"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results[0]["active"], serde_json::Value::Bool(true));
}

#[test]
fn test_bool_null_stays_null() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "flags", "--fields", "active:bool, name:text"])
        .assert()
        .success();
    let out = common::lodge_cmd(&dir)
        .args(["flags", "add", "--name", "no-active"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = common::parse_json_from_output(&out.stdout);
    assert!(json["active"].is_null());
}
