# morse-quest

[![Crates.io](https://img.shields.io/crates/v/morse-quest?color=blue
)](https://crates.io/crates/morse-quest)
[![Coverage](https://img.shields.io/badge/Coverage-Report-purple)](https://EnigmaCurry.github.io/morse-quest/coverage/master/)

This is a morse code practice tool.


## Install

[Download the latest release for your platform.](https://github.com/EnigmaCurry/morse-quest/releases)

Or install via cargo ([crates.io/crates/morse-quest](https://crates.io/crates/morse-quest)):

```
cargo install morse-quest
```

### Tab completion

To install tab completion support, put this in your `~/.bashrc` (assuming you use Bash):

```
### Bash completion for morse-quest (Put this in ~/.bashrc)
source <(morse-quest completions bash)
```

If you don't like to type out the full name `morse-quest`, you can make
a shorter alias (`h`), as well as enable tab completion for the alias
(`h`):

```
### Alias morse-quest as h (Put this in ~/.bashrc):
alias h=morse-quest
complete -F _morse-quest -o bashdefault -o default h
```

Completion for Zsh and/or Fish has also been implemented, but the
author has not tested this:

```
### Zsh completion for morse-quest (Put this in ~/.zshrc):
autoload -U compinit; compinit; source <(morse-quest completions zsh)

### Fish completion for morse-quest (Put this in ~/.config/fish/config.fish):
morse-quest completions fish | source
```

## Usage

```
$ morse-quest
Usage: morse-quest [OPTIONS] [COMMAND]

Commands:
  fecr-quiz   Start the Fast Enough Character Recognition quiz
  test-sound  Test that sound is working
  help        Print this message or the help of the given subcommand(s)

Options:
      --dot <DOT_DURATION>  Sets the dot duration in milliseconds [default: 60]
      --wpm <WPM>           Sets the speed in words per minute [default: 20]
      --tone <TONE_FREQ>    Sets the tone frequency in Hz [default: 440.0]
  -h, --help                Print help
  -V, --version             Print version
```

## Test sound

To test that your sound device is working, run this command:

```
morse-quest test-sound
```

You should hear an example 36s transmission at 20 WPM.

## Fast Enough Character Recognition quiz

To run the FECR quiz, run this command:

```
morse-quest fecr-quiz aeiou
```

The FECR quiz will examine your skills at recognizing single
characters from the given set (the full alphanumeric set is used by
default if not provided).

The quiz supports these optional arguments:

```
      --trials <trials>     [default: 128]
      --cheat               Print the text character to the screen
```

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md)
