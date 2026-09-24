//! PROTOTYPE — fake assistant turn state + rough view builders.
//! Mirrors the shape (not the fidelity) of `packages/tui/view.go`:
//! raw model state is kept, and every row is rebuilt from it at the
//! current width, which is what makes a resize cheap to satisfy.

use crate::flow::{wrap_plain, RESET};
use crate::partial_json::extract_partial_string_field;

pub const MUTED: &str = "\x1b[38;5;243m";
pub const FG: &str = "\x1b[38;5;252m";
pub const OK: &str = "\x1b[38;5;114m";

const MARGIN: usize = 2;
const PAD: usize = 1;
const MAX_PREVIEW_ROWS: usize = 14;

#[derive(Clone, Debug)]
pub enum StreamEvt {
    AssistantDelta(String),
    ToolStart { name: String, label: String },
    ToolJsonChunk(String),
    ToolEnd { summary: String },
    TurnDone,
}

#[derive(Clone, Debug, Default)]
pub struct TurnState {
    pub assistant: String,
    pub tool: Option<LiveTool>,
}

#[derive(Clone, Debug)]
pub struct LiveTool {
    pub name: String,
    pub label: String,
    pub raw: String,
    pub summary: Option<String>,
}

impl TurnState {
    pub fn apply(&mut self, ev: StreamEvt) {
        match ev {
            StreamEvt::AssistantDelta(s) => self.assistant.push_str(&s),
            StreamEvt::ToolStart { name, label } => {
                self.tool = Some(LiveTool {
                    name,
                    label,
                    raw: String::new(),
                    summary: None,
                })
            }
            StreamEvt::ToolJsonChunk(s) => {
                if let Some(t) = &mut self.tool {
                    t.raw.push_str(&s);
                }
            }
            StreamEvt::ToolEnd { summary } => {
                if let Some(t) = &mut self.tool {
                    t.summary = Some(summary);
                }
            }
            StreamEvt::TurnDone => {}
        }
    }
}

pub fn turn_lines(t: &TurnState, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    if !t.assistant.is_empty() {
        out.extend(wrap_plain(&t.assistant, width).into_iter().map(|l| {
            if l.is_empty() {
                l
            } else {
                format!("{FG}{l}{RESET}")
            }
        }));
    }
    if let Some(tool) = &t.tool {
        if !out.is_empty() {
            out.push(String::new());
        }
        let label = format!("{} {}", tool.name, tool.label);
        out.push(tool_box_top(&label, width));
        out.push(tool_box_side("", width));
        for body in tool_body_lines(tool, width) {
            out.push(tool_box_side(&body, width));
        }
        out.push(tool_box_side("", width));
        out.push(tool_box_bottom(width));
        if let Some(sum) = &tool.summary {
            out.push(format!("  {OK}\u{2713} {sum}{RESET}"));
        }
    }
    out
}

