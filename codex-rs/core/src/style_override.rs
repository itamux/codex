use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StyleDoc {
    pub kind: String,
    pub version: u32,
    pub regions: Regions,
    #[allow(dead_code)]
    pub selectors: Option<Selectors>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Regions {
    pub personality: Option<RegionSpec>,
    pub presentation: Option<RegionSpec>,
    pub preambles: Option<RegionSpec>,
    pub planning: Option<RegionSpec>,
    pub progress: Option<RegionSpec>,
}

#[derive(Debug, Deserialize)]
pub struct RegionSpec {
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Mode {
    #[default]
    Replace,
    Merge,
    Disable,
}

#[derive(Debug, Deserialize, Default)]
pub struct Selectors {
    pub personality: Option<Selector>,
    pub presentation: Option<Selector>,
    pub preambles: Option<Selector>,
    pub planning: Option<Selector>,
    pub progress: Option<Selector>,
}

#[derive(Debug, Deserialize)]
pub struct Selector {
    pub heading: Option<String>,
}

/// Attempt to parse YAML style && apply region replacements to the base prompt.
/// Returns Some(replaced_string) on success, or None if the input is not a style YAML.
pub fn apply_style_yaml(base: &str, yaml: &str) -> Option<String> {
    let doc: StyleDoc = match serde_yaml::from_str::<StyleDoc>(yaml).ok()? {
        d if d.kind == "codex-style" && d.version == 1 => d,
        _ => return None,
    };

    let mut out = base.to_string();
    // Apply in a stable order to avoid overlap.
    if let Some(spec) = &doc.regions.personality {
        out = apply_region(
            &out,
            spec,
            doc.selectors
                .as_ref()
                .and_then(|s| s.personality.as_ref())
                .and_then(|s| s.heading.clone()),
            "## Personality",
            &["
## "],
        );
    }
    if let Some(spec) = &doc.regions.presentation {
        out = apply_region(
            &out,
            spec,
            doc.selectors
                .as_ref()
                .and_then(|s| s.presentation.as_ref())
                .and_then(|s| s.heading.clone()),
            "## Presenting your work and final message",
            &["
## "],
        );
    }
    if let Some(spec) = &doc.regions.preambles {
        out = apply_region(
            &out,
            spec,
            doc.selectors
                .as_ref()
                .and_then(|s| s.preambles.as_ref())
                .and_then(|s| s.heading.clone()),
            "### Preamble messages",
            &[
                "
### ", "
## ",
            ],
        );
    }
    if let Some(spec) = &doc.regions.planning {
        out = apply_region(
            &out,
            spec,
            doc.selectors
                .as_ref()
                .and_then(|s| s.planning.as_ref())
                .and_then(|s| s.heading.clone()),
            "## Planning",
            &["
## "],
        );
    }
    if let Some(spec) = &doc.regions.progress {
        out = apply_region(
            &out,
            spec,
            doc.selectors
                .as_ref()
                .and_then(|s| s.progress.as_ref())
                .and_then(|s| s.heading.clone()),
            "## Sharing progress updates",
            &["
## "],
        );
    }
    Some(out)
}

fn apply_region(
    base: &str,
    spec: &RegionSpec,
    selector_heading: Option<String>,
    default_heading: &str,
    next_markers: &[&str],
) -> String {
    let heading = selector_heading.as_deref().unwrap_or(default_heading);
    if let Some((start, end)) = find_section_range(base, heading, next_markers) {
        match spec.mode {
            Mode::Replace => replace_range_with(base, start, end, heading, &spec.text),
            Mode::Merge => merge_after_heading(base, start, end, heading, &spec.text),
            Mode::Disable => remove_range_normalized(base, start, end),
        }
    } else {
        // If not found, append a replacement marker at the end to avoid silent failure.
        let mut out = base.to_string();
        if matches!(spec.mode, Mode::Replace | Mode::Merge) && !spec.text.trim().is_empty() {
            out.push_str(&format!(
                "

{heading}

{}
",
                spec.text
            ));
        }
        out
    }
}

fn find_section_range(base: &str, heading: &str, next_markers: &[&str]) -> Option<(usize, usize)> {
    // Find the start: either BOF or after a newline.
    let target_bof = format!(
        "{heading}
"
    );
    let target = format!(
        "
{heading}
"
    );
    let start = if base.starts_with(&target_bof) {
        Some(0)
    } else {
        base.find(&target).map(|p| p + 1)
    }?;

    // Move to content start (after the heading line).
    let after_heading = start + heading.len();
    let content_start = base[after_heading..]
        .find("\n")
        .map(|off| after_heading + off + 1)
        .unwrap_or(after_heading);

    // Find the next marker of same/higher rank.
    let mut next = base.len();
    for m in next_markers {
        if let Some(p) = base[content_start..].find(m) {
            let idx = content_start + p + 1; // include preceding newline
            if idx < next {
                next = idx;
            }
        }
    }
    Some((start, next))
}

fn replace_range_with(base: &str, start: usize, end: usize, heading: &str, text: &str) -> String {
    let mut out = String::with_capacity(base.len() + text.len());
    out.push_str(&base[..start]);
    out.push_str(&format!("{heading}\n\n{}\n", text.trim_end()));
    // Ensure exactly one blank line before the next section
    // Skip any existing leading newlines in the remainder
    let mut j = end;
    let bytes = base.as_bytes();
    while j < bytes.len() && bytes[j] == b'\n' {
        j += 1;
    }
    out.push('\n');
    out.push_str(&base[j..]);
    out
}

fn merge_after_heading(base: &str, start: usize, end: usize, heading: &str, text: &str) -> String {
    // Keep heading, replace content with original content + text appended.
    let after_heading = start + heading.len();
    let content_start = base[after_heading..]
        .find("\n")
        .map(|off| after_heading + off + 1)
        .unwrap_or(after_heading);
    let mut out = String::with_capacity(base.len() + text.len());
    out.push_str(&base[..content_start]);
    out.push_str(&base[content_start..end]);
    if !text.trim().is_empty() {
        out.push_str("\n\n");
        out.push_str(text);
        out.push('\n');
    }
    out.push_str(&base[end..]);
    out
}

fn remove_range_normalized(base: &str, start: usize, end: usize) -> String {
    let prefix = &base[..start];
    let suffix = &base[end..];
    let prefix_trimmed = prefix.trim_end_matches("\n");
    let mut j = 0usize;
    let sb = suffix.as_bytes();
    while j < sb.len() && sb[j] == b'\n' {
        j += 1;
    }
    let suffix_trimmed = &suffix[j..];
    let mut out = String::with_capacity(prefix_trimmed.len() + suffix_trimmed.len() + 2);
    out.push_str(prefix_trimmed);
    if !prefix_trimmed.is_empty() && !suffix_trimmed.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(suffix_trimmed);
    out
}
