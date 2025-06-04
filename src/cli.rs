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
                .conflicts_with("gpio")
                .help(
                    "Output sound in addition to the --text option",
                ),
        )
        .arg(
            Arg::new("device")
                .long("device")
                .global(true)
                .num_args(1)
                .value_name("DEVICE")
                .help("Choose an explicit sound device instead of the system default")
        )
        .arg(
            Arg::new("ptt-rts")
                .long("ptt-rts")
                .global(true)
                .num_args(1)
                .value_name("PORT")
                .conflicts_with_all(["gpio", "rigctl"])
                .help("Assert RTS on this serial port to trigger radio's PTT (e.g. /dev/ttyUSB0)")
        )
        .arg(
            Arg::new("cw-rts")
                .long("cw-rts")
                .global(true)
                .num_args(1)
                .value_name("PORT")
                .conflicts_with_all(["gpio"])
                .help("Assert RTS on this serial port to key radio's CW (e.g. /dev/ttyUSB0)")
        )
        .arg(
            Arg::new("rigctl")
                .long("rigctl")
                .global(true)
                .num_args(1)
                .value_name("DEVICE")
                .requires("rigctl-model")
                .conflicts_with_all(["gpio", "ptt-rts"])
                .help("Control radio PTT via Hamlib rigctl device (e.g. /dev/ttyACM0)")
        )
        .arg(
            Arg::new("rigctl-model")
                .long("rigctl-model")
                .global(true)
                .num_args(1)
                .requires("rigctl")
                .value_name("ID")
                .help("Hamlib rig model ID (e.g. 3085 for IC-705, see `rigctl -l`)")
        )
        .arg(
            Arg::new("gpio")
                .long("gpio")
                .global(true)
                .value_parser(clap::value_parser!(u8))
                .value_name("pin-number")
                .conflicts_with_all(["ptt-rts", "rigctl"])
                .help(
                    "Use GPIO instead of the sound device (select GPIO pin number)",
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
        .subcommand(Command::new("list-devices").about(
            "List all system sound devices",
        ))
        .subcommand(
            Command::new("send")
                .about(
                    "Send text from stdin as morse code",
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
            Command::new("receive")
                .about(
                    "Receive morse code from desktop audio monitor, a specific audio device, an audio file, or GPIO.",
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
                    Arg::new("listen")
                        .long("listen")
                        .help("Use PipeWire to receive morse code from default system audio monitor")
                        .action(clap::ArgAction::SetTrue)
                        .conflicts_with_all(["device", "file"]),
                )
                .arg(
                    Arg::new("file")
                        .short('f')
                        .long("file")
                        .help("Receive morse code from an audio file")
                        .conflicts_with_all(["device", "listen"]),
                )
                .arg(
                    Arg::new("device")
                        .short('d')
                        .long("device")
                        .help("Receive morse code from an audio device")
                        .conflicts_with_all(["file", "listen"]),
                ),
        )
        .subcommand(
            Command::new("transceive")
                .about("Interactive half-duplex send/receive session")
                .arg(
                    Arg::new("device")
                        .short('d')
                        .long("device")
                        .required(true)
                        .value_name("ALSA_DEVICE")
                        .help("ALSA device to receive morse from"),
                )
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
        .subcommand(
            Command::new("credits")
                .about(
                    "Prints license information for all dependencies",
                )
        )
}
