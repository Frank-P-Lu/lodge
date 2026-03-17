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

/// Parse JSON from stdout that may have a confirmation message line before the JSON.
pub fn parse_json_from_output(stdout: &[u8]) -> serde_json::Value {
    let text = std::str::from_utf8(stdout).unwrap();
    let start = text
        .find(|c| c == '{' || c == '[')
        .expect("No JSON in output");
    serde_json::from_str(&text[start..]).unwrap()
}
