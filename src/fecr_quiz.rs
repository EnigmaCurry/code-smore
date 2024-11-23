use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use textwrap::wrap;

pub fn start_quiz(trials: u32, _character_set: &str) {
    let paragraph = format!("Fast Enough Character Recognition quiz.\n\nMorse encoded characters will be played back to you one at a time and you must type the character you hear as soon as you recognize it.\n\nThis test will include {trials} trials. You will be timed in your response. You may stop the quiz at any time by pressing the ESC key.\n\nTo begin the quiz press the Enter key.");

    for line in wrap(&paragraph, 70) {
        println!("{}", line);
    }

    // Enable raw mode to capture key presses
    if let Err(e) = enable_raw_mode() {
        eprintln!("Error enabling raw mode: {}", e);
        return;
    }

    loop {
        match event::read() {
            Ok(Event::Key(key_event)) => {
                match key_event.code {
                    KeyCode::Enter => {
                        // Start the quiz
                        println!("Begin!");
                        break;
                    }
                    KeyCode::Esc => {
                        // Exit the quiz
                        println!("Exiting the quiz...");
                        break;
                    }
                    _ => {}
                }
            }
            Ok(_e) => {
                //eprintln!("Unknown event");
            }
            Err(e) => {
                eprintln!("Error reading event: {}", e);
                break;
            }
        }
    }

    // Disable raw mode after the loop
    if let Err(e) = disable_raw_mode() {
        eprintln!("Error disabling raw mode: {}", e);
    }

    println!("\nExiting...");

    // Here, you can add the logic for the quiz after the Enter key is pressed
}
