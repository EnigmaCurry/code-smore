use clap_complete::shells::Shell;

mod cli;
mod credits;
mod fecr_quiz;
mod filter;
mod gpio;
mod message;
mod morse;
mod pipewire;
mod prelude;
mod term;

use is_terminal::IsTerminal;
use prelude::*;
use std::io::BufRead;
use std::u8;

use crate::pipewire::ensure_pipewire;

use crate::{credits::print_credits, morse::text_to_morse};

fn main() {
    let mut cmd = cli::app();
    let matches = cmd.clone().get_matches();
    let rts_port = matches.get_one::<String>("rts").map(|s| s.as_str());

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
    let gpio = matches.get_one::<u8>("gpio").is_some();
    let gpio_pin: u8 = matches.get_one::<u8>("gpio").copied().unwrap_or(u8::MAX);

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
                rts_port,
            );
            0
        }
        Some(("test-sound", _sub_matches)) => {
            let player = morse::MorsePlayer::new();
            let message = "If sound is working, you should hear this test message now.";
            println!("{}", message);
            println!("{}", text_to_morse(message));
            player.play(message, dot_duration, tone_freq, rts_port);
            0
        }
        Some(("send", sub_matches)) => {
            let player = morse::MorsePlayer::new();
            let morse = sub_matches
                .get_one::<bool>("morse")
                .expect("Missing --morse arg default");

            let stdin = std::io::stdin();
            if stdin.is_terminal() {
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
                                    player.play_morse(&line, dot_duration, tone_freq, rts_port);
                                    player.play_gap(dot_duration * 14, rts_port);
                                } else if gpio {
                                    player.gpio_morse(&line, dot_duration, gpio_pin);
                                    player.gpio_gap(dot_duration * 14, gpio_pin);
                                }
                            } else {
                                // Encode stdin as morse code:
                                println!("{}", morse::text_to_morse(&line));
                                if sound {
                                    player.play(&line, dot_duration, tone_freq, rts_port);
                                    player.play_gap(dot_duration * 14, rts_port);
                                } else if gpio {
                                    player.gpio(&line, dot_duration, gpio_pin);
                                    player.gpio_gap(dot_duration * 14, gpio_pin);
                                }
                            }
                        } else if *morse {
                            // stdin is already morse encoded:
                            if gpio {
                                player.gpio_morse(&line, dot_duration, gpio_pin);
                                player.gpio_gap(dot_duration * 14, gpio_pin);
                            } else {
                                // Sound is the default:
                                player.play_morse(&line, dot_duration, tone_freq, rts_port);
                                player.play_gap(dot_duration * 14, rts_port);
                            }
                        } else {
                            // Convert stdin into morse and play it:
                            if gpio {
                                player.gpio(&line, dot_duration, gpio_pin);
                                player.gpio_gap(dot_duration * 14, gpio_pin);
                            } else {
                                // Sound is the default:
                                player.play(&line, dot_duration, tone_freq, rts_port);
                                player.play_gap(dot_duration * 14, rts_port);
                            }
                        }
                    }
                    Err(e) => eprintln!("Error reading line: {}", e),
                }
            }
            0
        }
        Some(("receive", sub_matches)) => {
            //
            let morse = sub_matches
                .get_one::<bool>("morse")
                .expect("Missing --morse arg default");
            let listen = sub_matches
                .get_one::<bool>("listen")
                .copied()
                .unwrap_or(false);
            if gpio {
                // Receive from GPIO
                gpio::gpio_receive(dot_duration, gpio_pin, *morse)
                    .expect("Unhandled SIGINT or other fault");
            } else if listen {
                // Receive from audio device
                let device = sub_matches
                    .get_one::<String>("device")
                    .map(|s| s.to_string());
                let file = sub_matches.get_one::<String>("file").map(|s| s.to_string());
                let threshold = sub_matches
                    .get_one::<f32>("threshold")
                    .copied()
                    .unwrap_or(0.3);
                let bandwidth = sub_matches
                    .get_one::<f32>("bandwidth")
                    .copied()
                    .unwrap_or(200.0);
                match (&device, &file) {
                    (None, Some(_file)) => {
                        error!("TODO. Audio file input is not supported yet.");
                        std::process::exit(1);
                    }
                    (Some(_device), None) => {
                        error!("TODO. Setting the input device name is not supported yet. Leave this setting unset to use the default device.");
                        std::process::exit(1);
                    }
                    (Some(_device), Some(_file)) => {
                        error!("Cannot specify --device and --file simultaneousy.");
                        std::process::exit(1);
                    }
                    _ => {}
                }
                if cfg!(target_os = "linux") {
                    ensure_pipewire();
                    pipewire::listen(tone_freq, bandwidth, threshold, dot_duration, *morse)
                        .expect("pipewire::listen() failed");
                } else {
                    error!("Sorry, the listen feature is only supported on Linux right now.");
                    std::process::exit(1);
                }
            } else {
                // No valid input source specified
                eprintln!("Error: You must specify an input method. Try one of:");
                eprintln!("  --gpio <PIN>");
                eprintln!("  --listen");
                eprintln!("  --device <name> (not implemented yet)");
                eprintln!("  --file <path>   (not implemented yet)");
                println!();
                cmd.find_subcommand_mut("receive")
                    .expect("Missing 'receive' subcommand")
                    .print_help()
                    .unwrap();
                println!();
                std::process::exit(1);
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
                eprintln!();
                eprintln!("### Bash (put this in ~/.bashrc:)");
                eprintln!("  source <({} completions bash)", env!("CARGO_BIN_NAME"));
                eprintln!();
                eprintln!("### To make an alias (eg. 'h'), add this too:");
                eprintln!("  alias h={}", env!("CARGO_BIN_NAME"));
                eprintln!(
                    "  complete -F _{} -o bashdefault -o default h",
                    env!("CARGO_BIN_NAME")
                );
                eprintln!();
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
        Some(("credits", _sub_matches)) => {
            print_credits();
            0
        }
        _ => 1,
    };

    eprintln!();
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
