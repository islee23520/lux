# Review Skill Routing

Use this category when the request asks to inspect quality, correctness, security, performance, or implementation risk.

## Route By Intent

- `code-review`: default LUX diff review for correctness, maintainability, tests, protocol consistency, and verification evidence.
- `studio-code-review`: structured game-studio file or small-change review with optional specialist subreviews.
- `security-audit`: use for credentials, command execution, network exposure, logs, generated code, or automation guardrails.
- `perf-profile`: use for latency, memory, CPU, file IO, startup, or responsiveness investigations.

## Do Not Use For

- Architecture ownership decisions. Use `../architecture/AGENTS.md`.
- Regression command selection after a known change. Use `../quality/regression-suite`.
