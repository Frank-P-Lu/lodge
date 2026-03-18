mod common;

use serde_json::Value;

/// CSV export represents nulls as empty strings; importing that CSV
/// back should preserve them as null (not as the literal string "null").
#[test]
fn csv_null_roundtrip_preserves_nulls() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "name:text, note:text"])
        .assert()
        .success();

    // Add a record with the optional `note` field omitted → stored as NULL
    common::lodge_cmd(&dir)
        .args(["items", "add", "--name", "alpha"])
        .assert()
        .success();

    // Export as CSV
    let csv_out = common::lodge_cmd(&dir)
        .args(["export", "items", "--format", "csv"])
        .output()
        .unwrap();
    let csv_text = std::str::from_utf8(&csv_out.stdout).unwrap().trim();

    // Write CSV to a file, then import into a fresh collection
    let csv_path = dir.path().join("items.csv");
    std::fs::write(&csv_path, csv_text).unwrap();

    common::lodge_cmd(&dir)
        .args(["create", "items2", "--fields", "name:text, note:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["import", "items2", "--file", csv_path.to_str().unwrap()])
        .assert()
        .success();

    // Query the re-imported collection as JSON — `note` should be null, not ""
    let output = common::lodge_cmd(&dir)
        .args(["items2", "query"])
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "alpha");
    assert!(
        arr[0]["note"].is_null(),
        "note should be null after CSV round-trip, got: {}",
        arr[0]["note"]
    );
}

/// The literal string "null" in a text field must survive a CSV round-trip
/// without being converted to actual NULL.
#[test]
fn csv_roundtrip_preserves_literal_null_string() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "name:text, note:text"])
        .assert()
        .success();

    // Add a record where note is literally the string "null"
    common::lodge_cmd(&dir)
        .args(["items", "add", "--name", "beta", "--note", "null"])
        .assert()
        .success();

    // Export as CSV
    let csv_out = common::lodge_cmd(&dir)
        .args(["export", "items", "--format", "csv"])
        .output()
        .unwrap();
    let csv_text = std::str::from_utf8(&csv_out.stdout).unwrap().trim();

    // Write CSV to a file, then import into a fresh collection
    let csv_path = dir.path().join("items.csv");
    std::fs::write(&csv_path, csv_text).unwrap();

    common::lodge_cmd(&dir)
        .args(["create", "items2", "--fields", "name:text, note:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["import", "items2", "--file", csv_path.to_str().unwrap()])
        .assert()
        .success();

    // The literal string "null" should be preserved, not become JSON null
    let output = common::lodge_cmd(&dir)
        .args(["items2", "query"])
        .output()
        .unwrap();
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0]["note"], "null",
        "literal string 'null' should survive CSV round-trip"
    );
}
