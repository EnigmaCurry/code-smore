use crate::{alsa::listen_with_alsa, morse::MorsePlayer};
use chrono::Local;
use crossterm::{
    cursor::MoveTo,
    event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::{
    io::{stdout, Write},
    sync::mpsc::{self},
    thread,
    time::Duration,
};

pub fn run_transeiver(
    tone_freq: f32,
    dot_duration: u32,
    sound_device: &str,
    rts_port: Option<&str>,
    rigctl_port: Option<&str>,
    rigctl_model: Option<&str>,
) {
    let mut stdout = stdout();
    enable_raw_mode().expect("Failed to enable raw mode");
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        Clear(ClearType::All),
        MoveTo(0, 0)
    )
    .unwrap();

    let (tx, rx) = mpsc::channel::<String>();
    let tx_clone = tx.clone();

    // Spawn receiver thread
    let dev = sound_device.to_string();
    thread::spawn(move || {
        let _ = listen_with_alsa(
            &dev,
            tone_freq,
            200.0, // default bandwidth
            0.3,   // default threshold
            dot_duration,
            true, // output as morse code
            Some(tx_clone),
        );
    });

    let player = MorsePlayer::new();
    let mut input = String::new();
    let mut log: Vec<String> = Vec::new();

    loop {
        // Drain any received messages
        while let Ok(line) = rx.try_recv() {
            let timestamp = Local::now().format("%H:%M:%S").to_string();
            log.push(format!("[{timestamp}] {}", line));
        }

        // Redraw screen
        execute!(stdout, MoveTo(0, 0), Clear(ClearType::All)).unwrap();
        let max_lines = crossterm::terminal::size().unwrap().1.saturating_sub(2) as usize;
        let start = log.len().saturating_sub(max_lines);
        for (i, line) in log.iter().skip(start).enumerate() {
            execute!(stdout, MoveTo(0, i as u16)).unwrap();
            println!("{}", line);
        }

        // Show prompt
        let bottom = crossterm::terminal::size().unwrap().1 - 1;
        execute!(stdout, MoveTo(0, bottom), Clear(ClearType::CurrentLine)).unwrap();
        print!("> {}", input);
        stdout.flush().unwrap();

        // Handle input
        if let Event::Key(key_event) = read().unwrap() {
            match key_event.code {
                KeyCode::Char(c) => input.push(c),
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Enter => {
                    let message = input.trim();
                    if !message.is_empty() {
                        player.play(
                            message,
                            dot_duration,
                            tone_freq,
                            rts_port,
                            rigctl_port,
                            rigctl_model,
                        );
                    }
                    input.clear();
                }
                KeyCode::Esc => break,
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(20));
    }

    // Clean up
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
