//! PROTOTYPE — tui-spike for wayfinder ticket 02.
//!
//! Question under test: can a crossterm-only renderer (crossterm as the
//! platform/input layer, a ported custom flow renderer for output)
//! reproduce agente's inline streaming UX — including a mid-stream
//! resize with no duplicated transcript — and can it be verified
//! without a real terminal?
//!
//! Run:
//!   cargo run -- demo     interactive spike (resize the window mid-stream)
//!   cargo run -- script   headless scripted session + emulator report
//!   cargo test            assertions (idle no-op, append diff, resize)

mod flow;
mod partial_json;
mod screen;
mod turn;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use flow::FlowRenderer;
use screen::Screen;
use std::io::Write;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use turn::{bottom_lines, turn_lines, StreamEvt, TurnState};

const SPIN: [&str; 10] = [
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280f}",
];

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "demo".to_string());
    match arg.as_str() {
        "demo" => demo(),
        "script" => script_report(),
        other => {
            eprintln!("usage: tui-spike [demo|script] (got {other:?})");
            std::process::exit(2);
        }
    }
}

fn size_or_default() -> (usize, usize) {
    terminal::size()
        .map(|(c, r)| (c as usize, r as usize))
        .unwrap_or((80, 24))
}

fn build_chat(committed: &[TurnState], current: &TurnState, cols: usize) -> Vec<String> {
    let mut chat = Vec::new();
    for t in committed {
        chat.extend(turn_lines(t, cols));
        chat.push(String::new());
    }
    chat.extend(turn_lines(current, cols));
    chat
}

fn paint(
    renderer: &mut FlowRenderer<Vec<u8>>,
    committed: &[TurnState],
    current: &TurnState,
    cols: usize,
    status: &str,
) -> Vec<u8> {
    let chat = build_chat(committed, current, cols);
    let bottom = bottom_lines(status, cols);
    renderer.draw_log(&chat, &bottom, 1, 2);
    renderer.take_output()
}

fn chunk_delay(ev: &StreamEvt) -> Duration {
    match ev {
        StreamEvt::AssistantDelta(_) => Duration::from_millis(14),
        StreamEvt::ToolJsonChunk(_) => Duration::from_millis(40),
        StreamEvt::ToolStart { .. } | StreamEvt::ToolEnd { .. } => Duration::from_millis(160),
        StreamEvt::TurnDone => Duration::from_millis(0),
    }
}

