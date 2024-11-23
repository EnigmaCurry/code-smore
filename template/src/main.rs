use clap_complete::shells::Shell;

mod cli;
mod prelude;

use prelude::*;

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

    // Handle the subcommands:
    eprintln!("");
    let exit_code = match matches.subcommand() {
        Some(("hello", sub_matches)) => {
            let name = sub_matches.get_one::<String>("NAME").unwrap();
            println!("Hello, {name}!");
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
