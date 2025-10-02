use std::process::{Command, Stdio};
use std::time::Duration;
use std::io::Write;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Glyph") || stdout.contains("MCP"));
}

#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

#[test]
fn test_serve_stdio_starts() {
    let mut child = Command::new("cargo")
        .args(&["run", "--", "serve", "--transport", "stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    // Give it a moment to start
    std::thread::sleep(Duration::from_millis(500));

    // Send initialize request
    if let Some(mut stdin) = child.stdin.take() {
        let init_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
        stdin.write_all(init_request.as_bytes()).ok();
        stdin.write_all(b"\n").ok();
    }

    std::thread::sleep(Duration::from_millis(500));

    // Kill the process
    child.kill().expect("Failed to kill process");
    let output = child.wait_with_output().expect("Failed to wait for process");

    // Should have started without panic/error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panic"));
}

#[test]
fn test_serve_websocket_binds() {
    let mut child = Command::new("cargo")
        .args(&["run", "--", "serve", "--address", "127.0.0.1:17331"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    // Give it time to bind
    std::thread::sleep(Duration::from_secs(2));

    // Kill the process
    child.kill().expect("Failed to kill process");
    let output = child.wait_with_output().expect("Failed to wait for process");

    // Should have started and bound successfully
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panic"));
    assert!(!stderr.contains("Address already in use"));
}

#[test]
fn test_test_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "test"])
        .output()
        .expect("Failed to execute command");

    // Should complete without panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panic"));
}