fn tool_body_lines(tool: &LiveTool, width: usize) -> Vec<String> {
    let w = width.saturating_sub(2 * MARGIN).max(12);
    let inner = w.saturating_sub(2 + 2 * PAD).max(8);
    let mut rows: Vec<String> = Vec::new();

    if tool.name == "write" {
        let (content, ok, _) = extract_partial_string_field(&tool.raw, "content");
        if ok && !content.is_empty() {
            for line in content.split('\n') {
                for wrapped in wrap_plain(line, inner) {
                    rows.push(format!("  {wrapped}"));
                }
            }
        } else {
            let tail: String = {
                let chars: Vec<char> = tool.raw.chars().collect();
                let start = chars.len().saturating_sub(400);
                chars[start..].iter().collect()
            };
            rows.extend(wrap_plain(&tail, inner).into_iter().map(|l| format!("  {l}")));
        }
    } else {
        rows.extend(wrap_plain(&tool.raw, inner).into_iter().map(|l| format!("  {l}")));
    }

    if rows.len() > MAX_PREVIEW_ROWS {
        rows.truncate(MAX_PREVIEW_ROWS - 1);
        rows.push("  \u{2026}".to_string());
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

pub fn bottom_lines(status: &str, cols: usize) -> Vec<String> {
    vec![
        crate::flow::truncate_plain(status, cols),
        crate::flow::truncate_plain("> ", cols),
    ]
}

pub fn tool_box_top(label: &str, width: usize) -> String {
    let w = width.saturating_sub(2 * MARGIN).max(12);
    let mut label: String = label.split_whitespace().collect::<Vec<_>>().join(" ");
    let prefix = "\u{250c}\u{2500}  ";
    let suffix = "  ";
    let mut used = prefix.chars().count() + label.chars().count() + suffix.chars().count();
    let mut fill = (w as isize - used as isize - 1).max(0) as usize;
    if (w as isize - used as isize - 1) < 0 {
        let over = used + 1 - w;
        let n = label.chars().count();
        if over + 3 < n {
            label = format!("{}...", label.chars().take(n - over - 3).collect::<String>());
        } else {
            label = "...".to_string();
        }
        used = prefix.chars().count() + label.chars().count() + suffix.chars().count();
        fill = (w as isize - used as isize - 1).max(0) as usize;
    }
    let margin = " ".repeat(MARGIN);
    let name = label.split(' ').next().unwrap_or("").to_string();
    let rest = label[name.len()..].to_string();
    let fill_str = "\u{2500}".repeat(fill);
    format!(
        "{margin}{MUTED}{prefix}{FG}{name}{MUTED}{rest}{suffix}{fill_str}\u{2510}{RESET}{margin}"
    )
}

pub fn tool_box_side(content: &str, width: usize) -> String {
    let w = width.saturating_sub(2 * MARGIN).max(12);
    let inner = w.saturating_sub(2 + 2 * PAD);
    let margin = " ".repeat(MARGIN);
    let left = format!("\u{2502}{}", " ".repeat(PAD));
    let right = format!("{}\u{2502}", " ".repeat(PAD));
    let stripped = content.strip_prefix("  ").unwrap_or(content);
    let (clipped, clipped_len) = if stripped.chars().count() > inner {
        let body: String = stripped.chars().take(inner.saturating_sub(1)).collect();
        (format!("{body}\u{2026}"), inner)
    } else {
        (stripped.to_string(), stripped.chars().count())
    };
    let pad_right = inner.saturating_sub(clipped_len);
    format!(
        "{margin}{MUTED}{left}{RESET}{clipped}{}{MUTED}{right}{RESET}{margin}",
        " ".repeat(pad_right)
    )
}

pub fn tool_box_bottom(width: usize) -> String {
    let w = width.saturating_sub(2 * MARGIN).max(12);
    let margin = " ".repeat(MARGIN);
    format!(
        "{margin}{MUTED}\u{2514}{}\u{2518}{RESET}{margin}",
        "\u{2500}".repeat(w.saturating_sub(2))
    )
}

fn delta_chunks(s: &str) -> Vec<StreamEvt> {
    chunk_str(s, 10)
        .into_iter()
        .map(StreamEvt::AssistantDelta)
        .collect()
}

fn chunk_str(s: &str, n: usize) -> Vec<String> {
    s.as_bytes()
        .chunks(n)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect()
}

/// Deterministic event stream used by both the interactive demo and the
/// scripted session. Sentinels asserted by tests: "addressable",
/// "reflows", "throttle", "partial", "wrote 9 lines".
pub fn script() -> Vec<StreamEvt> {
    let mut evs = Vec::new();
    let p1 = "The Go flow renderer keeps a logical buffer of chat rows plus the live bottom band. \
Committed rows scroll into native scrollback; only the rows still addressable in the viewport are \
diffed and repainted, and the renderer tracks its own viewport anchor so it never rewrites terminal history.";
    evs.extend(delta_chunks(p1));
    evs.push(StreamEvt::AssistantDelta("\n\n".to_string()));

    let p2 = "A resize is the hard case: the terminal reflows nothing for us, so the spike rebuilds \
every row from raw model state at the new width, then clears screen and scrollback and repaints once. \
Watching the viewport jump is exactly what this artifact exists to let you judge.";
    evs.extend(delta_chunks(p2));
    evs.push(StreamEvt::AssistantDelta("\n\n".to_string()));

    let p3 = "Each frame is throttled: the renderer bails out when neither the buffer nor the cursor \
changed, so an idle terminal keeps its native cursor blink instead of being repainted ten times a second.";
    evs.extend(delta_chunks(p3));
    evs.push(StreamEvt::AssistantDelta("\n\n".to_string()));

    evs.push(StreamEvt::ToolStart {
        name: "write".to_string(),
        label: "src/demo.rs".to_string(),
    });
    let json = r#"{"path":"src/demo.rs","content":"fn main() {\n    let anchor = \"viewport\";\n    let rows = [\"chat\", \"band\"];\n    println!(\"rendering {}\", anchor);\n    for row in rows {\n        println!(\"row {row}\");\n    }\n}\n"}"#;
    for chunk in chunk_str(json, 12) {
        evs.push(StreamEvt::ToolJsonChunk(chunk));
    }
    evs.push(StreamEvt::ToolEnd {
        summary: "wrote 9 lines to src/demo.rs".to_string(),
    });
    evs.push(StreamEvt::AssistantDelta(
        "\nThe tool box above was streamed from partial JSON; the final summary lands once the call completes.\n"
            .to_string(),
    ));
    evs.push(StreamEvt::TurnDone);
    evs
}
