// src/transeive.rs

use crate::{alsa::listen_with_alsa, morse::MorsePlayer};
use chrono::Local;
use crossterm::{
    cursor::{Hide, MoveTo},
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::io::Stdout;
use std::{
    io::{stdout, Write},
    sync::mpsc,
    thread,
    time::Duration,
};

fn redraw_log(stdout: &mut Stdout, log: &Vec<String>, max_log_rows: usize) {
    execute!(stdout, MoveTo(0, 0), Clear(ClearType::FromCursorDown)).unwrap();
    let start = log.len().saturating_sub(max_log_rows);
    for (i, line) in log.iter().skip(start).enumerate() {
        execute!(stdout, MoveTo(0, i as u16)).unwrap();
        print!("{line}");
    }
}

/// A “half-duplex” TUI: top area is the log, one row from the bottom is
/// the “preview” (updated character by character), and the bottom row
/// is the user’s input prompt.
///
/// Incoming `:typing:` messages overwrite the last log line in place.
/// A final (non-`:typing:`) line replaces that preview and becomes “complete.”
pub fn run_transeiver(
    tone_freq: f32,
    dot_duration: u32,
    sound_device: &str,
    rts_port: Option<&str>,
    rigctl_port: Option<&str>,
    rigctl_model: Option<&str>,
) {
    // ─── 1) Enter Alternate Screen & Raw Mode ─────────────────────────────────
    let mut stdout = stdout();
    enable_raw_mode().expect("Failed to enable raw mode");
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide).unwrap();

    // ─── 2) Spawn listener thread (ALSA) with a Sender<String> ───────────────
    let (tx, rx) = mpsc::channel::<String>();
    let dev = sound_device.to_string();
    thread::spawn(move || {
        // The `listen_with_alsa` is expected to send:
        //   • preview updates as ":typing:partial_text"
        //   • final text as "COMPLETE_TEXT" (no prefix)
        let _ = listen_with_alsa(&dev, tone_freq, 200.0, 0.3, dot_duration, false, Some(tx));
    });

    // ─── 3) Shared UI state ───────────────────────────────────────────────────
    let player = MorsePlayer::new();
    let mut input = String::new();

    // `log` holds each _completed_ message. When building is true, the
    // last element of `log` is being overwritten by previews.
    let mut log: Vec<String> = Vec::new();
    let mut building = false; // true if a preview has arrived but not yet finalized
    let mut last_preview = String::new(); // the raw preview text (e.g. "AI7XP T")

    // We’ll track how many lines we’ve drawn so far, so we only clear/redraw when needed:
    let mut last_log_len = 0;

    loop {
        // ─── 4) Compute layout rows ────────────────────────────────────────────
        let (_term_width, term_height) = crossterm::terminal::size().unwrap();
        let input_row = term_height - 1; // bottom row for user input
        let preview_row = input_row - 1; // one row above input for preview
        let max_log_rows = preview_row as usize; // rows available for “log” (0..preview_row-1)

        // ─── 5) Drain `rx`: process each incoming string ───────────────────────
        // If it starts with ":typing:", treat as a preview. Otherwise, it’s final text.
        let mut need_full_log_redraw = false;
        while let Ok(raw) = rx.try_recv() {
            if let Some(partial) = raw.strip_prefix(":typing:") {
                last_preview = partial.to_string();
                building = true;
            } else {
                // ─── 5.b) Final message ───────────────────────────────
                // If we were building, overwrite that same last entry:
                let timestamp = Local::now().format("%H:%M:%S %Z").to_string();
                log.push(format!("[{timestamp}] > {raw}"));
                building = false;
                last_preview.clear();
                need_full_log_redraw = true;
            }
        }

        // ─── 6) Redraw the “log” area if needed ───────────────────────────────
        let current_log_len = log.len();
        if need_full_log_redraw || current_log_len != last_log_len {
            redraw_log(&mut stdout, &log, max_log_rows);
            last_log_len = log.len();
        }

        // ─── 7) Redraw the “preview” row unconditionally on change ────────────
        // Always clear that single line, then reprint if building. No newline.
        execute!(
            stdout,
            MoveTo(0, preview_row as u16),
            Clear(ClearType::CurrentLine)
        )
        .unwrap();
        if building && !last_preview.is_empty() {
            print!("\x1b[2m[...]{last_preview}\x1b[22m");
        }

        // ─── 8) Redraw the input prompt (bottom row) ─────────────────────────
        execute!(
            stdout,
            MoveTo(0, input_row as u16),
            Clear(ClearType::CurrentLine)
        )
        .unwrap();
        print!("< {input}");
        stdout.flush().unwrap();

        // ─── 9) Handle user keystrokes ────────────────────────────────────────
        if poll(Duration::from_millis(50)).unwrap() {
            if let Event::Key(key_event) = read().unwrap() {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Exit on Ctrl-C
                        break;
                    }
                    KeyCode::Esc => {
                        // Exit on ESC
                        break;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        let message = input.trim();
                        if !message.is_empty() {
                            execute!(
                                stdout,
                                MoveTo(0, preview_row as u16),
                                Clear(ClearType::CurrentLine)
                            )
                            .unwrap();
                            print!("\x1b[2m[...sending \"{message}\"]\x1b[22m");
                            stdout.flush().unwrap();

                            player.play(
                                message,
                                dot_duration,
                                tone_freq,
                                rts_port,
                                rigctl_port,
                                rigctl_model,
                            );

                            let timestamp = Local::now().format("%H:%M:%S %Z").to_string();
                            log.push(format!("[{timestamp}] < {message}"));
                            redraw_log(&mut stdout, &log, max_log_rows);
                            last_log_len = log.len(); // force re-render if the log fills up
                        }
                        input.clear();
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    _ => {}
                }
            }
        }
    }

    // ─── 10) Cleanup: leave alternate screen, restore normal mode ───────────
    disable_raw_mode().unwrap();
    execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();
}
