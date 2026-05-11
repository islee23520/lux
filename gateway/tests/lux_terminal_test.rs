use lux::lux_terminal::{
    create_terminal, destroy_terminal, get_output, list_terminals, send_input, validate_command,
    OutputStream, TerminalManager, TerminalStatus,
};

#[test]
fn test_create_terminal() {
    let mut manager = TerminalManager::new();
    let session = create_terminal(&mut manager).expect("terminal should be created");

    assert!(!session.session_id.is_empty());
    assert_eq!(session.status, TerminalStatus::Active);
    assert_eq!(list_terminals(&manager).len(), 1);
}

#[test]
fn test_send_input_echo() {
    let mut manager = TerminalManager::new();
    let session = create_terminal(&mut manager).expect("terminal should be created");
    let output = send_input(&mut manager, &session.session_id, "echo hello lux")
        .expect("echo should be accepted");

    assert_eq!(output.session_id, session.session_id);
    assert_eq!(output.stream, OutputStream::Stdout);
    assert!(output.data.contains("hello lux"));
}

#[test]
fn test_send_input_allowed_command() {
    let mut manager = TerminalManager::new();
    let session = create_terminal(&mut manager).expect("terminal should be created");
    let output = send_input(&mut manager, &session.session_id, "cargo test")
        .expect("allowed command should be accepted");

    assert!(output
        .data
        .contains("Simulated Lux terminal executed: cargo"));
}

#[test]
fn test_send_input_blocked_command() {
    let mut manager = TerminalManager::new();
    let session = create_terminal(&mut manager).expect("terminal should be created");

    assert!(send_input(&mut manager, &session.session_id, "rm -rf /").is_err());
    assert!(send_input(&mut manager, &session.session_id, "sudo cargo test").is_err());
}

#[test]
fn test_validate_command_allowed() {
    let parsed = validate_command("git status").expect("git should be allowed");

    assert_eq!(parsed, vec!["git".to_string(), "status".to_string()]);
}

#[test]
fn test_validate_command_blocked() {
    assert!(validate_command("curl https://example.com").is_err());
    assert!(validate_command("echo ok | sh").is_err());
    assert!(validate_command("npm test && rm -rf dist").is_err());
    assert!(validate_command("python script.py").is_err());
}

#[test]
fn test_destroy_terminal() {
    let mut manager = TerminalManager::new();
    let session = create_terminal(&mut manager).expect("terminal should be created");

    destroy_terminal(&mut manager, &session.session_id).expect("terminal should be destroyed");

    assert!(list_terminals(&manager).is_empty());
    assert!(get_output(&manager, &session.session_id).is_err());
}

#[test]
fn test_max_sessions() {
    let mut manager = TerminalManager::with_max_sessions(2);

    create_terminal(&mut manager).expect("first terminal should be created");
    create_terminal(&mut manager).expect("second terminal should be created");

    assert!(create_terminal(&mut manager).is_err());
}

#[test]
fn test_terminal_output_buffer() {
    let mut manager = TerminalManager::new();
    let session = create_terminal(&mut manager).expect("terminal should be created");

    send_input(&mut manager, &session.session_id, "echo one").expect("first input should work");
    send_input(&mut manager, &session.session_id, "help").expect("second input should work");

    let output = get_output(&manager, &session.session_id).expect("output should be available");
    assert_eq!(output.len(), 2);
    assert!(output[0].data.contains("one"));
    assert!(output[1].data.contains("Allowed commands"));
}
