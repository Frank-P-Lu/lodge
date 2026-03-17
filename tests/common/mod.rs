use assert_cmd::Command;
use tempfile::TempDir;

pub fn lodge_cmd(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("lodge").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

/// Create a TempDir with `lodge init` already run.
pub fn setup() -> TempDir {
    let dir = TempDir::new().unwrap();
    lodge_cmd(&dir).args(["init"]).assert().success();
    dir
}
