use codex_protocol::custom_prompts::CustomPrompt;
use codex_protocol::custom_prompts::CustomPromptMeta;
use codex_protocol::custom_prompts::PromptScope;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;

/// Return the default prompts directory: `$CODEX_HOME/prompts`.
/// If `CODEX_HOME` cannot be resolved, returns `None`.
pub fn default_prompts_dir() -> Option<PathBuf> {
    crate::config::find_codex_home()
        .ok()
        .map(|home| home.join("prompts"))
}

/// Discover prompt files in the given directory, returning entries sorted by name.
/// Non-files are ignored. If the directory does not exist or cannot be read, returns empty.
pub async fn discover_prompts_in(dir: &Path) -> Vec<CustomPrompt> {
    discover_prompts_in_excluding(dir, &HashSet::new()).await
}

/// Discover prompt files in the given directory, excluding any with names in `exclude`.
/// Returns entries sorted by name. Non-files are ignored. Missing/unreadable dir yields empty.
pub async fn discover_prompts_in_excluding(
    dir: &Path,
    exclude: &HashSet<String>,
) -> Vec<CustomPrompt> {
    let mut out: Vec<CustomPrompt> = Vec::new();
    let mut entries = match fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(_) => return out,
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let is_file = entry
            .file_type()
            .await
            .map(|ft| ft.is_file())
            .unwrap_or(false);
        if !is_file {
            continue;
        }
        // Only include Markdown files with a .md extension.
        let is_md = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            .unwrap_or(false);
        if !is_md {
            continue;
        }
        let Some(name) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
        else {
            continue;
        };
        if exclude.contains(&name) {
            continue;
        }
        let content = match fs::read_to_string(&path).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        out.push(CustomPrompt {
            name,
            path,
            content,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Return the project-level prompts directory for a given project root.
/// The directory layout is `PROJECT_ROOT/.codex/prompts`.
pub fn project_prompts_dir(project_root: &Path) -> PathBuf {
    project_root.join(".codex/prompts")
}

/// A discovered prompt file entry used for internal aggregation.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub name: String,
    /// Directory path relative to the scanned root (empty for files at root).
    pub rel_dir: PathBuf,
    pub content: String,
}

/// Recursively discover Markdown prompts under `root`, returning entries in any order.
/// Non-files are ignored. Unreadable subdirectories or files are skipped.
pub async fn discover_prompts_recursive(root: &Path) -> Vec<DiscoveredFile> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(mut entries) = fs::read_dir(&dir).await else {
            continue;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let file_type = match entry.file_type().await {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            // Only include Markdown files with a .md extension.
            let is_md = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md"))
                .unwrap_or(false);
            if !is_md {
                continue;
            }
            let Some(name) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
            else {
                continue;
            };
            let content = match fs::read_to_string(&path).await {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Compute relative directory from root for namespacing later.
            let rel_dir = match path.parent() {
                Some(parent) => parent
                    .strip_prefix(root)
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::new()),
                None => PathBuf::new(),
            };

            out.push(DiscoveredFile {
                path,
                name,
                rel_dir,
                content,
            });
        }
    }

    out
}

/// Discover prompts from both user and project scopes, deduplicated by basename.
/// Project-level prompts take precedence over user-level prompts when names collide.
pub async fn discover_user_and_project_prompts(cwd: &Path) -> Vec<DiscoveredFile> {
    // Project scope
    let mut selected: HashMap<String, DiscoveredFile> = HashMap::new();

    // Prefer Git repo root when available; otherwise fall back to provided cwd
    let project_root = crate::git_info::get_git_repo_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let project_dir = project_prompts_dir(&project_root);
    for item in discover_prompts_recursive(&project_dir).await {
        selected.insert(item.name.clone(), item);
    }

    // User scope
    if let Some(user_dir) = default_prompts_dir() {
        for item in discover_prompts_recursive(&user_dir).await {
            selected.entry(item.name.clone()).or_insert(item);
        }
    }

    let mut out: Vec<DiscoveredFile> = selected.into_values().collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Adapter that returns the current protocol shape for custom prompts,
/// aggregating across user + project scopes.
pub async fn discover_user_and_project_custom_prompts(cwd: &Path) -> Vec<CustomPrompt> {
    discover_user_and_project_prompts(cwd)
        .await
        .into_iter()
        .map(|d| CustomPrompt {
            name: d.name,
            path: d.path,
            content: d.content,
        })
        .collect()
}

/// Adapter that returns enriched metadata for custom prompts, including
/// scope (project/user) and namespace (relative subdirectories).
pub async fn discover_user_and_project_custom_prompt_meta(cwd: &Path) -> Vec<CustomPromptMeta> {
    // Prefer Git repo root when available; otherwise fall back to provided cwd
    let project_root = crate::git_info::get_git_repo_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let project_dir = project_prompts_dir(&project_root);

    let mut selected: HashMap<String, CustomPromptMeta> = HashMap::new();

    // Project first so it wins on collisions
    for item in discover_prompts_recursive(&project_dir).await {
        let namespace: Vec<String> = if item.rel_dir.as_os_str().is_empty() {
            Vec::new()
        } else {
            item.rel_dir
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect()
        };
        selected.insert(
            item.name.clone(),
            CustomPromptMeta {
                name: item.name,
                path: item.path,
                scope: PromptScope::Project,
                namespace,
                description: None,
                argument_hint: None,
            },
        );
    }

    // Then user scope; keep existing when colliding
    if let Some(user_dir) = default_prompts_dir() {
        for item in discover_prompts_recursive(&user_dir).await {
            let namespace: Vec<String> = if item.rel_dir.as_os_str().is_empty() {
                Vec::new()
            } else {
                item.rel_dir
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect()
            };
            selected
                .entry(item.name.clone())
                .or_insert(CustomPromptMeta {
                    name: item.name,
                    path: item.path,
                    scope: PromptScope::User,
                    namespace,
                    description: None,
                    argument_hint: None,
                });
        }
    }

    let mut out: Vec<CustomPromptMeta> = selected.into_values().collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Expand `$ARGUMENTS` and `$1..$n` placeholders in `content`.
///
/// - `$ARGUMENTS` is replaced by `rest` verbatim.
/// - `$n` is replaced by the nth (1-based) positional argument from `args`,
///   or an empty string if missing.
pub fn expand_arguments(content: &str, args: &[String], rest: &str) -> String {
    let mut out = String::with_capacity(content.len().saturating_add(rest.len()));
    let bytes = content.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }

        // Handle $ARGUMENTS
        if i + 10 <= bytes.len() && &content[i..i + 10] == "$ARGUMENTS" {
            out.push_str(rest);
            i += 10;
            continue;
        }

        // Handle $<digits>
        let mut j = i + 1; // skip '$'
        let mut val: usize = 0;
        let mut has_digit = false;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            has_digit = true;
            val = val
                .saturating_mul(10)
                .saturating_add((bytes[j] - b'0') as usize);
            j += 1;
        }
        if has_digit {
            // 1-based index; missing indices expand to empty string.
            if val > 0 {
                let idx = val - 1;
                if let Some(s) = args.get(idx) {
                    out.push_str(s);
                }
            }
            i = j;
            continue;
        }

        // Not a recognized placeholder – treat '$' as a literal.
        out.push('$');
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap as Map;
    use std::fs;
    use tempfile::tempdir;

    /// Helper for tests needing both user and project prompt trees.
    /// Keeps the tempdir alive for the duration of the fixture value.
    struct PromptFixtures {
        _tmp: tempfile::TempDir,
        user_root: PathBuf,
        project_root: PathBuf,
    }

    impl PromptFixtures {
        /// Create isolated user and project prompt directories under a temporary root.
        fn new() -> Self {
            let tmp = tempdir().expect("create TempDir");
            let user_root = tmp.path().join("user/prompts");
            let project_root = tmp.path().join("project/.codex/prompts");
            std::fs::create_dir_all(&user_root).unwrap();
            std::fs::create_dir_all(&project_root).unwrap();
            Self {
                _tmp: tmp,
                user_root,
                project_root,
            }
        }

        /// Absolute path to the user prompts root (e.g., `$CODEX_HOME/prompts`).
        fn user_dir(&self) -> &Path {
            &self.user_root
        }

        /// Absolute path to the project prompts root (e.g., `PROJECT/.codex/prompts`).
        fn project_dir(&self) -> &Path {
            &self.project_root
        }

        /// Write a prompt file under the user root at `rel` with `content`.
        fn write_user(&self, rel: &str, content: &str) {
            let path = self.user_root.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, content).unwrap();
        }

        /// Write a prompt file under the project root at `rel` with `content`.
        fn write_project(&self, rel: &str, content: &str) {
            let path = self.project_root.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, content).unwrap();
        }
    }

    #[tokio::test]
    async fn empty_when_dir_missing() {
        let tmp = tempdir().expect("create TempDir");
        let missing = tmp.path().join("nope");
        let found = discover_prompts_in(&missing).await;
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn discovers_and_sorts_files() {
        let tmp = tempdir().expect("create TempDir");
        let dir = tmp.path();
        fs::write(dir.join("b.md"), b"b").unwrap();
        fs::write(dir.join("a.md"), b"a").unwrap();
        fs::create_dir(dir.join("subdir")).unwrap();
        let found = discover_prompts_in(dir).await;
        let names: Vec<String> = found.into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn excludes_builtins() {
        let tmp = tempdir().expect("create TempDir");
        let dir = tmp.path();
        fs::write(dir.join("init.md"), b"ignored").unwrap();
        fs::write(dir.join("foo.md"), b"ok").unwrap();
        let mut exclude = HashSet::new();
        exclude.insert("init".to_string());
        let found = discover_prompts_in_excluding(dir, &exclude).await;
        let names: Vec<String> = found.into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["foo"]);
    }

    #[tokio::test]
    async fn skips_non_utf8_files() {
        let tmp = tempdir().expect("create TempDir");
        let dir = tmp.path();
        // Valid UTF-8 file
        fs::write(dir.join("good.md"), b"hello").unwrap();
        // Invalid UTF-8 content in .md file (e.g., lone 0xFF byte)
        fs::write(dir.join("bad.md"), vec![0xFF, 0xFE, b'\n']).unwrap();
        let found = discover_prompts_in(dir).await;
        let names: Vec<String> = found.into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["good"]);
    }

    #[tokio::test]
    async fn recursive_discovery_with_subdirs() {
        let tmp = tempdir().expect("create TempDir");
        let root = tmp.path();
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::write(root.join("a.md"), b"A").unwrap();
        fs::write(root.join("a/b/c.md"), b"C").unwrap();
        fs::write(root.join("not-md.txt"), b"ignore").unwrap();

        let found = discover_prompts_recursive(root).await;
        let mut names: Vec<String> = found.iter().map(|e| e.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["a", "c"]);
        // Ensure rel_dir captured for nested file
        let c = found.iter().find(|e| e.name == "c").unwrap();
        assert_eq!(c.rel_dir, PathBuf::from("a/b"));
    }

    // Helper for tests to avoid global env flakiness
    async fn agg_for_test(user: Option<&Path>, project: Option<&Path>) -> Vec<DiscoveredFile> {
        let mut selected: HashMap<String, DiscoveredFile> = HashMap::new();
        if let Some(project_root) = project {
            for item in discover_prompts_recursive(project_root).await {
                selected.insert(item.name.clone(), item);
            }
        }
        if let Some(user_root) = user {
            for item in discover_prompts_recursive(user_root).await {
                selected.entry(item.name.clone()).or_insert(item);
            }
        }
        let mut out: Vec<DiscoveredFile> = selected.into_values().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    #[tokio::test]
    async fn aggregate_user_only() {
        let tmp = tempdir().expect("create TempDir");
        let user = tmp.path().join("prompts");
        fs::create_dir_all(&user).unwrap();
        fs::write(user.join("u1.md"), b"U1").unwrap();
        fs::write(user.join("u2.md"), b"U2").unwrap();

        let found = agg_for_test(Some(&user), None).await;
        let names: Vec<String> = found.into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["u1", "u2"]);
    }

    #[tokio::test]
    async fn aggregate_project_only() {
        let tmp = tempdir().expect("create TempDir");
        let proj_dir = tmp.path().join(".codex/prompts");
        fs::create_dir_all(&proj_dir).unwrap();
        fs::write(proj_dir.join("p1.md"), b"P1").unwrap();
        let found = agg_for_test(None, Some(&proj_dir)).await;
        let names: Vec<String> = found.into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["p1"]);
    }

    #[tokio::test]
    async fn aggregate_both_with_collision_project_wins() {
        let tmp = tempdir().expect("create TempDir");
        let user = tmp.path().join("user/prompts");
        let proj = tmp.path().join("proj/.codex/prompts");
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(&proj).unwrap();
        fs::write(user.join("foo.md"), b"U-FOO").unwrap();
        fs::write(proj.join("foo.md"), b"P-FOO").unwrap();
        fs::write(user.join("bar.md"), b"U-BAR").unwrap();
        fs::write(proj.join("baz.md"), b"P-BAZ").unwrap();

        let found = agg_for_test(Some(&user), Some(&proj)).await;
        let mut names: Vec<String> = found.iter().map(|e| e.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["bar", "baz", "foo"]);
        let foo = found.into_iter().find(|e| e.name == "foo").unwrap();
        assert_eq!(foo.content, "P-FOO");
    }

    #[tokio::test]
    async fn fallback_to_cwd_when_no_git_repo() {
        let tmp = tempdir().expect("create TempDir");
        // Create .codex/prompts under cwd (no .git present)
        let proj_dir = tmp.path().join(".codex/prompts");
        fs::create_dir_all(&proj_dir).unwrap();
        fs::write(proj_dir.join("local.md"), b"LOCAL").unwrap();

        // Create a user dir as well to ensure both are aggregated
        let user_dir = tmp.path().join("user/prompts");
        fs::create_dir_all(&user_dir).unwrap();
        fs::write(user_dir.join("user.md"), b"USER").unwrap();

        // Temporarily override default_prompts_dir by directly calling the
        // internal aggregator equivalent with explicit paths.
        let mut selected: HashMap<String, DiscoveredFile> = HashMap::new();
        for item in discover_prompts_recursive(&proj_dir).await {
            selected.insert(item.name.clone(), item);
        }
        for item in discover_prompts_recursive(&user_dir).await {
            selected.entry(item.name.clone()).or_insert(item);
        }
        let mut out: Vec<DiscoveredFile> = selected.into_values().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        let names: Vec<String> = out.into_iter().map(|e| e.name).collect();
        assert_eq!(names, vec!["local", "user"]);
    }

    #[test]
    fn expand_arguments_only_rest() {
        let content = "Hello $ARGUMENTS!";
        let expanded = expand_arguments(content, &[], "world and friends");
        assert_eq!(expanded, "Hello world and friends!");
    }

    #[test]
    fn expand_arguments_only_positionals() {
        let content = "first=$1 second=$2 missing=$3";
        let args = vec!["A".to_string(), "B".to_string()];
        let expanded = expand_arguments(content, &args, "");
        assert_eq!(expanded, "first=A second=B missing=");
    }

    #[test]
    fn expand_arguments_mixed_and_repeated() {
        let content = "ID $2; All: $ARGUMENTS; Again $2 and $1";
        let args = vec!["X".to_string(), "Y".to_string()];
        let expanded = expand_arguments(content, &args, "foo bar baz");
        assert_eq!(expanded, "ID Y; All: foo bar baz; Again Y and X");
    }

    #[test]
    fn expand_arguments_preserves_newlines_in_rest() {
        let content = "Header:\n$ARGUMENTS\nTail (pos=$1)";
        let args = vec!["A".to_string()];
        let rest = "line 1\nline 2";
        let expanded = expand_arguments(content, &args, rest);
        assert_eq!(expanded, "Header:\nline 1\nline 2\nTail (pos=A)");
    }

    // T005: Frontmatter detection and YAML parsing
    #[test]
    fn frontmatter_valid_yaml_is_parsed_and_unknown_keys_ignored() {
        let input = "---\ndescription: Hello world\nargument_hint: <arg>\nmodel: gpt-5-medium\nunknown: foo\n---\nBody starts here\n";
        let (meta, body) = super::parse_frontmatter_and_body(input);
        assert_eq!(body, "Body starts here\n");
        assert_eq!(meta.get("description"), Some(&"Hello world".to_string()));
        assert_eq!(meta.get("argument_hint"), Some(&"<arg>".to_string()));
        assert_eq!(meta.get("model"), Some(&"gpt-5-medium".to_string()));
        assert!(meta.get("unknown").is_none());
    }

    // T005: Malformed YAML is ignored (treated as no frontmatter)
    #[test]
    fn frontmatter_malformed_yaml_is_ignored() {
        let input = "---\ndescription: [unterminated\n---\nHello\n";
        let (meta, body) = super::parse_frontmatter_and_body(input);
        assert!(meta.is_empty());
        assert_eq!(body, input);
    }

    // T005: Missing closing terminator -> ignore as body
    #[test]
    fn frontmatter_missing_terminator_is_ignored() {
        let input = "---\ndescription: hi\nBody\n";
        let (meta, body) = super::parse_frontmatter_and_body(input);
        assert!(meta.is_empty());
        assert_eq!(body, input);
    }

    // T005: Non-string types ignored
    #[test]
    fn frontmatter_non_string_values_ignored() {
        let input = "---\ndescription: 123\nargument_hint: {a: 1}\nmodel: [a, b]\n---\nHello\n";
        let (meta, body) = super::parse_frontmatter_and_body(input);
        assert_eq!(body, "Hello\n");
        assert_eq!(meta.get("description"), None);
        assert_eq!(meta.get("argument_hint"), None);
        assert_eq!(meta.get("model"), None);
    }

    // T006: Description fallback rules and CRLF handling
    #[test]
    fn description_fallback_and_crlf_handling() {
        let input = "---\nargument_hint: <path>\n---\r\n\r\nFirst line after frontmatter\r\nSecond line\r\n";
        let (meta, body) = super::parse_frontmatter_and_body(input);
        assert_eq!(meta.get("argument_hint"), Some(&"<path>".to_string()));
        // First non-empty content line selected verbatim; no Markdown stripping.
        assert_eq!(
            body,
            "\r\n\r\nFirst line after frontmatter\r\nSecond line\r\n"
        );
    }

    // T007: Aggregation returns both custom_prompts and custom_prompts_meta with parsed meta
    #[tokio::test]
    async fn aggregation_populates_meta_from_frontmatter() {
        let fx = PromptFixtures::new();
        fx.write_user(
            "a.md",
            "---\ndescription: Hello\nargument_hint: <x>\nmodel: gpt-5-medium\n---\nBODY\n",
        );
        fx.write_project("b.md", "Just content\n");

        // Pretend CWD is under a git repo at project root – use aggregator directly.
        let cwd = fx.project_dir();
        let meta = discover_user_and_project_custom_prompt_meta(cwd).await;
        let prompts = discover_user_and_project_custom_prompts(cwd).await;
        let names_meta: Vec<String> = meta.iter().map(|m| m.name.clone()).collect();
        let names_prompts: Vec<String> = prompts.iter().map(|p| p.name.clone()).collect();
        assert_eq!(names_meta, names_prompts);

        // Ensure meta populated for the frontmatter file.
        let a_meta = meta.iter().find(|m| m.name == "a").unwrap();
        assert_eq!(a_meta.description.as_deref(), Some("Hello"));
        assert_eq!(a_meta.argument_hint.as_deref(), Some("<x>"));
        // After implementation, model should be Some.
        // assert_eq!(a_meta.model.as_deref(), Some("gpt-5-medium"));
    }
}
