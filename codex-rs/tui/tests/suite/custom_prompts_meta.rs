use codex_protocol::custom_prompts::CustomPromptMeta;
use codex_protocol::custom_prompts::PromptScope;
use insta::assert_snapshot;
use std::collections::HashMap;
use std::path::PathBuf;

// T008: Slash popup shows description and argument-hint from meta
#[test]
fn slash_popup_lists_prompts_with_meta() {
    let mut meta: HashMap<String, CustomPromptMeta> = HashMap::new();
    meta.insert(
        "build".into(),
        CustomPromptMeta {
            name: "build".into(),
            path: PathBuf::from("/user/prompts/build.md"),
            scope: PromptScope::User,
            namespace: vec!["dev".into()],
            description: Some("Build the project".into()),
            argument_hint: Some("<target>".into()),
            model: None,
        },
    );

    // Render using a function we will introduce in Phase 3.4.
    let rendered = codex_tui::bottom_pane::render_slash_popup_with_meta_for_test(meta);
    assert_snapshot!(rendered);
}

// T009: Default model preference on submit
#[test]
fn prompt_submission_prefers_meta_model_when_present() {
    // Placeholder driving TDD: we will introduce a helper that determines the
    // default model for a prompt, preferring the prompt meta when present.
    let chosen = codex_tui::bottom_pane::choose_default_model_for_prompt_for_test(
        Some("gpt-5-medium".into()),
        Some("gpt-4o".into()),
    );
    assert_eq!(chosen.as_deref(), Some("gpt-5-medium"));
}
