mod common;

use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn init_creates_lodge_dir_and_db() {
    let dir = TempDir::new().unwrap();
    common::lodge_cmd(&dir)
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized lodge database"));

    assert!(dir.path().join(".lodge").exists());
    assert!(dir.path().join(".lodge/lodge.db").exists());
}

#[test]
fn init_twice_gives_error() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already initialized"));
}

#[test]
fn init_creates_meta_table() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["sql", "SELECT name FROM sqlite_master WHERE type='table' AND name='_lodge_meta'"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_lodge_meta"));
}
