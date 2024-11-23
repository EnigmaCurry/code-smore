use crate::morse::play;
use rand::prelude::SliceRandom;

use crossterm::{
    cursor,
    event::{self, read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use textwrap::wrap;

pub fn start_quiz(trials: u32, char_set: &str, dot_duration: u32, tone_freq: f32, cheat: bool) {
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
                        println!("\nQuiz terminated.");
                        if let Err(e) = disable_raw_mode() {
                            eprintln!("Error disabling raw mode: {}", e);
                        }
                        return;
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
    // Disable raw mode
    if let Err(e) = disable_raw_mode() {
        eprintln!("Error disabling raw mode: {}", e);
    }

    let results = reaction_time_quiz(char_set, trials, dot_duration, tone_freq, cheat);
    print_results(&results, Duration::from_millis(dot_duration.into()));
}

struct QuizResult {
    prompts: Vec<char>,
    responses: Vec<bool>,
    reaction_times: Vec<Duration>,
}

fn reaction_time_quiz(
    char_set: &str,
    trials: u32,
    dot_duration: u32,
    tone_freq: f32,
    cheat: bool,
) -> QuizResult {
    let mut prompts = Vec::new();
    let mut responses = Vec::new();
    let mut reaction_times = Vec::new();

    let mut stdout = stdout();

    // Enable raw mode to capture key presses without Enter
    if let Err(e) = enable_raw_mode() {
        eprintln!("Error enabling raw mode: {}", e);
        return QuizResult {
            prompts,
            responses,
            reaction_times,
        };
    }

    let mut rng = rand::thread_rng();

    for _ in 0..trials {
        // Generate a random letter from the char set
        let target_letter = char_set.chars().collect::<Vec<_>>();
        let target_letter = target_letter
            .choose(&mut rng)
            .expect("Could not generate random character");
        prompts.push(*target_letter);
    }
    for i in 0..trials {
        std::thread::sleep(Duration::from_millis(500));
        let target_letter: char = prompts[i as usize];
        // Clear the screen and display the letter
        stdout.execute(Clear(ClearType::All)).unwrap();
        stdout.execute(cursor::MoveTo(0, 0)).unwrap();
        if cheat {
            print!("Type the letter: ");
            stdout.flush().unwrap();
        }

        play(&target_letter.to_string(), dot_duration, tone_freq);
        if cheat {
            println!("{}", target_letter);
            stdout.flush().unwrap();
        }

        // Start the timer
        let start_time = Instant::now();

        let mut is_correct = false;
        let mut key_processed = false; // Ensure unique processing per key press

        // Wait for user input (key press followed by release)
        loop {
            if let Ok(Event::Key(event)) = event::read() {
                match event.kind {
                    crossterm::event::KeyEventKind::Press if !key_processed => {
                        if let KeyCode::Char(input_char) = event.code {
                            is_correct = input_char.to_ascii_uppercase()
                                == target_letter.to_ascii_uppercase();
                            key_processed = true; // Block further processing until release
                        }
                        if event.code == KeyCode::Esc {
                            disable_raw_mode().unwrap();
                            println!("\nQuiz terminated.");
                            return QuizResult {
                                prompts,
                                responses,
                                reaction_times,
                            };
                        }
                    }
                    crossterm::event::KeyEventKind::Release => {
                        key_processed = false; // Allow next key press
                    }
                    _ => {}
                }

                // Exit loop after processing a valid key
                if key_processed {
                    break;
                }
            }
        }

        // Stop the timer
        let elapsed = start_time.elapsed();
        reaction_times.push(elapsed);

        responses.push(is_correct);
    }

    // Disable raw mode after the quiz
    if let Err(e) = disable_raw_mode() {
        eprintln!("Error disabling raw mode: {}", e);
    }

    QuizResult {
        prompts,
        responses,
        reaction_times,
    }
}

fn print_results(results: &QuizResult, dot_duration: Duration) {
    let total = results.prompts.len();
    let correct = results.responses.iter().filter(|&&r| r).count();
    let incorrect = total - correct;

    let total_time: Duration = results.reaction_times.iter().sum();
    let average_time = if total > 0 {
        total_time / total as u32
    } else {
        Duration::default()
    };

    let correct_times: Vec<_> = results
        .reaction_times
        .iter()
        .zip(results.responses.iter())
        .filter_map(|(&time, &is_correct)| if is_correct { Some(time) } else { None })
        .collect();

    let incorrect_times: Vec<_> = results
        .reaction_times
        .iter()
        .zip(results.responses.iter())
        .filter_map(|(&time, &is_correct)| if !is_correct { Some(time) } else { None })
        .collect();

    let average_correct_time = if !correct_times.is_empty() {
        correct_times.iter().sum::<Duration>() / correct_times.len() as u32
    } else {
        Duration::default()
    };

    let average_incorrect_time = if !incorrect_times.is_empty() {
        incorrect_times.iter().sum::<Duration>() / incorrect_times.len() as u32
    } else {
        Duration::default()
    };

    // Assign a grade based on correctness and reaction speed
    let percentage_correct = (correct as f64 / total as f64) * 100.0;
    let speed_score = if average_correct_time <= dot_duration {
        "Excellent"
    } else if average_correct_time <= dot_duration * 2 {
        "Good"
    } else {
        "Needs Improvement"
    };

    let grade = match (percentage_correct, speed_score) {
        (90.0..=100.0, "Excellent") => "A+",
        (90.0..=100.0, _) => "A",
        (80.0..=89.9, _) => "B",
        (70.0..=79.9, _) => "C",
        (60.0..=69.9, _) => "D",
        _ => "F",
    };

    println!("\nResults:");
    println!("You got {}/{} correct!", correct, total);
    println!("Number incorrect: {}", incorrect);
    println!("Average reaction time: {:.2?}", average_time);
    println!(
        "Average correct reaction time: {:.2?}",
        average_correct_time
    );
    println!(
        "Average incorrect reaction time: {:.2?}",
        average_incorrect_time
    );
    println!("Total reaction time: {:.2?}", total_time);
    println!(
        "\nYour grade: {}
Speed Rating: {}",
        grade, speed_score
    );

    match grade {
        "A+" => println!("Phenomenal! You nailed both speed and accuracy."),
        "A" => println!("Excellent work! A little faster and you'll be perfect."),
        "B" => println!("Great job! Keep honing your skills."),
        "C" => println!("Good effort! Practice to improve both speed and accuracy."),
        "D" => println!("Keep at it! You can do better with more focus."),
        "F" => println!("Don't give up! Consistency and practice will help."),
        _ => (),
    }
}
