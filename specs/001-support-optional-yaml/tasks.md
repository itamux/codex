# Tasks: Support optional YAML frontmatter in prompt files

**Input**: Design documents from `/home/iatzmon/workspace/codex/specs/001-support-optional-yaml/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/

## Execution Flow (applied)
1) Loaded plan.md; extracted tech stack (`serde_yaml`, tokio, ratatui) and structure (codex-protocol, codex-core, codex-tui).  
2) Loaded optional docs: data-model (entities), contracts (protocol-change.md), research (decisions), quickstart (scenarios).  
3) Generated TDD-ordered tasks with [P] for parallelizable items across different files.

## Phase 3.1: Setup
- [x] T001 Add serde_yaml dependency to `codex-rs/core/Cargo.toml`; keep versions consistent. File: `/home/iatzmon/workspace/codex/codex-rs/core/Cargo.toml`. Command: `cargo check -p codex-core`.
- [x] T002 Ensure protocol crate exposes updated `CustomPromptMeta` docs. File: `/home/iatzmon/workspace/codex/codex-rs/protocol/src/custom_prompts.rs`. Command: `cargo check -p codex-protocol`.
- [x] T003 [P] Prepare temporary prompt fixtures folder under tests using `tempfile` (helper function). File: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs` (tests module).

Implementation details (for Phase 3.2+):
- serde_yaml added to core: `serde_yaml = "0.9"` in `codex-rs/core/Cargo.toml`.
  - Builds pull in `unsafe-libyaml`; `cargo check -p codex-core` succeeds.
- Protocol docs clarified: `codex-rs/protocol/src/custom_prompts.rs` now documents that `description` and `argument_hint` may be extracted from optional YAML frontmatter by core; wire-up still pending until Phase 3.3.
- Test fixtures helper scaffolded in core tests: `PromptFixtures` in `codex-rs/core/src/custom_prompts.rs` test module.
  - Structure: creates two roots under a tempdir
    - `user/prompts` (user scope, equivalent to `$CODEX_HOME/prompts`)
    - `project/.codex/prompts` (project scope)
  - Helpers: `new()`, `user_dir()`, `project_dir()`, `write_user(rel, content)`, `write_project(rel, content)`.
  - Intended for T005–T007 tests to quickly arrange trees and files.
- Env/test note: an unrelated shell-detection test in core (`shell::tests::test_current_shell_detects_zsh`) can fail depending on host env. When focusing on these tasks, filter to relevant tests, e.g.:
  - `cargo test -p codex-core -- custom_prompts::`
  - `cargo test -p codex-protocol -- tests::custom_prompts_meta`

## Phase 3.2: Tests First (TDD)
CRITICAL: Write tests and ensure they FAIL before implementation.
- [x] T004 Contract test [P]: Validate protocol sample from contracts doc. Create a test ensuring `CustomPromptMeta` serializes/deserializes with optional `model`. File: `/home/iatzmon/workspace/codex/codex-rs/protocol/src/custom_prompts.rs` (tests). Command: `cargo test -p codex-protocol`.
- [x] T005 Core unit tests [P]: Frontmatter detection and YAML parsing (valid block; missing terminator → ignored; malformed YAML → ignored; unknown keys ignored; non-string types ignored). File: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs` (tests). Command: `cargo test -p codex-core`.
- [x] T006 Core unit tests [P]: Description fallback rules and CRLF handling; first non-empty content line after frontmatter; no Markdown stripping. File: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs` (tests). Command: `cargo test -p codex-core`.
- [x] T007 Integration test [P]: Aggregation returns both `custom_prompts` and `custom_prompts_meta` with meta populated (description, argument_hint, model). Target Op: `ListCustomPromptsResponse`. File: `/home/iatzmon/workspace/codex/codex-rs/core/src/codex.rs` (integration or unit-level harness). Command: `cargo test -p codex-core`.
- [x] T008 TUI snapshot tests [P]: Slash popup shows description from meta; argument-hint appears in autocomplete/help. Update or add tests under `/home/iatzmon/workspace/codex/codex-rs/tui/tests` and snapshot directories. Commands: `cargo test -p codex-tui` then `cargo insta pending-snapshots -p codex-tui`.
- [x] T009 TUI integration test [P]: When running a prompt, the default model used equals the prompt meta’s `model` (or default). File: `/home/iatzmon/workspace/codex/codex-rs/tui` tests. Command: `cargo test -p codex-tui`.

Implementation details (for Phase 3.2):
- Protocol (T004): Added a serde roundtrip test targeting `CustomPromptMeta` with an expected `model: Option<String>` field in `/home/iatzmon/workspace/codex/codex-rs/protocol/src/custom_prompts.rs`. The test asserts deserialization of a JSON object with `model` and expects the field to be present on the struct and included on serialization. This currently fails to compile (no `model` yet), driving T010.
- Core (T005–T007): Added unit tests in `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs` that exercise a to‑be‑implemented `parse_frontmatter_and_body(&str)` helper:
  - Valid frontmatter parsed; unknown keys ignored; only string values accepted.
  - Malformed YAML and missing closing `---` are ignored (treated as body only).
  - Description fallback and CRLF behavior validated; body preserved verbatim (no Markdown stripping).
  - Aggregation test ensures `discover_user_and_project_custom_prompt_meta` returns meta populated from frontmatter. These tests currently fail to compile due to the missing helper and will later pass with T011–T013.
