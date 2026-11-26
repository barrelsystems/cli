use std::process::Command;

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_barrel"));
    command.env_remove("BARREL_CREDENTIAL");
    command
}

#[test]
fn login_preview_json_is_one_document_and_creates_no_session() {
    let output = command()
        .args(["login", "--output", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["preview"], true);
    assert_eq!(value["authenticated"], false);
}

#[test]
fn piped_login_is_honest_plain_text_without_terminal_controls() {
    let output = command().arg("login").output().unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("No browser was opened and no session was created"));
    assert!(!text.contains("Opening https://"));
    assert!(!text.contains('\u{1b}'));
    assert!(output.stderr.is_empty());
}

#[test]
fn quickstart_json_is_machine_readable() {
    let output = command().args(["--output", "json"]).output().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["name"], "barrel");
    assert!(output.stderr.is_empty());
}

#[test]
fn no_color_applies_to_help_even_with_a_forced_color_environment() {
    let output = command()
        .env("CLICOLOR_FORCE", "1")
        .args(["--no-color", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
}
