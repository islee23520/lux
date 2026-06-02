//! Shared I/O helpers for LUX - atomic writes and append-only JSONL event logs.
//! Enforces invariant #4 (Atomicity): all .lux/ writes use write-to-tmp + rename.

pub use lux_core::{append_jsonl, atomic_write_json, read_jsonl, write_evidence_file};
