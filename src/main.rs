use clap_complete::shells::Shell;

mod cli;
mod fecr_quiz;
mod morse;
mod prelude;

use prelude::*;
use std::io::BufRead;

use crate::morse::text_to_morse;

fn main() {
    let mut cmd = cli::app();
    let matches = cmd.clone().get_matches();

    // Configure logging:
    let log_level = if matches.get_flag("verbose") {
        Some("debug".to_string())
    } else {
        matches.get_one::<String>("log").cloned()
    };
    // Use RUST_LOG env var if no command-line option is provided
    let log_level = log_level.or_else(|| std::env::var("RUST_LOG").ok());
    // Fallback to "info" if neither command-line option nor env var is set
    let log_level = log_level.unwrap_or_else(|| "info".to_string());
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::from_str(&log_level).unwrap_or(log::LevelFilter::Info))
        .format_timestamp(None)
        .init();
    debug!("logging initialized.");

    // Print help if no subcommand is given:
    if matches.subcommand_name().is_none() {
        cmd.print_help().unwrap();
        println!();
        return;
    }

    // Global arguments
    let tone_freq: f32 = *matches
        .get_one::<f32>("tone")
        .expect("Missing --tone arg default");
    let text = *matches
        .get_one::<bool>("text")
        .expect("Missing --text arg default");
    let sound = *matches
        .get_one::<bool>("sound")
        .expect("Missing --sound arg default");

    // Calculate dot duration from wpm if not provided:
    let dot_duration = match (matches.get_one::<u32>("dot"), matches.get_one::<u32>("wpm")) {
        (Some(_), Some(_)) => {
            eprintln!("Error: '--dot' and '--wpm' cannot be used together.");
            std::process::exit(1);
        }
        (Some(&dot), None) => dot,
        (None, Some(&wpm)) => morse::wpm_to_dot_length(wpm),
        (None, None) => 60, // Default dot duration @ 20WPM
    };

    // Handle the subcommands:
    let exit_code = match matches.subcommand() {
        Some(("fecr-quiz", sub_matches)) => {
            let trials = sub_matches
                .get_one::<u32>("trials")
                .expect("Missing trials arg default");
            let char_set = sub_matches
                .get_one::<String>("characters")
                .expect("Missing --character arg default");
            let randomize = sub_matches
                .get_one::<bool>("random")
                .expect("Missing random arg default");
            let calibration_mode = sub_matches
                .get_one::<bool>("baseline-calibration")
                .expect("Missing --baseline-calibration arg default");
            let baseline = sub_matches
                .get_one::<u32>("baseline")
                .expect("Missing --baseline arg default");
            fecr_quiz::start_quiz(
                *trials,
                char_set,
                dot_duration,
                tone_freq,
                text,
                *randomize,
                *calibration_mode,
                *baseline,
            );
            0
        }
        Some(("test-sound", _sub_matches)) => {
            let player = morse::MorsePlayer::new();
            let message = "If sound is working, you should hear this test message now.";
            println!("{}", message);
            println!("{}", text_to_morse(message));
            player.play(message, dot_duration, tone_freq);
            0
        }
        Some(("read", sub_matches)) => {
            let player = morse::MorsePlayer::new();
            let morse = sub_matches
                .get_one::<bool>("morse")
                .expect("Missing --morse arg default");

            let stdin = std::io::stdin();
            if atty::is(atty::Stream::Stdin) {
                println!("## Type some text and it will be output as morse code.");
                println!("## You may also pipe text to this same command.");
                println!("## Press Enter after each line.");
                println!("## When done, press Ctrl-D to exit.");
            }
            for line in stdin.lock().lines() {
                match line {
                    Ok(line) => {
                        if text {
                            // Output text instead of sound
                            if *morse {
                                // stdin is already morse encoded, convert it to text:
                                println!("{}", morse::code_to_text(&line));
                                if sound {
                                    player.play_morse(&line, dot_duration, tone_freq);
                                    player.play_gap(dot_duration * 14);
                                }
                            } else {
                                // Encode stdin as morse code:
                                println!("{}", morse::text_to_morse(&line));
                                if sound {
                                    player.play(&line, dot_duration, tone_freq);
                                    player.play_gap(dot_duration * 14);
                                }
                            }
                        } else {
                            if *morse {
                                // stdin is already morse encoded:
                                player.play_morse(&line, dot_duration, tone_freq);
                                player.play_gap(dot_duration * 14);
                            } else {
                                // Convert stdin into morse and play it:
                                player.play(&line, dot_duration, tone_freq);
                                player.play_gap(dot_duration * 14);
                            }
                        }
                    }
                    Err(e) => eprintln!("Error reading line: {}", e),
                }
            }
            0
        }
        Some(("completions", sub_matches)) => {
            if let Some(shell) = sub_matches.get_one::<String>("shell") {
                match shell.as_str() {
                    "bash" => generate_completion_script(Shell::Bash),
                    "zsh" => generate_completion_script(Shell::Zsh),
                    "fish" => generate_completion_script(Shell::Fish),
                    shell => eprintln!("Unsupported shell: {shell}"),
                }
                0
            } else {
                eprintln!(
                    "### Instructions to enable tab completion for {}",
                    env!("CARGO_BIN_NAME")
                );
                eprintln!("");
                eprintln!("### Bash (put this in ~/.bashrc:)");
                eprintln!("  source <({} completions bash)", env!("CARGO_BIN_NAME"));
                eprintln!("");
                eprintln!("### To make an alias (eg. 'h'), add this too:");
                eprintln!("  alias h={}", env!("CARGO_BIN_NAME"));
                eprintln!(
                    "  complete -F _{} -o bashdefault -o default h",
                    env!("CARGO_BIN_NAME")
                );
                eprintln!("");
                eprintln!("### If you don't use Bash, you can also use Fish or Zsh:");
                eprintln!("### Fish (put this in ~/.config/fish/config.fish");
                eprintln!("  {} completions fish | source)", env!("CARGO_BIN_NAME"));
                eprintln!("### Zsh (put this in ~/.zshrc)");
                eprintln!(
                    "  autoload -U compinit; compinit; source <({} completions zsh)",
                    env!("CARGO_BIN_NAME")
                );
                1
            }
        }
        _ => 1,
    };

    eprintln!("");
    std::process::exit(exit_code);
}

fn generate_completion_script(shell: clap_complete::shells::Shell) {
    clap_complete::generate(
        shell,
        &mut cli::app(),
        env!("CARGO_BIN_NAME"),
        &mut io::stdout(),
    )
}