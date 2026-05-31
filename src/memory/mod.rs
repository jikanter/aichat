//! Phase 34A: Auto-memory read surface.
//!
//! Reads `memory/MEMORY.md` at startup and exposes it as a system-prompt
//! preamble that `Input::build_messages` injects after the active role's
//! prompt body. This module is **read-only** — the write loop (34C/34D
//! Reflector + Curator) is deferred pending a separate design review, see
//! `docs/roadmap/phase-34-overview.md`. Mirrors Claude Code's
//! first-~200-lines auto-load discipline (phase-34 §34A and open question 3).
//!
//! Discovery precedence (phase-34 open question 1): project-local `./memory/`
//! wins; the user-level `<config_dir>/memory/` is the fallback only when the
//! project-local directory carries no `MEMORY.md`. The two never merge — the
//! precedence is binary. The `AICHAT_MEMORY_DIR` env override short-circuits
//! both (used by the integration tests and by power users who keep memory
//! outside the default chain).

use std::path::PathBuf;

use crate::config::Config;
use crate::utils::get_env_name;

/// Index file at the root of a memory directory.
pub const MEMORY_INDEX: &str = "MEMORY.md";
/// Sub-directory name probed under the project root and the config dir.
pub const MEMORY_SUBDIR: &str = "memory";
/// Cap the preamble at 200 lines — Claude Code parity (phase-34 open Q3).
pub const MAX_PREAMBLE_LINES: usize = 200;
/// ...or 8 KiB, whichever hits first.
pub const MAX_PREAMBLE_BYTES: usize = 8 * 1024;
/// Header framing the injected block so the model reads it as standing
/// project context rather than task instructions.
pub const PREAMBLE_HEADER: &str = "# Project memory";

/// A loaded memory preamble plus provenance for `--info` and the cap warning.
#[derive(Debug, Clone)]
pub struct MemoryPreamble {
    /// Capped `MEMORY.md` content (no surrounding header).
    pub text: String,
    /// True when the 200-line / 8-KiB cap dropped content.
    pub truncated: bool,
    /// The `MEMORY.md` file the content was read from.
    pub source: PathBuf,
}

impl MemoryPreamble {
    /// The text injected into the system prompt, header included. Kept
    /// separate from [`text`](Self::text) so `--info` can show the raw memory
    /// without the synthetic header.
    pub fn as_system_block(&self) -> String {
        format!("{PREAMBLE_HEADER}\n{}", self.text)
    }
}

/// Resolve the active memory directory, honoring the env override first and
/// then the project-then-user precedence. Returns `None` when no `MEMORY.md`
/// is discoverable.
pub fn memory_dir() -> Option<PathBuf> {
    // Explicit override wins unconditionally — no fallback if it lacks the
    // index, so tests and power users get a predictable single source.
    if let Ok(dir) = std::env::var(get_env_name("memory_dir")) {
        let dir = PathBuf::from(dir);
        return dir.join(MEMORY_INDEX).is_file().then_some(dir);
    }
    // Project-local first: `$CWD/memory/` (phase-34 open Q1, tenet 5).
    if let Ok(cwd) = std::env::current_dir() {
        let dir = cwd.join(MEMORY_SUBDIR);
        if dir.join(MEMORY_INDEX).is_file() {
            return Some(dir);
        }
    }
    // User-level fallback: `<config_dir>/memory/`.
    let dir = Config::config_dir().join(MEMORY_SUBDIR);
    dir.join(MEMORY_INDEX).is_file().then_some(dir)
}

/// Apply the 200-line / 8-KiB cap to raw `MEMORY.md` content. Returns the
/// capped string and whether anything was dropped. Caps on a line boundary
/// first; if still over the byte budget, drops whole trailing lines; a single
/// over-budget line is hard-truncated on a UTF-8 char boundary so the output
/// is never invalid UTF-8.
pub fn cap_preamble(raw: &str) -> (String, bool) {
    let mut truncated = false;
    let lines: Vec<&str> = raw.lines().collect();
    let mut kept: Vec<&str> = if lines.len() > MAX_PREAMBLE_LINES {
        truncated = true;
        lines[..MAX_PREAMBLE_LINES].to_vec()
    } else {
        lines
    };
    let mut joined = kept.join("\n");
    // Byte cap: drop whole trailing lines until within budget.
    while joined.len() > MAX_PREAMBLE_BYTES && kept.len() > 1 {
        kept.pop();
        truncated = true;
        joined = kept.join("\n");
    }
    // Edge: a lone line longer than the byte budget — hard-truncate on a char
    // boundary.
    if joined.len() > MAX_PREAMBLE_BYTES {
        let mut end = MAX_PREAMBLE_BYTES;
        while end > 0 && !joined.is_char_boundary(end) {
            end -= 1;
        }
        joined.truncate(end);
        truncated = true;
    }
    (joined, truncated)
}

