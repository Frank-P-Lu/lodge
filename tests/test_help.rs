mod common;

use predicates::prelude::*;

#[test]
fn help_shows_static_commands() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("sql"));
}

#[test]
fn help_shows_collections_after_create() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tasks"));
}

#[test]
fn collection_help_shows_subcommands() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["tasks", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("query"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn collection_add_help_shows_fields() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();

    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--priority"));
}
