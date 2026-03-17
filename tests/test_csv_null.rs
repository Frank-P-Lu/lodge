mod common;

#[test]
fn test_csv_null_is_empty() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "name:text, count:int"])
        .assert()
        .success();
    // Add record with only name, count will be null
    common::lodge_cmd(&dir)
        .args(["items", "add", "--name", "widget"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["items", "query", "--format", "csv"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    // Header: id,name,count,created_at,updated_at
    // Data line should have empty field for null count, not "null"
    let data_line = lines[1];
    assert!(
        !data_line.contains("null"),
        "CSV should not contain literal 'null': {data_line}"
    );
    // Verify the field is empty (two consecutive commas around the null field)
    let fields: Vec<&str> = data_line.split(',').collect();
    // count is the 3rd field (index 2)
    assert_eq!(fields[2], "", "Null field should be empty string");
}

#[test]
fn test_json_null_stays_null() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "name:text, count:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["items", "add", "--name", "widget"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["items", "query", "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results[0]["count"].is_null());
}