- TUI (T008–T009): Added new tests under `/home/iatzmon/workspace/codex/codex-rs/tui/tests/suite/custom_prompts_meta.rs` using `insta`:
  - Snapshot for slash popup rendering that expects description and argument hint.
  - Test that default model on submit prefers the prompt meta’s model. These reference helper functions to be added in Phase 3.4/3.5, and compilation currently fails (module private and functions missing), intentionally enforcing TDD.

Validation commands and status:
- Ran `cargo test -p codex-protocol` (filtered to the new test): compile fails as expected due to missing `model`.
- Ran `cargo test -p codex-core -- custom_prompts::`: compile fails as expected due to missing `parse_frontmatter_and_body`.
- Ran `cargo test -p codex-tui --test all -- tests::suite::custom_prompts_meta::`: compile fails as expected due to missing helpers and private module.
- After implementation in Phases 3.3–3.4, re-run each package’s tests and then consider snapshot acceptance via `cargo insta pending-snapshots -p codex-tui` followed by `cargo insta accept -p codex-tui` if diffs are intended.

## Phase 3.3: Core Implementation (after tests are failing)
- [ ] T010 Protocol change: Add `model: Option<String>` to `CustomPromptMeta`; document allowed values. File: `/home/iatzmon/workspace/codex/codex-rs/protocol/src/custom_prompts.rs`. Command: `cargo check -p codex-protocol`.
- [ ] T011 Core parsing helper: Implement `parse_frontmatter_and_body(&str) -> (Meta, Body)` using `serde_yaml`, exact `---` rules, string-only known keys, warnings on errors. File: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs`. Command: `cargo check -p codex-core`.
- [ ] T012 Populate meta during discovery: In `discover_user_and_project_custom_prompt_meta`, read file once, parse frontmatter, fill `description`, `argument_hint`, `model`; set `CustomPrompt.content` to body (after frontmatter). File: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs`. Command: `cargo test -p codex-core`.
- [ ] T013 Validation defaults: Enforce allowed `model` values; fallback to `gpt-5-medium` on invalid/missing; emit `tracing::warn!`. File: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs`. Command: `cargo test -p codex-core`.

## Phase 3.4: TUI Implementation
- [ ] T014 [P] Consume meta in UI: Update `/home/iatzmon/workspace/codex/codex-rs/tui/src/bottom_pane/mod.rs` and `/home/iatzmon/workspace/codex/codex-rs/tui/src/bottom_pane/chat_composer.rs` to display `description` and `argument-hint` based on `custom_prompts_meta`. Command: `cargo test -p codex-tui`.
- [ ] T015 Default model on submit: When submitting a prompt, prefer meta.model as the per-turn default model (OverrideTurnContext or equivalent). Likely changes in `/home/iatzmon/workspace/codex/codex-rs/tui/src/bottom_pane/chat_composer.rs` and event dispatch path. Command: `cargo test -p codex-tui`.

## Phase 3.5: Polish
- [ ] T016 [P] Update docs with frontmatter behavior and examples. Files: `/home/iatzmon/workspace/codex/docs/architecture/custom-prompts.md`, and link `quickstart.md`. Command: n/a.
- [ ] T017 [P] Ensure logs are helpful: Add/verify `tracing::warn!` messages for malformed YAML/invalid model. Files: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs`. Command: `cargo test -p codex-core`.
- [ ] T018 [P] Performance sanity: Add a benchmark-ish test iterating many small files to ensure discovery remains fast (optional). Files: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs` tests. Command: `cargo test -p codex-core`.
- [ ] T019 Backward compatibility regression: Tests for prompts without frontmatter unchanged. File: `/home/iatzmon/workspace/codex/codex-rs/core/src/custom_prompts.rs` tests. Command: `cargo test -p codex-core`.
- [ ] T020 Manual E2E checklist: Create sample prompts with/without frontmatter and validate TUI/exec behavior using `quickstart.md`. Command: manual.

## Dependencies
- Setup (T001–T003) → all tests
- Tests (T004–T009) before implementation (T010–T015)
- Core parsing (T011) blocks discovery wiring (T012) and validation defaulting (T013)
- T014 depends on protocol meta present in events (T010, T012)
- Implementation before polish (T016–T020)

## Parallel Execution Examples
Run independent tests in parallel:
```
# In one shell
cargo test -p codex-protocol -- tests::custom_prompts_meta

# In another shell
cargo test -p codex-core -- custom_prompts::

# In another shell
cargo test -p codex-tui
```

Run [P] tasks together:
- T003 (fixtures helper) + T004 (protocol test) + T005 (core parse tests) + T008 (TUI snapshots)

## Notes
- Use absolute file paths as specified.
- Follow TDD: ensure tests fail first, then implement.
- Keep changes additive and backward compatible.
- For TUI, follow styling conventions in `/home/iatzmon/workspace/codex/codex-rs/tui/styles.md` and snapshot update process from the spec.
