mod common;

use predicates::prelude::*;

#[test]
fn guide_prints_decision_framework() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["guide"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Lodge is for data you ACT on"))
        .stdout(predicate::str::contains("Markdown is for context you READ"));
}

#[test]
fn guide_includes_litmus_test() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["guide"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Am I reading this whole file just to find one thing"));
}

#[test]
fn help_mentions_guide_command() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("guide"));
}
