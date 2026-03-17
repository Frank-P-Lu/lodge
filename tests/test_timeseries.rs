mod common;

use predicates::prelude::*;

fn setup_with_gym() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args([
            "create",
            "gym",
            "--fields",
            "date:date, weight:real, notes:text",
        ])
        .assert()
        .success();
    dir
}

fn add_gym_entry(dir: &tempfile::TempDir, date: &str, weight: &str) {
    common::lodge_cmd(dir)
        .args(["gym", "add", "--date", date, "--weight", weight])
        .assert()
        .success();
}

#[test]
fn streak_with_consecutive_days() {
    let dir = setup_with_gym();
    add_gym_entry(&dir, "2026-03-15", "80.0");
    add_gym_entry(&dir, "2026-03-16", "79.5");
    add_gym_entry(&dir, "2026-03-17", "79.8");

    let out = common::lodge_cmd(&dir)
        .args(["gym", "streak", "--field", "date"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["current_streak"], 3);
    assert_eq!(result["longest_streak"], 3);
    assert_eq!(result["total_days_with_records"], 3);
}

#[test]
fn streak_with_gaps_current_vs_longest_differ() {
    let dir = setup_with_gym();
    // Longest streak: 3 days
    add_gym_entry(&dir, "2026-03-01", "80.0");
    add_gym_entry(&dir, "2026-03-02", "79.5");
    add_gym_entry(&dir, "2026-03-03", "79.8");
    // Gap
    // Current streak: 2 days
    add_gym_entry(&dir, "2026-03-10", "78.0");
    add_gym_entry(&dir, "2026-03-11", "77.5");

    let out = common::lodge_cmd(&dir)
        .args(["gym", "streak", "--field", "date"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["current_streak"], 2);
    assert_eq!(result["longest_streak"], 3);
    assert_eq!(result["total_days_with_records"], 5);
}

#[test]
fn streak_on_empty_collection() {
    let dir = setup_with_gym();
    let out = common::lodge_cmd(&dir)
        .args(["gym", "streak", "--field", "date"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["current_streak"], 0);
    assert_eq!(result["longest_streak"], 0);
    assert_eq!(result["total_days_with_records"], 0);
}

#[test]
fn streak_on_wrong_field_type_errors() {
    let dir = setup_with_gym();
    common::lodge_cmd(&dir)
        .args(["gym", "streak", "--field", "weight"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("wrong type"));
}

#[test]
fn gaps_finds_gaps_above_threshold() {
    let dir = setup_with_gym();
    add_gym_entry(&dir, "2026-03-01", "80.0");
    add_gym_entry(&dir, "2026-03-02", "79.5");
    // 5-day gap
    add_gym_entry(&dir, "2026-03-07", "79.0");
    add_gym_entry(&dir, "2026-03-08", "78.5");

    let out = common::lodge_cmd(&dir)
        .args(["gym", "gaps", "--field", "date", "--threshold", "2"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["gap_start"], "2026-03-02");
    assert_eq!(results[0]["gap_end"], "2026-03-07");
    assert_eq!(results[0]["days"], 5);
}

#[test]
fn gaps_with_high_threshold_filters_small_gaps() {
    let dir = setup_with_gym();
    add_gym_entry(&dir, "2026-03-01", "80.0");
    add_gym_entry(&dir, "2026-03-04", "79.5"); // 3-day gap

    let out = common::lodge_cmd(&dir)
        .args(["gym", "gaps", "--field", "date", "--threshold", "5"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());
}

#[test]
fn gaps_with_no_gaps_returns_empty() {
    let dir = setup_with_gym();
    add_gym_entry(&dir, "2026-03-01", "80.0");
    add_gym_entry(&dir, "2026-03-02", "79.5");
    add_gym_entry(&dir, "2026-03-03", "79.0");

    let out = common::lodge_cmd(&dir)
        .args(["gym", "gaps", "--field", "date"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(results.is_empty());
}

#[test]
fn rolling_average_computes_correctly() {
    let dir = setup_with_gym();
    add_gym_entry(&dir, "2026-03-01", "80.0");
    add_gym_entry(&dir, "2026-03-02", "82.0");
    add_gym_entry(&dir, "2026-03-03", "81.0");
    add_gym_entry(&dir, "2026-03-04", "83.0");

    let out = common::lodge_cmd(&dir)
        .args([
            "gym",
            "rolling-avg",
            "--field",
            "weight",
            "--over",
            "date",
            "--window",
            "3",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let results: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 4);

    // First entry: avg of just itself = 80.0
    assert_eq!(results[0]["value"], 80.0);
    assert_eq!(results[0]["rolling_avg"], 80.0);

    // Second entry: avg of (80, 82) = 81.0
    assert_eq!(results[1]["rolling_avg"], 81.0);

    // Third entry: avg of (80, 82, 81) = 81.0
    assert_eq!(results[2]["rolling_avg"], 81.0);

    // Fourth entry: avg of (82, 81, 83) = 82.0
    assert_eq!(results[3]["rolling_avg"], 82.0);
}

#[test]
#[test]
fn test_streak_reports_skipped_nulls() {
    let dir = setup_with_gym();
    // Add records with dates
    add_gym_entry(&dir, "2026-03-15", "80.0");
    add_gym_entry(&dir, "2026-03-16", "79.5");
    // Add records with null dates (only weight, no date)
    common::lodge_cmd(&dir)
        .args(["gym", "add", "--weight", "81.0"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["gym", "add", "--weight", "82.0"])
        .assert()
        .success();

    let out = common::lodge_cmd(&dir)
        .args(["gym", "streak", "--field", "date"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["total_days_with_records"], 2);
    assert_eq!(result["skipped_nulls"], 2);
}

#[test]
fn rolling_average_on_wrong_field_type_errors() {
    let dir = setup_with_gym();
    common::lodge_cmd(&dir)
        .args([
            "gym",
            "rolling-avg",
            "--field",
            "notes",
            "--over",
            "date",
            "--window",
            "3",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("wrong type"));
}
