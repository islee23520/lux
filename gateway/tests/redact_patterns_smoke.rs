//! LUX Phase 7 LINA-3: Regex-based secret pattern detection tests.
//!
//! Extends the existing Bearer/token= redaction with three new regex patterns:
//!   1. GitHub PAT: ghp_[A-Za-z0-9]{36,}
//!   2. AWS Access Key ID: AKIA[0-9A-Z]{16}
//!   3. OpenAI API Key: sk-(openai|project)-[A-Za-z0-9]{20,}
//!
//! Each pattern has positive (redacted) and false-positive (preserved) tests.

mod common;

use lux::ai_log::redact_secrets;

// ===========================================================================
// GitHub PAT — ghp_[A-Za-z0-9]{36,}
// ===========================================================================

#[test]
fn github_pat_redacted() {
    // ghp_ + 40 alphanumeric chars (exceeds 36-char minimum)
    let input = "token ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890aBcDeF end";
    let out = redact_secrets(input);
    assert!(
        out.contains("[REDACTED]"),
        "GitHub PAT must be redacted, got: {out}"
    );
    assert!(
        !out.contains("ghp_"),
        "ghp_ prefix must not survive in output"
    );
}

#[test]
fn github_pat_redacted_exact_length() {
    // Exactly 36 chars after ghp_ (minimum matching length)
    let input = "ghp_abcdefghijklmnopqrstuvwxyz1234567890";
    let out = redact_secrets(input);
    assert_eq!(
        out, "[REDACTED]",
        "exact-length GitHub PAT must be fully redacted"
    );
}

#[test]
fn github_pat_redacted_longer_than_min() {
    // 42 chars after ghp_ (exceeds minimum)
    let input = "ghp_abcdefghijklmnopqrstuvwxyz1234567890abcdef";
    let out = redact_secrets(input);
    assert_eq!(
        out, "[REDACTED]",
        "longer GitHub PAT must be fully redacted"
    );
}

#[test]
fn github_pat_false_positive_short_token() {
    // Only 10 chars after ghp_ — too short to match
    let input = "ghp_shortToken";
    let out = redact_secrets(input);
    assert_eq!(out, input, "short ghp_ token should NOT be redacted");
}

#[test]
fn github_pat_false_positive_partial_prefix() {
    // "ghp" alone without underscore and sufficient length
    let inputs = ["ghp", "ghp_", "ghp_onlyshort"];
    for input in &inputs {
        let out = redact_secrets(input);
        assert_eq!(
            out, *input,
            "'{input}' should not be redacted as GitHub PAT"
        );
    }
}

// ===========================================================================
// AWS Access Key ID — AKIA[0-9A-Z]{16}
// ===========================================================================

#[test]
fn aws_key_redacted() {
    // AKIA + 16 alphanumeric chars (exact match for pattern)
    let input = "aws_key AKIAIOSFODNN7EXAMPLE end";
    let out = redact_secrets(input);
    assert!(
        out.contains("[REDACTED]"),
        "AWS key must be redacted, got: {out}"
    );
    assert!(
        !out.contains("AKIA"),
        "AKIA prefix must not survive in output"
    );
}

#[test]
fn aws_key_redacted_exact_format() {
    // AKIA + exactly 16 alphanumeric chars
    let input = "AKIA1234567890ABCDEF";
    let out = redact_secrets(input);
    assert_eq!(
        out, "[REDACTED]",
        "exact-format AWS key must be fully redacted"
    );
}

#[test]
fn aws_key_false_positive_too_short() {
    // AKIA + only 3 chars — too short
    let input = "AKIA123";
    let out = redact_secrets(input);
    assert_eq!(out, input, "short AKIA prefix should NOT be redacted");
}

#[test]
fn aws_key_false_positive_lowercase() {
    // Lowercase akia does not match the uppercase-only pattern
    let input = "akia1234567890abcdef";
    let out = redact_secrets(input);
    assert_eq!(out, input, "lowercase 'akia' should NOT be redacted");
}

#[test]
fn aws_key_false_positive_with_special_chars() {
    // Special chars in the key portion break the [0-9A-Z] character class
    let input = "AKIA1234-5678-90AB-CDEF";
    let out = redact_secrets(input);
    assert!(
        out.contains("AKIA"),
        "AWS key with hyphens should not match pure-alphanumeric pattern"
    );
}

// ===========================================================================
// OpenAI API Key — sk-(openai|project)-[A-Za-z0-9]{20,}
// ===========================================================================

