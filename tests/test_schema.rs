mod common;

#[test]
fn schema_returns_fields() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["tasks", "schema"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["collection"], "tasks");
    let fields = json["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0]["name"], "title");
    assert_eq!(fields[0]["type"], "text");
    assert_eq!(fields[1]["name"], "priority");
    assert_eq!(fields[1]["type"], "int");
}

#[test]
fn schema_after_alter() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["alter", "tasks", "--add-fields", "status:text"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["tasks", "schema"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[1]["name"], "status");
    assert_eq!(fields[1]["type"], "text");
}
