mod common;

#[test]
fn schema_shows_distinct_values_for_low_cardinality_text() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "status:text, title:text"])
        .assert()
        .success();
    for status in &["open", "done", "open", "done", "open"] {
        common::lodge_cmd(&dir)
            .args(["tasks", "add", "--status", status, "--title", "something"])
            .assert()
            .success();
    }
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "schema"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let status_field = fields.iter().find(|f| f["name"] == "status").unwrap();
    let values = status_field["values"].as_array().unwrap();
    assert_eq!(values, &["done", "open"]);
}

#[test]
fn schema_hides_values_above_threshold() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "tag:text"])
        .assert()
        .success();
    // Add 16 distinct values (default threshold is 15)
    for i in 0..16 {
        common::lodge_cmd(&dir)
            .args(["items", "add", "--tag", &format!("tag_{i:02}")])
            .assert()
            .success();
    }
    let output = common::lodge_cmd(&dir)
        .args(["items", "schema"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let tag_field = fields.iter().find(|f| f["name"] == "tag").unwrap();
    assert!(
        tag_field.get("values").is_none(),
        "Should not show values above threshold"
    );
}

#[test]
fn schema_omits_values_for_empty_collection() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "status:text"])
        .assert()
        .success();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "schema"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let status_field = fields.iter().find(|f| f["name"] == "status").unwrap();
    assert!(
        status_field.get("values").is_none(),
        "Empty collection should have no values"
    );
}

#[test]
fn schema_only_shows_values_for_text_fields() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "tasks",
            "--fields",
            "priority:int, done:bool, status:text",
        ])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args([
            "tasks",
            "add",
            "--priority",
            "1",
            "--done",
            "true",
            "--status",
            "open",
        ])
        .assert()
        .success();
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "schema"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let priority_field = fields.iter().find(|f| f["name"] == "priority").unwrap();
    assert!(
        priority_field.get("values").is_none(),
        "int field should not have values"
    );
    let done_field = fields.iter().find(|f| f["name"] == "done").unwrap();
    assert!(
        done_field.get("values").is_none(),
        "bool field should not have values"
    );
    let status_field = fields.iter().find(|f| f["name"] == "status").unwrap();
    assert!(status_field["values"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("open")));
}

#[test]
fn list_shows_distinct_values() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "status:text"])
        .assert()
        .success();
    for status in &["open", "done"] {
        common::lodge_cmd(&dir)
            .args(["tasks", "add", "--status", status])
            .assert()
            .success();
    }
    let output = common::lodge_cmd(&dir).args(["list"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    let tasks = arr.iter().find(|c| c["name"] == "tasks").unwrap();
    let fields = tasks["fields"].as_array().unwrap();
    let status_field = fields.iter().find(|f| f["name"] == "status").unwrap();
    let values = status_field["values"].as_array().unwrap();
    assert_eq!(values, &["done", "open"]);
}
