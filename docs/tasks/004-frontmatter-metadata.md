# Task: Frontmatter Metadata for Custom Prompts

## Overview
Support YAML frontmatter in prompt files to provide metadata:

- `description`: short text for UI listing (fallback: first non-empty content line)
- `argument-hint`: hint shown in autocomplete/help for arguments
- `allowed-tools`: (parsed now, enforced in a later milestone)
- `model`: optional per-command model override (parsed now, honored in a later milestone)

## Goals
- Parse frontmatter blocks at the top of markdown files
- Populate metadata fields in the prompt list response
- Show `description` and `argument-hint` in the TUI popup

## Exact Files To Modify/Add

- Modify: `codex-rs/core/Cargo.toml`
  - Add dependency: `serde_yaml = "0.9"` (or compatible)
- Modify: `codex-rs/protocol/src/custom_prompts.rs`
  - Ensure `CustomPromptMeta` includes `description: Option<String>`, `argument_hint: Option<String>`, `model: Option<String>`
- Modify: `codex-rs/core/src/custom_prompts.rs`
  - Implement `fn parse_frontmatter(text: &str) -> (Option<Frontmatter>, &str)`
  - Define `Frontmatter { description: Option<String>, argument_hint: Option<String>, model: Option<String>, allowed_tools: Option<String> }`
  - Populate meta during discovery and strip the frontmatter from `content` returned to the model later
- Modify: `codex-rs/core/src/codex.rs`
  - When sending `ListCustomPromptsResponse`, include parsed metadata
- Modify: `codex-rs/tui/src/bottom_pane/command_popup.rs`
  - Render description and (if present) argument-hint (dim) after the command name and scope tag

## Technical Plan

1) Frontmatter parser
- Detect leading `---` at start of file, read until the next line that is exactly `---`
- Parse YAML with `serde_yaml` into `Frontmatter`
- Return remainder as the prompt body

2) Metadata filling
- `description`: from frontmatter, else fallback to first non-empty body line (trimmed)
- `argument-hint`: optional string
- `allowed-tools` and `model`: stored but enforcement/use deferred

3) UI display
- Build display line as: `/name` + ` (scope:namespace)` + ` — description` + ` [argument-hint]` (hint styled dim)

## Acceptance Criteria
- Prompts with frontmatter show the provided description and argument hint
- Prompts without frontmatter show the first body line as description; no hint
- No regressions for listing when no frontmatter is present

## Tests
- Core unit tests for `parse_frontmatter` with:
  - Valid YAML block, missing fields, malformed YAML (gracefully ignore with logs)
  - Fallback to first line when description missing
- TUI snapshot/row tests to confirm display formatting

## Risks / Notes
- Keep parser tolerant: if frontmatter is malformed, skip it and use fallbacks
- Ensure stripping the frontmatter from body so users don’t send raw metadata to the model

---

References
- Protocol meta: `codex-rs/protocol/src/custom_prompts.rs`
- Core parsing: `codex-rs/core/src/custom_prompts.rs`
- TUI rendering: `codex-rs/tui/src/bottom_pane/command_popup.rs`
