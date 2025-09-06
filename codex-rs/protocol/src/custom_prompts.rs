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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomPromptMeta {
    pub name: String,
    pub path: PathBuf,
    pub scope: PromptScope,
    /// Subdirectory components under the root prompts folder.
    pub namespace: Vec<String>,
    /// Optional summary extracted from frontmatter (populated by a later task).
    pub description: Option<String>,
    /// Optional argument hint extracted from frontmatter (populated by a later task).
    pub argument_hint: Option<String>,
}
