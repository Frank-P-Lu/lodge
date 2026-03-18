mod common;

#[test]
fn schema_shows_distinct_values_for_low_cardinality_text() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "status:text, title:text"])
        .assert()
        .success();
    // 2 unique values across 12 rows = 16.7% ratio, below default 20% threshold
    for status in &[
        "open", "done", "open", "done", "open", "done", "open", "done", "open", "done", "open",
        "done",
    ] {
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
fn schema_hides_values_when_ratio_too_high() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "tag:text"])
        .assert()
        .success();
    // 7 unique values out of 7 rows = 100% ratio, way above default 20%
    for i in 0..7 {
        common::lodge_cmd(&dir)
            .args(["items", "add", "--tag", &format!("tag_{i}")])
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
        "Should not show values when ratio exceeds distinct_ratio"
    );
}

#[test]
fn schema_hides_values_above_max() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "tag:text"])
        .assert()
        .success();
    // Set distinct_max to 5, add 6 unique values across 200 rows (3% ratio, under 20%)
    common::lodge_cmd(&dir)
        .args(["set", "distinct_max", "5"])
        .assert()
        .success();
    for i in 0..6 {
        // Add each tag ~33 times to get a low ratio
        for _ in 0..33 {
            common::lodge_cmd(&dir)
                .args(["items", "add", "--tag", &format!("tag_{i}")])
                .assert()
                .success();
        }
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
        "Should not show values when count exceeds distinct_max even if ratio is low"
    );
}

#[test]
fn schema_shows_values_when_ratio_is_low() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "status:text"])
        .assert()
        .success();
    // 3 unique values across 30 rows = 10% ratio, well under 20%
    for _ in 0..10 {
        for val in &["open", "closed", "pending"] {
            common::lodge_cmd(&dir)
                .args(["items", "add", "--status", val])
                .assert()
                .success();
        }
    }
    let output = common::lodge_cmd(&dir)
        .args(["items", "schema"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let status_field = fields.iter().find(|f| f["name"] == "status").unwrap();
    let values = status_field["values"].as_array().unwrap();
    assert_eq!(values, &["closed", "open", "pending"]);
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
    // Add enough rows to get ratio below 20%: 1 unique status across 6 rows = 16.7%
    for _ in 0..6 {
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
    }
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
    // 2 unique values across 12 rows = 16.7% ratio
    for status in &[
        "open", "done", "open", "done", "open", "done", "open", "done", "open", "done", "open",
        "done",
    ] {
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
