# Task: Arguments Expansion for Custom Prompts

## Overview
Enable `$ARGUMENTS` (all args) and `$1`, `$2`, … (positional) placeholders in custom prompt content.

## Goals
- Parse `/command arg1 arg2 …` from the composer first line
- Expand placeholders inside the prompt content on execution
- Keep parsing simple (whitespace split); quote handling can be a follow-up

## Exact Files To Modify/Add

- Modify: `codex-rs/protocol/src/protocol.rs`
  - Add new op: `Op::RunCustomPrompt { path: PathBuf, args: Vec<String>, rest: String }`
  - Document semantics in comments
- Modify: `codex-rs/core/src/codex.rs`
  - Handle `Op::RunCustomPrompt`:
    - Load file content (and meta if present)
    - Expand `$ARGUMENTS` with `rest`
    - Expand `$1..$n` with `args[n-1]` or empty if missing
    - Submit as `Op::UserTurn { items: [InputItem::Text{ text: expanded_markdown }], … }`
- Modify: `codex-rs/core/src/custom_prompts.rs`
  - Add helper `fn expand_arguments(content: &str, args: &[String], rest: &str) -> String`
  - Unit tests for expansion (mixed placeholders, missing indices)
- Modify: `codex-rs/tui/src/bottom_pane/chat_composer.rs`
  - When Enter on a selected custom prompt, parse first line `/name …` into `{ args: Vec<String>, rest: String }`
  - Dispatch `Op::RunCustomPrompt { path, args, rest }` (resolve `path` from selected metadata)
- Modify: `codex-rs/tui/src/bottom_pane/command_popup.rs`
  - Store and surface `path` for selected prompt entries (via `CustomPromptMeta`)
- Modify: `codex-rs/tui/src/chatwidget.rs`
  - Adjust `on_list_custom_prompts` to store the new metadata set used by the composer

## Technical Plan

1) Protocol op
- Add `RunCustomPrompt` to the `Op` enum and derive serde

2) Core expansion
- Implement a fast, allocation-friendly expander:
  - Replace `$ARGUMENTS` tokens globally with `rest`
  - Replace `$n` tokens using a regex or on-the-fly scan; treat non-digits after `$` as a literal `$`

3) TUI parsing
- Extract the first line, strip the leading `/name`, and split remaining by whitespace:
  - `rest = remainder.trim().to_string()`
  - `args = remainder.split_whitespace().map(str::to_owned).collect()`

## Acceptance Criteria
- `$ARGUMENTS` expands to the full trailing text after the command name
- `$1..$n` expand to their positional tokens; missing indices → empty string
- Mixed usage works (e.g., "ID $1, all: $ARGUMENTS")
- No change to behavior for normal free-text messages

## Tests
- Core unit tests for `expand_arguments` covering:
  - Only `$ARGUMENTS`, only `$n`, and mixed
  - Missing positional indices
  - Multiple occurrences

## Risks / Notes
- Quotes/escaping are not supported in this milestone; content is split on whitespace
- Execution model (model override, frontmatter) deferred to separate tasks

---

References
- Protocol op: `codex-rs/protocol/src/protocol.rs`
- Core helpers: `codex-rs/core/src/custom_prompts.rs`
- TUI composer/popup: `codex-rs/tui/src/bottom_pane/chat_composer.rs`, `command_popup.rs`
