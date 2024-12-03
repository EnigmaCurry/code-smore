use clap::{value_parser, Arg, Command};

pub fn app() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("dot")
                .long("dot")
                .global(true)
                .num_args(1)
                .value_name("DOT_DURATION")
                .value_parser(value_parser!(u32))
                .help("Sets the dot duration in milliseconds [default: 60]"),
        )
        .arg(
            Arg::new("wpm")
                .long("wpm")
                .global(true)
                .num_args(1)
                .value_name("WPM")
                .value_parser(value_parser!(u32))
                .help(
                    "Sets the speed in words per minute [default: 20]",
                ),
        )
        .arg(
            Arg::new("tone")
                .long("tone")
                .global(true)
                .num_args(1)
                .value_name("TONE_FREQ")
                .value_parser(value_parser!(f32))
                .default_value("440.0")
                .help("Sets the tone frequency in Hz"),
        )
        .arg(
            Arg::new("text")
                .long("text")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help(
                    "Output text rather than sound",
                ),
        )
        .arg(
            Arg::new("sound")
                .long("sound")
                .global(true)
                .action(clap::ArgAction::SetTrue)
                .help(
                    "Output sound in addition to the --text option",
                ),
        )
        .arg(
            Arg::new("log")
                .long("log")
                .global(true)
                .num_args(1)
                .value_name("LEVEL")
                .value_parser(["trace", "debug", "info", "warn", "error"])
                .hide(true)
                .help(
                    "Sets the log level, overriding the RUST_LOG environment variable.",
                ),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .global(true)
                .hide(true)
                .help("Sets the log level to debug.")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("fecr-quiz")
                .about(
                    "Start the Fast Enough Character Recognition quiz",
                )
                .arg(
                    Arg::new("characters")
                        .short('c')
                        .long("characters")
                        .default_value("ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890")
                        .help("Character set to shuffle/randomize for the quiz"),
                )
                .arg(
                    Arg::new("baseline-calibration")
                        .short('B')
                        .long("baseline-calibration")
                        .help(
                            "Runs the calibration process to calculate your personal output latency",
                        )
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("baseline")
                        .short('b')
                        .long("baseline")
                        .help("The baseline keyboard input latency in milliseconds")
                        .default_value("500")
                        .value_parser(value_parser!(u32)),
                )
                .arg(
                    Arg::new("trials")
                        .long("trials")
                        .default_value("26")
                        .value_parser(value_parser!(u32)),
                )
                .arg(
                    Arg::new("random")
                        .long("random")
                        .help(
                            "True randomization of characters (not just
    shuffled)",
                        )
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(Command::new("test-sound").about(
            "Test that sound is working",
        ))
        .subcommand(
            Command::new("read")
                .about(
                    "Read text from stdin and output it as morse code",
                )
                .arg(
                    Arg::new("morse")
                        .long("morse")
                        .action(clap::ArgAction::SetTrue)
                        .help(
                            "Input text is already morse encoded",
                        ),
                ),
        )
        .subcommand(
            Command::new("listen")
                .about(
                    "listen to morse code from a file or audio device and output it",
                )
                .arg(
                    Arg::new("morse")
                        .long("morse")
                        .action(clap::ArgAction::SetTrue)
                        .help(
                            "Output text in morse code",
                        ),
                )
                .arg(
                    Arg::new("threshold")
                        .short('t')
                        .long("threshold")
                        .value_parser(|v: &str| {
                            v.parse::<f32>()
                                .map_err(|_| String::from("Threshold must be a valid floating-point number"))
                                .and_then(|val| {
                                    if (0.0..=1.0).contains(&val) {
                                        Ok(val)
                                    } else {
                                        Err(String::from("Threshold must be between 0.0 and 1.0"))
                                    }
                                })
                        })
                        .help(
                            "Minimal signal value threshold [0.0..1.0]",
                        ),
                )
                .arg(
                    Arg::new("bandwidth")
                        .short('W')
                        .long("bandwidth")
                        .value_parser(|v: &str| {
                            v.parse::<f32>()
                                .map_err(|_| String::from("Bandwidth must be a valid floating-point number"))
                                .and_then(|val| {
                                    if (0.0..=1000.0).contains(&val) {
                                        Ok(val)
                                    } else {
                                        Err(String::from("Threshold must be between 0.0Hz and 1000.0Hz"))
                                    }
                                })
                        })
                        .help(
                            "Minimal signal value threshold [0.0..1.0]",
                        ),
                )
                .arg(
                    Arg::new("file")
                        .short('f')
                        .long("file")
                        .help("Read morse code from an audio file")
                        .conflicts_with("device"), // Ensures `--file` and `--device` are mutually exclusive
                )
                .arg(
                    Arg::new("device")
                        .short('d')
                        .long("device")
                        .help("Read morse code from an audio device")
                        .conflicts_with("file"), // Ensures `--device` and `--file` are mutually exclusive
                ),
        )
        .subcommand(
            Command::new("completions")
                .about(
                    "Generates shell completions
    script (tab completion)",
                )
                .hide(true)
                .arg(
                    Arg::new("shell")
                        .help("The shell to generate completions for")
                        .required(false)
                        .value_parser(["bash", "zsh", "fish"]),
                ),
        )
}