#[test]
fn openai_key_sk_openai_redacted() {
    let input = "api sk-openai-abcdefghijklmnopqrstuvwxyz12 end";
    let out = redact_secrets(input);
    assert!(
        out.contains("[REDACTED]"),
        "sk-openai key must be redacted, got: {out}"
    );
    assert!(
        !out.contains("sk-openai-"),
        "sk-openai- prefix must not survive"
    );
}

#[test]
fn openai_key_sk_project_redacted() {
    let input = "key sk-project-abcdefghijklmnopqrstuvwxyz12 done";
    let out = redact_secrets(input);
    assert!(
        out.contains("[REDACTED]"),
        "sk-project key must be redacted, got: {out}"
    );
    assert!(
        !out.contains("sk-project-"),
        "sk-project- prefix must not survive"
    );
}

#[test]
fn openai_key_exact_minimum_length() {
    // sk-openai- + exactly 20 chars
    let input = "sk-openai-abcdefghijklmnopqrst";
    let out = redact_secrets(input);
    assert_eq!(
        out, "[REDACTED]",
        "minimum-length OpenAI key must be redacted"
    );
}

#[test]
fn openai_key_false_positive_too_short() {
    // sk-openai- + only 5 chars — too short
    let input = "sk-openai-abcde";
    let out = redact_secrets(input);
    assert_eq!(out, input, "short sk-openai key should NOT be redacted");
}

#[test]
fn openai_key_false_positive_unknown_variant() {
    // sk-something- is not a recognized variant
    let input = "sk-custom-abcdefghijklmnopqrst";
    let out = redact_secrets(input);
    assert_eq!(out, input, "unknown sk-* variant should NOT be redacted");
}

#[test]
fn openai_key_false_positive_plain_sk() {
    // Legacy sk- keys without openai/project qualifier
    let input = "sk-abcdefghijklmnopqrstuvwx"; // 24 chars but no qualifier
    let out = redact_secrets(input);
    assert_eq!(
        out, input,
        "plain sk- key without qualifier should NOT be redacted"
    );
}

// ===========================================================================
// Mixed content — multiple secret types in one string
// ===========================================================================

#[test]
fn mixed_content_secrets_all_redacted() {
    let input =
        "Bearer token ghp_abcdefghijklmnopqrstuvwxyz1234567890abcdef AKIA1234567890ABCDEF sk-openai-abcdefghijklmnopqrstuv normal text";
    let out = redact_secrets(input);

    // All four secret patterns must be redacted
    let redacted_count = out.matches("[REDACTED]").count();
    assert!(
        redacted_count >= 4,
        "expected at least 4 [REDACTED] markers, got {redacted_count}: {out}"
    );
    // Non-secret text must survive
    assert!(out.contains("normal text"), "non-secret text must survive");
}

#[test]
fn mixed_content_secrets_in_middle_of_sentence() {
    let input = "Deploying with key AKIAIOSFODNN7EXAMPLE to production server now";
    let out = redact_secrets(input);

    assert!(
        out.contains("[REDACTED]"),
        "embedded AWS key must be redacted"
    );
    assert!(
        out.contains("Deploying with key"),
        "prefix before secret must survive"
    );
    assert!(
        out.contains("to production server now"),
        "suffix after secret must survive"
    );
}

#[test]
fn mixed_content_multiple_github_pats() {
    let input =
        "First: ghp_abcdefghijklmnopqrstuvwxyz1234567890abcdef Second: ghp_0123456789abcdefghijklmnopqrstuvwxyz012345 End";
    let out = redact_secrets(input);

    let redacted_count = out.matches("[REDACTED]").count();
    assert_eq!(
        redacted_count, 2,
        "both GitHub PATs must be individually redacted, got: {out}"
    );
}

// ===========================================================================
// Backward compatibility — existing Bearer/token= behavior preserved
// ===========================================================================

#[test]
fn backward_compat_bearer_still_works() {
    let input = "Authorization: Bearer mysecretval";
    let out = redact_secrets(input);
    assert!(out.contains("[REDACTED]"), "Bearer redaction still works");
}

#[test]
fn backward_compat_token_equals_still_works() {
    let input = "config token=super_secret_value here";
    let out = redact_secrets(input);
    assert!(out.contains("[REDACTED]"), "token= redaction still works");
    assert!(!out.contains("super_secret_value"), "token value leaked");
}

#[test]
fn backward_compat_no_secrets_unchanged() {
    let input = "normal log message with no secrets whatsoever";
    assert_eq!(redact_secrets(input), input);
}