/// Read and cap `memory/MEMORY.md`. `None` when no memory file is
/// discoverable or it is empty after trimming. Pure read; never writes.
pub fn load_preamble() -> Option<MemoryPreamble> {
    let dir = memory_dir()?;
    let source = dir.join(MEMORY_INDEX);
    let raw = std::fs::read_to_string(&source).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    let (text, truncated) = cap_preamble(&raw);
    if text.trim().is_empty() {
        return None;
    }
    Some(MemoryPreamble {
        text,
        truncated,
        source,
    })
}

/// Inject a memory block into the system message of an already-assembled
/// message list, appended after any existing system text. If no system
/// message exists (e.g. a bare prompt), prepend one carrying the block.
/// Idempotent at the call site: `Input::build_messages` rebuilds the list
/// every turn, so this never double-appends within a conversation.
pub fn inject_preamble(messages: &mut Vec<crate::client::Message>, block: &str) {
    use crate::client::{Message, MessageContent, MessageRole};
    for msg in messages.iter_mut() {
        if msg.role == MessageRole::System {
            if let MessageContent::Text(ref mut text) = msg.content {
                text.push_str("\n\n");
                text.push_str(block);
                return;
            }
        }
    }
    messages.insert(
        0,
        Message::new(MessageRole::System, MessageContent::Text(block.to_string())),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{Message, MessageContent, MessageRole};

    #[test]
    fn cap_under_limit_is_untouched() {
        let raw = "- one\n- two\n- three";
        let (text, truncated) = cap_preamble(raw);
        assert_eq!(text, raw);
        assert!(!truncated);
    }

    #[test]
    fn cap_drops_lines_past_200() {
        let raw: String = (1..=250)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (text, truncated) = cap_preamble(&raw);
        assert!(truncated);
        assert_eq!(text.lines().count(), MAX_PREAMBLE_LINES);
        assert!(text.contains("line 200"));
        assert!(!text.contains("line 201"));
    }

    #[test]
    fn cap_honors_byte_budget_before_line_budget() {
        // 50 lines, each ~300 bytes => ~15 KiB, well under 200 lines but over
        // the 8-KiB byte cap. Expect a byte-driven truncation.
        let long = "x".repeat(300);
        let raw: String = (0..50)
            .map(|_| long.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let (text, truncated) = cap_preamble(&raw);
        assert!(truncated);
        assert!(text.len() <= MAX_PREAMBLE_BYTES);
        assert!(text.lines().count() < 50);
    }

    #[test]
    fn cap_hard_truncates_single_oversized_line() {
        let raw = "y".repeat(MAX_PREAMBLE_BYTES + 500);
        let (text, truncated) = cap_preamble(&raw);
        assert!(truncated);
        assert_eq!(text.len(), MAX_PREAMBLE_BYTES);
    }

    #[test]
    fn cap_never_splits_utf8() {
        // Fill just past the byte budget with multi-byte chars on one line.
        let raw = "é".repeat(MAX_PREAMBLE_BYTES); // 2 bytes each
        let (text, _truncated) = cap_preamble(&raw);
        // The truncation point must land on a char boundary — `String::truncate`
        // would have panicked otherwise, but assert the content stays valid.
        assert!(text.chars().all(|c| c == 'é'));
        assert!(text.len() <= MAX_PREAMBLE_BYTES);
    }

    #[test]
    fn inject_appends_to_existing_system_message() {
        let mut messages = vec![
            Message::new(MessageRole::System, MessageContent::Text("Role body.".into())),
            Message::new(MessageRole::User, MessageContent::Text("hi".into())),
        ];
        inject_preamble(&mut messages, "# Project memory\n- remember this");
        match &messages[0].content {
            MessageContent::Text(t) => {
                assert!(t.starts_with("Role body."));
                assert!(t.contains("# Project memory"));
                assert!(t.contains("remember this"));
            }
            _ => panic!("system message should be text"),
        }
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn inject_prepends_when_no_system_message() {
        let mut messages = vec![Message::new(
            MessageRole::User,
            MessageContent::Text("hi".into()),
        )];
        inject_preamble(&mut messages, "# Project memory\n- standing note");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::System);
        match &messages[0].content {
            MessageContent::Text(t) => assert!(t.contains("standing note")),
            _ => panic!("inserted system message should be text"),
        }
    }
}
