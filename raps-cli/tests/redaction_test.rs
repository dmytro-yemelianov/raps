use raps_kernel::logging;

#[test]
fn test_redaction_logic() {
    let input = "Bearer: abcdefghijklmnopqrstuvwxyz1234567890";
    let output = logging::redact_secrets(input);
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("1234567890"));
}
