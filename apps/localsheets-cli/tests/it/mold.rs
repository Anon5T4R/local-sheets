//! The suite's agent-native mould in Local Sheets: `run --script` accepting a JSON array, and
//! `mcp --connect` taking a bare loopback port while refusing any other host.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_localsheets-cli")).args(args).output().expect("run localsheets-cli")
}

fn tmpdir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("localsheets-cli-mould-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_script_may_be_a_json_array() {
    let dir = tmpdir("array");
    let script = dir.join("steps.json");
    // One object step and one string step: both forms the line parser takes.
    std::fs::write(&script, r#"[{"command":"cell.set","params":{"cell":"A1","input":"Hello"}},"cell.set {\"cell\":\"B1\",\"input\":\"World\"}"]"#)
        .unwrap();
    let out = run(&["run", "--script", script.to_str().unwrap(), "--print", "A1:B1", "--csv"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let printed = String::from_utf8_lossy(&out.stdout);
    assert!(printed.contains("Hello") && printed.contains("World"), "{printed}");
}

#[test]
fn an_array_names_a_bad_step_by_its_index() {
    let dir = tmpdir("bad");
    let script = dir.join("steps.json");
    std::fs::write(&script, r#"["file.new", {"command": "cell.set", "args": {}}]"#).unwrap();
    let out = run(&["run", "--script", script.to_str().unwrap()]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("[1]"), "{stderr}");
}

#[test]
fn connect_takes_a_bare_port_and_refuses_a_public_host() {
    // A bare port means loopback: the failure names 127.0.0.1, not the bare port.
    let out = Command::new(env!("CARGO_BIN_EXE_localsheets-cli")).args(["mcp", "--connect", "1"]).stdin(Stdio::null()).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("127.0.0.1:1"), "{}", String::from_utf8_lossy(&out.stderr));
    // A host that isn't loopback never receives the control token, so it is refused outright.
    let out =
        Command::new(env!("CARGO_BIN_EXE_localsheets-cli")).args(["mcp", "--connect", "192.168.1.5:7979"]).stdin(Stdio::null()).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("loopback only"));
}
