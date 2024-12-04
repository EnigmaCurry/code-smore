use crate::morse::MorsePlayer;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use tabled::settings::style::Style;
use tabled::{Table, Tabled};
use textwrap::wrap;

#[allow(clippy::too_many_arguments)]
pub fn start_quiz(
    trials: u32,
    char_set: &str,
    dot_duration: u32,
    tone_freq: f32,
    text: bool,
    randomize: bool,
    calibration: bool,
    baseline: u32,
) {
    let paragraph = match calibration {
        true => "Calibration process.\n\nThis process will measure your native keyboard typing skills to calculate your personal output latency. A series of characters will be displayed at the same time a tone is played. Enter the characters as fast as you can.\n".to_string(),
        false => format!("Fast Enough Character Recognition quiz.\n\nMorse encoded characters will be played back to you one at a time and you must type the character you hear as soon as you recognize it.\n\nThis test will include {trials} trials. You will be timed in your response. Your reaction time is subtracted from the baseline input latency of {baseline}ms.\n")
    };

    for line in wrap(&paragraph, 70) {
        println!("{}", line);
    }
    let player = MorsePlayer::new();

    if calibration {
    } else {
        println!("Initializing audio (VVV) ...");
        player.play("VVV", dot_duration, tone_freq);
    }

    if calibration {
        println!("\nYou may stop the calibration at any time by pressing the ESC key.\nTo begin the calibration press the Enter key.");
    } else {
        println!("\nYou may stop the quiz at any time by pressing the ESC key.\nTo begin the quiz press the Enter key.");
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
                        if calibration {
                            println!("\nCalibration process terminated.");
                        } else {
                            println!("\nQuiz terminated.");
                        }
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

    let results = reaction_time_quiz(
        &player,
        char_set,
        trials,
        dot_duration,
        tone_freq,
        text,
        randomize,
        calibration,
        if calibration { 0 } else { baseline },
    );
    print_results(
        &results,
        Duration::from_millis(dot_duration.into()),
        calibration,
        if calibration { 0 } else { baseline },
    );
}

struct QuizResult {
    prompts: Vec<char>,
    responses: Vec<Option<bool>>,
    reaction_times: Vec<Option<Duration>>,
}

#[allow(clippy::too_many_arguments)]
fn reaction_time_quiz(
    player: &MorsePlayer,
    char_set: &str,
    trials: u32,
    dot_duration: u32,
    tone_freq: f32,
    text: bool,
    randomize: bool,
    calibration: bool,
    baseline: u32,
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

    if randomize {
        for _ in 0..trials {
            // Generate a random letter from the char set
            let target_letter = char_set.chars().collect::<Vec<_>>();
            let target_letter = target_letter
                .choose(&mut rng)
                .expect("Could not generate random character");
            prompts.push(*target_letter);
        }
    } else {
        let mut target_letters = char_set.chars().collect::<Vec<_>>();
        target_letters.shuffle(&mut rng);
        for i in 0..trials {
            prompts.push(target_letters[i as usize % target_letters.len()]);
        }
        prompts.shuffle(&mut rng);
    }
    for i in 0..trials {
        std::thread::sleep(Duration::from_millis(500));
        let target_letter: char = prompts[i as usize];
        // Clear the screen and display the letter
        stdout.execute(Clear(ClearType::All)).unwrap();
        stdout.execute(cursor::MoveTo(0, 0)).unwrap();
        if text || calibration {
            print!("Type the letter:");
            stdout.flush().unwrap();
        }

        if calibration {
            player.play_nonblocking_tone(dot_duration, tone_freq);
        } else {
            player.play(&target_letter.to_string(), dot_duration, tone_freq);
        }

        if text || calibration {
            println!(" {target_letter}");
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
                            // Truncate prompts to the same size as `responses`:
                            if responses.len() < prompts.len() {
                                prompts.truncate(responses.len());
                            }
                            if reaction_times.len() > responses.len() {
                                reaction_times.truncate(responses.len());
                            }
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
        let baseline_duration = Duration::from_millis(baseline.into());
        let clamped_duration = if elapsed > baseline_duration {
            elapsed - baseline_duration
        } else {
            Duration::from_millis(0)
        };
        reaction_times.push(Some(clamped_duration));

        responses.push(Some(is_correct));
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

#[derive(Tabled)]
struct SummaryRow {
    character: char,
    count: u32,
    avg_correct_time: String,
    avg_incorrect_time: String,
    times_correct: u32,
    times_incorrect: u32,
}

fn print_results(results: &QuizResult, dot_duration: Duration, calibration: bool, baseline: u32) {
    println!("\nTest complete!\n");
    let total = results.prompts.len();
    let correct = results
        .responses
        .iter()
        .filter_map(|&r| r)
        .filter(|&r| r)
        .count();
    let incorrect = total - correct;

    let total_time: Duration = results
        .reaction_times
        .iter()
        .filter_map(|&time| time) // Filter out None values and keep Some(Duration)
        .sum();
    let average_time = if total > 0 {
        total_time / total as u32
    } else {
        Duration::default()
    };

    let correct_times: Vec<_> = results
        .reaction_times
        .iter()
        .filter_map(|&time| time)
        .zip(results.responses.iter())
        .filter_map(|(time, &is_correct)| {
            if is_correct.unwrap_or_default() {
                Some(time)
            } else {
                None
            }
        })
        .collect();

    let incorrect_times: Vec<_> = results
        .reaction_times
        .iter()
        .filter_map(|&time| time)
        .zip(results.responses.iter())
        .filter_map(|(time, &is_correct)| {
            if !is_correct.unwrap_or_default() {
                Some(time)
            } else {
                None
            }
        })
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

    // Summary output by character
    let mut character_stats: HashMap<char, (u32, Duration, Duration, u32, u32)> = HashMap::new();

    for (i, &prompt) in results.prompts.iter().enumerate() {
        let entry = character_stats.entry(prompt).or_insert((
            0,
            Duration::default(),
            Duration::default(),
            0,
            0,
        ));
        entry.0 += 1; // Increment trial count

        if let Some(res) = results.responses[i] {
            if res {
                entry.1 += results.reaction_times[i].expect("Reaction time not found");
                entry.3 += 1
            // Add to correct times
            } else {
                entry.2 += results.reaction_times[i].expect("Reaction time not found");
                entry.4 += 1
                // Add to incorrect times
            }
        }
    }

    let mut summary: Vec<SummaryRow> = character_stats
        .into_iter()
        .map(
            |(character, (count, correct_time, incorrect_time, times_correct, times_incorrect))| {
                let avg_correct_time = if count > 0 {
                    correct_time / count
                } else {
                    Duration::default()
                };
                let avg_incorrect_time = if count > 0 {
                    incorrect_time / count
                } else {
                    Duration::default()
                };

                SummaryRow {
                    character,
                    count,
                    avg_correct_time: format!("{:.0?}ms", avg_correct_time.as_millis()),
                    avg_incorrect_time: format!("{:.0?}ms", avg_incorrect_time.as_millis()),
                    times_correct,
                    times_incorrect,
                }
            },
        )
        .collect();

    summary.sort_by(|a, b| {
        let avg_a = a
            .avg_correct_time
            .replace("ms", "")
            .parse::<f64>()
            .unwrap_or_default();
        let avg_b = b
            .avg_correct_time
            .replace("ms", "")
            .parse::<f64>()
            .unwrap_or_default();

        // Sort primarily by times_incorrect, then by avg_correct_time
        (a.times_incorrect, avg_a)
            .partial_cmp(&(b.times_incorrect, avg_b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Create and style the table.
    let mut table = Table::new(&summary);
    let table = table.with(Style::rounded());

    // Highlight rows where `times_incorrect > 0`

    println!("\nCharacter Performance Summary:\n");
    println!("{}", table);

    // Overall results
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

    if calibration {
        let average = average_time.as_millis();
        println!("\nYour calibrated baseline score is: {average}");
        println!("Provide this score as your baseline to the FECR quiz:");
        println!("\n   code-smore fecr-quiz -b {average}")
    } else {
        println!("Baseline latency subtracted: {baseline}ms");
        println!(
            "\nYour grade: {}
Speed rating: {}",
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
}
