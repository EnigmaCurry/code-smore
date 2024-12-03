use crate::message::Message;

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use std::io::stdout;

pub fn clear_screen() {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
}

pub fn log_message(message: &Message) {
    // Get the terminal dimensions
    let terminal_width = term_size::dimensions().map_or(80, |(w, _)| w);

    // Calculate the wrapping width (2/3 of terminal width)
    let wrap_width = (terminal_width as f32 * 2.0 / 3.0) as usize;

    // Split the message content into wrapped lines
    let mut wrapped_lines = vec![];
    let mut current_line = String::new();

    for word in message.content.split_whitespace() {
        if current_line.len() + word.len() + 1 > wrap_width {
            wrapped_lines.push(current_line);
            current_line = String::new();
        }
        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    if !current_line.is_empty() {
        wrapped_lines.push(current_line);
    }

    // Print the first line with the timestamp aligned to the right
    if let Some(first_line) = wrapped_lines.first() {
        let padding = terminal_width.saturating_sub(first_line.len() + message.timestamp.len());
        let spaces = " ".repeat(padding);
        println!("{}{}{}", first_line, spaces, message.timestamp);
    }

    // Print the rest of the wrapped lines
    for line in &wrapped_lines[1..] {
        println!("{}", line);
    }

    // Print an empty line at the end
    println!();
}