fn demo() {
    if terminal::enable_raw_mode().is_err() {
        eprintln!(
            "tui-spike demo needs a real terminal (raw mode failed). \
Run `cargo run -- script` for the headless report instead."
        );
        std::process::exit(1);
    }
    let real_size = size_or_default();
    let (mut cols, mut rows) = real_size;
    let keep_scrollback = std::env::var("TERM_PROGRAM")
        .map(|v| v == "vscode")
        .unwrap_or(false);
    let mut renderer = FlowRenderer::new(std::io::stdout(), keep_scrollback);
    renderer.resize(cols, rows);

    let (ev_tx, ev_rx) = channel::<StreamEvt>();
    let (go_tx, go_rx) = channel::<()>();
    std::thread::spawn(move || loop {
        if go_rx.recv().is_err() {
            break;
        }
        for ev in turn::script() {
            std::thread::sleep(chunk_delay(&ev));
            if ev_tx.send(ev).is_err() {
                return;
            }
        }
    });
    let _ = go_tx.send(());

    let mut committed: Vec<TurnState> = Vec::new();
    let mut current = TurnState::default();
    let mut streaming = true;
    let mut spinner = 0usize;
    let mut last_spin = Instant::now();
    let mut hist = false;
    let mut size_cycle = 0usize;
    let mut quit = false;

    while !quit {
        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            loop {
                match event::read() {
                    Ok(Event::Resize(c, r)) => {
                        cols = c as usize;
                        rows = r as usize;
                        renderer.resize(cols, rows);
                    }
                    Ok(Event::Key(k))
                        if matches!(k.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                    {
                        let ctrl_c = k.code == KeyCode::Char('c')
                            && k.modifiers.contains(KeyModifiers::CONTROL);
                        if ctrl_c || k.code == KeyCode::Char('q') {
                            quit = true;
                        } else if k.code == KeyCode::Char(' ') {
                            if !streaming {
                                let _ = go_tx.send(());
                                streaming = true;
                                current = TurnState::default();
                                last_spin = Instant::now();
                            }
                        } else if k.code == KeyCode::Char('r') {
                            // Synthetic resize for terminals where dragging
                            // the window is awkward: cycle assumed widths.
                            size_cycle = (size_cycle + 1) % 4;
                            let w = match size_cycle {
                                0 => real_size.0,
                                1 => 60,
                                2 => 100,
                                _ => 40,
                            };
                            cols = w;
                            rows = real_size.1;
                            renderer.resize(cols, rows);
                        } else if k.code == KeyCode::Char('h') {
                            if hist {
                                renderer.close_history();
                                hist = false;
                            } else {
                                let chat = build_chat(&committed, &current, cols);
                                renderer.draw_history(&chat, -1, 0);
                                hist = true;
                            }
                        }
                    }
                    _ => {}
                }
                if !event::poll(Duration::from_millis(0)).unwrap_or(false) {
                    break;
                }
            }
        }

        while let Ok(ev) = ev_rx.try_recv() {
            match ev {
                StreamEvt::TurnDone => {
                    committed.push(std::mem::take(&mut current));
                    streaming = false;
                }
                other => current.apply(other),
            }
        }

        if streaming && last_spin.elapsed() >= Duration::from_millis(80) {
            spinner += 1;
            last_spin = Instant::now();
        }

        if !hist {
            let status = if streaming {
                format!(
                    "{} streaming \u{b7} size {}x{} \u{b7} q quit",
                    SPIN[spinner % SPIN.len()],
                    cols,
                    rows
                )
            } else {
                format!(
                    "idle \u{b7} space: new turn \u{b7} r: cycle width \u{b7} h: history \u{b7} q: quit \u{b7} size {}x{}",
                    cols, rows
                )
            };
            let chat = build_chat(&committed, &current, cols);
            let bottom = bottom_lines(&status, cols);
            renderer.draw_log(&chat, &bottom, 1, 2);
        }
    }

    let _ = renderer.close_history();
    let mut out = std::io::stdout();
    let _ = out.write_all(flow::SHOW_CURSOR.as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = terminal::disable_raw_mode();
}

struct ScriptReport {
    cols: usize,
    rows: usize,
    paints: usize,
    full_paints: usize,
    idle_bytes: usize,
    purges: usize,
    scrollback_len: usize,
    scrollback_at_resize: usize,
    visible: String,
    all: String,
    bytes: usize,
    resize_at_event: usize,
    resize_size: (usize, usize),
}

fn run_scripted(cols0: usize, rows0: usize, resize_offset_after_tool_end: usize) -> ScriptReport {
    let mut renderer = FlowRenderer::new(Vec::<u8>::new(), false);
    renderer.resize(cols0, rows0);
    let _ = renderer.take_output();
    let mut screen = Screen::new(cols0, rows0);
    let mut committed: Vec<TurnState> = Vec::new();
    let mut current = TurnState::default();
    let mut cols = cols0;
    let mut rows = rows0;
    let events = turn::script();
    // Resize mid-stream, after the tool box has fully streamed and while
    // the final assistant delta is still arriving.
    let tool_end = events
        .iter()
        .position(|e| matches!(e, StreamEvt::ToolEnd { .. }))
        .unwrap_or(events.len() / 2);
    let resize_at =
        (tool_end + resize_offset_after_tool_end).min(events.len().saturating_sub(2));
    let resize_size = (60usize, 20usize);
    let mut resized = false;
    let mut scrollback_at_resize = 0usize;
    let mut bytes = 0usize;

    {
        let out = paint(&mut renderer, &committed, &current, cols, "streaming\u{2026}");
        bytes += out.len();
        screen.feed(&out);
    }

    for (i, ev) in events.into_iter().enumerate() {
        if !resized && i == resize_at {
            scrollback_at_resize = screen.scrollback_len();
            cols = resize_size.0;
            rows = resize_size.1;
            renderer.resize(cols, rows);
            screen.set_size(cols, rows);
            resized = true;
            let out = paint(
                &mut renderer,
                &committed,
                &current,
                cols,
                "streaming\u{2026} (resized)",
            );
            bytes += out.len();
            screen.feed(&out);
        }
        let done = matches!(ev, StreamEvt::TurnDone);
        match ev {
            StreamEvt::TurnDone => committed.push(std::mem::take(&mut current)),
            other => current.apply(other),
        }
        let status = if done { "idle" } else { "streaming\u{2026}" };
        let out = paint(&mut renderer, &committed, &current, cols, status);
        bytes += out.len();
        screen.feed(&out);
    }

    let mut idle_bytes = 0usize;
    for _ in 0..2 {
        let out = paint(&mut renderer, &committed, &current, cols, "idle");
        idle_bytes += out.len();
    }

    ScriptReport {
        cols,
        rows,
        paints: renderer.paints,
        full_paints: renderer.full_paints,
        idle_bytes,
        purges: screen.purges,
        scrollback_len: screen.scrollback_len(),
        scrollback_at_resize,
        visible: screen.text(),
        all: screen.all(),
        bytes,
        resize_at_event: resize_at,
        resize_size,
    }
}

fn script_report() {
    let rep = run_scripted(80, 24, 8);
    println!("== tui-spike headless session ==");
    println!(
        "start 80x24, mid-stream resize at event {} to {}x{}",
        rep.resize_at_event, rep.resize_size.0, rep.resize_size.1
    );
    println!(
        "frames painted: {} (full repaints: {})",
        rep.paints, rep.full_paints
    );
    println!(
        "bytes emitted across paints: {}; idle repaint bytes (unchanged state, 2 draws): {}",
        rep.bytes, rep.idle_bytes
    );
    println!(
        "scrollback rows at resize: {}; scrollback purges: {}; final scrollback rows: {} (final size {}x{})",
        rep.scrollback_at_resize, rep.purges, rep.scrollback_len, rep.cols, rep.rows
    );
    println!("sentinel occurrences in scrollback+screen (1 = no duplicate transcript):");
    for needle in ["addressable", "reflows", "throttle", "partial", "wrote 9 lines"] {
        println!("  {needle:?}: {}", rep.all.matches(needle).count());
    }
    println!("final visible screen:");
    println!("{}", rep.visible);
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow::wrap_plain;
    use partial_json::extract_partial_string_field;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn idle_draw_emits_nothing() {
        let mut r = FlowRenderer::new(Vec::<u8>::new(), false);
        r.resize(40, 10);
        let _ = r.take_output();
        let chat = lines(&["hello"]);
        let bottom = lines(&["status"]);
        r.draw_log(&chat, &bottom, 1, 2);
        assert!(!r.take_output().is_empty());
        r.draw_log(&chat, &bottom, 1, 2);
        assert!(r.take_output().is_empty(), "unchanged draw must be a no-op");
    }

    #[test]
    fn append_patches_less_than_a_full_repaint() {
        let mut r = FlowRenderer::new(Vec::<u8>::new(), false);
        r.resize(60, 12);
        let _ = r.take_output();
        let chat1: Vec<String> = (0..40).map(|i| format!("line {i}")).collect();
        let bottom = lines(&["status"]);
        r.draw_log(&chat1, &bottom, 1, 2);
        let full = r.take_output();
        let mut chat2 = chat1.clone();
        chat2.push("line 40".to_string());
        r.draw_log(&chat2, &bottom, 1, 2);
        let inc = r.take_output();
        assert!(inc.len() < full.len(), "append should not repaint everything");
        assert!(String::from_utf8_lossy(&inc).contains("line 40"));
    }

    #[test]
    fn resize_mid_stream_has_no_duplicate_transcript() {
        let rep = run_scripted(80, 24, 8);
        assert_eq!(rep.idle_bytes, 0, "idle repaint emitted bytes");
        assert!(rep.full_paints >= 2, "expected initial + post-resize full paints");
        assert!(
            rep.scrollback_at_resize > 0,
            "test is only meaningful if rows had scrolled before the resize"
        );
        assert!(rep.purges >= 1, "resize must purge stale scrollback");
        for needle in ["addressable", "reflows", "throttle", "partial"] {
            assert_eq!(
                rep.all.matches(needle).count(),
                1,
                "transcript contains {needle:?} more than once after resize:\n{}",
                rep.all
            );
        }
        assert!(
            rep.all.contains("wrote 9 lines"),
            "tool summary missing:\n{}",
            rep.all
        );
    }

    #[test]
    fn wrap_is_word_aware() {
        assert_eq!(
            wrap_plain("hello world this is a test", 10),
            vec!["hello", "world this", "is a test"]
        );
    }

    #[test]
    fn partial_json_extracts_streaming_content() {
        let raw = r#"{"path":"a.rs","content":"fn main() {\n    println!(\"hi\");"#;
        let (v, ok, done) = extract_partial_string_field(raw, "content");
        assert!(ok);
        assert!(!done);
        assert_eq!(v, "fn main() {\n    println!(\"hi\");");
    }

    #[test]
    fn partial_json_drops_incomplete_unicode_escape() {
        let raw = r#"{"content":"a\u00"#;
        let (v, ok, done) = extract_partial_string_field(raw, "content");
        assert!(ok);
        assert!(!done);
        assert_eq!(v, "a");
    }
}
