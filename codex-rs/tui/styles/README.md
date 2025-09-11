# Output Styles (YAML)

This folder contains built‑in output styles for Codex TUI. Styles are simple YAML documents that selectively override parts of the base system prompt without causing contradictions.

Styles are discovered at build time (see `tui/build.rs`) — any `*.yaml`/`*.yml` file here is automatically:
- Listed in the TUI “Select Output Style” popup.
- Available to the debug CLI (`codex debug output-style <name>`).

## File naming

- One file per style: `<name>.yaml` (e.g., `explanatory.yaml`).
- The `<name>` (filename stem) is the identifier shown in the UI and accepted by the debug command.

## YAML schema

```
kind: codex-style
version: 1
regions:
  personality:
    mode: replace | merge | disable
    text: |-
      ...
  presentation:
    mode: replace | merge | disable
    text: |-
      ...
  preambles:
    mode: replace | merge | disable
    text: |-
      ...
  planning:
    mode: replace | merge | disable
    text: |-
      ...
  progress:
    mode: replace | merge | disable
    text: |-
      ...

# Optional: override headings if the base prompt changes titles.
# When omitted, defaults are used and usually sufficient.
selectors:
  personality: { heading: "## Personality" }
  presentation: { heading: "## Presenting your work and final message" }
  preambles:   { heading: "### Preamble messages" }
  planning:    { heading: "## Planning" }
  progress:    { heading: "## Sharing progress updates" }
```

### Regions

- `personality`: Agent tone and high‑level persona.
- `presentation`: How answers are structured (headers, bullets, monospace, tone rules). This replaces the entire “Final answer structure and style guidelines” subtree.
- `preambles`: When/how to describe intent before executing commands/patches.
- `planning`: When and how to show/update step plans.
- `progress`: When/how to show interim progress updates.

### Modes

- `replace`: Replace the entire target section’s content.
- `merge`: Append your text to the end of the section (keeps the defaults).
- `disable`: Remove the section entirely.

The engine guarantees exactly one blank line between sections after replacement or merge, and normalizes any extra whitespace.

## Replacement strategy (robust)

- The engine finds sections by headings in the base prompt (H2/H3), not fragile phrases:
  - Personality: `## Personality` → next H2
  - Presentation: `## Presenting your work and final message` → next H2
  - Preambles: `### Preamble messages` → next H3/H2
  - Planning: `## Planning` → next H2
  - Progress: `## Sharing progress updates` → next H2
- If a selector heading is provided, that is used instead.
- If a heading is not found, the engine appends a new section at the end rather than silently failing.
- Other base sections (Capabilities, Tool Guidelines, Safety, etc.) are preserved.

## Good practices

- Put only style/structure guidance here — do not copy Tool Guidelines or other base content.
- Use `replace` when you want a clean override (recommended for `personality` and `presentation`).
- Use `merge` for incremental tweaks (e.g., add a sentence on when to show plans).
- Keep content concise to avoid re‑introducing contradictions across sections.

## Try it quickly

- List available styles:
  - `codex debug output-style`
- See the fully assembled prompt for a style:
  - `codex debug output-style verbose`
- See only certain sections (comma‑separated):
  - `codex debug output-style explanatory --sections personality,preambles,planning`

## How TUI uses styles

- The style picker in the TUI is dynamic — it lists files from this folder.
- Selecting a style creates a new session with that style applied.
- The current style name is shown in `/status` under “Client → Output Style”.
- “Default” means no YAML style is applied; the base prompt’s style rules are used.

## Examples

See the existing styles in this folder:
- `explanatory.yaml` — brief, high‑signal insights between steps.
- `learning.yaml` — collaborative, learn‑by‑doing with small TODO(human)s.
- `checklist.yaml` — checklist → changes → next steps; code‑first.
- `verbose.yaml` — detailed rationale, alternatives, and annotated code.

