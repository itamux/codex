//! Protocol types for custom prompts.
//!
//! Prompt files are simple Markdown files. They may optionally begin with a YAML
//! frontmatter block delimited by lines containing exactly `---`. When present,
//! known string keys from the frontmatter are surfaced in `CustomPromptMeta`
//! (e.g., `description`, `argument_hint`). The parsing and population of these
//! fields is performed in the `codex-core` crate.
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomPrompt {
    pub name: String,
    pub path: PathBuf,
    pub content: String,
}

/// Scope for a discovered custom prompt.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptScope {
    Project,
    User,
}

/// Metadata for a discovered custom prompt, enriched with scope and namespace.
///
/// The `description` and `argument_hint` fields may be extracted from optional
/// YAML frontmatter in the prompt file (handled by `codex-core`). When no
/// frontmatter is present or keys are missing, these remain `None`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomPromptMeta {
    pub name: String,
    pub path: PathBuf,
    pub scope: PromptScope,
    /// Subdirectory components under the root prompts folder.
    pub namespace: Vec<String>,
    /// Optional summary extracted from YAML frontmatter.
    pub description: Option<String>,
    /// Optional argument hint extracted from YAML frontmatter.
    pub argument_hint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::from_str as from_json;
    use serde_json::to_string_pretty as to_json;

    #[test]
    fn custom_prompt_meta_json_serializes_with_optional_model() {
        // T004: Ensure serde shape includes optional `model` when present.
        let meta = CustomPromptMeta {
            name: "my-prompt".to_string(),
            path: PathBuf::from("/abs/path/my-prompt.md"),
            scope: PromptScope::User,
            namespace: vec!["team".to_string(), "sub".to_string()],
            description: Some("Short description".to_string()),
            argument_hint: Some("<arg>".to_string()),
            // model: Some("gpt-5-medium".to_string()),
        };

        // When `model` is Some, JSON should include it; when None it should be omitted or null.
        let json = to_json(&meta).expect("serialize");
        // Roundtrip – construct with model to assert presence.
        let with_model = format!(
            "{{\n  \"name\": \"{}\",\n  \"path\": \"{}\",\n  \"scope\": \"user\",\n  \"namespace\": [\n    \"team\",\n    \"sub\"\n  ],\n  \"description\": \"Short description\",\n  \"argument_hint\": \"<arg>\",\n  \"model\": \"gpt-5-medium\"\n}}",
            meta.name,
            meta.path.display()
        );
        // Parse JSON with a `model` key and ensure it deserializes.
        let parsed: CustomPromptMeta = from_json(&with_model).expect("deserialize with model");
        assert_eq!(parsed.name, meta.name);
        assert_eq!(parsed.scope, PromptScope::User);
        assert_eq!(parsed.namespace, meta.namespace);
        assert_eq!(parsed.description, meta.description);
        assert_eq!(parsed.argument_hint, meta.argument_hint);
        // After implementing T010, this should be Some.
        assert_eq!(parsed.model.as_deref(), Some("gpt-5-medium"));

        // Also verify that serializing a struct with model set will include the field.
        let json_with_model = to_json(&CustomPromptMeta {
            model: Some("gpt-5-medium".into()),
            ..meta
        })
        .unwrap();
        assert!(json_with_model.contains("\"model\""));

        // For now, keep the baseline JSON without model to drive TDD failure.
        assert!(json.contains("\"name\""));
        assert!(!json.contains("\"__nonexistent_key__\""));
    }
}
