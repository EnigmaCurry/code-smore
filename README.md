# morse-quest

[![Crates.io](https://img.shields.io/crates/v/morse-quest?color=blue
)](https://crates.io/crates/morse-quest)
[![Coverage](https://img.shields.io/badge/Coverage-Report-purple)](https://EnigmaCurry.github.io/morse-quest/coverage/master/)

This is a morse code practice tool.

## Install

[Download the latest release for your platform.](https://github.com/EnigmaCurry/morse-quest/releases)

Or install via cargo ([crates.io/crates/morse-quest](https://crates.io/crates/morse-quest)):

```
$ cargo install morse-quest
```

## Usage

`morse-quest` is a CLI program to run in your terminal (command window):

```
$ morse-quest
Usage: morse-quest [OPTIONS] [COMMAND]

Commands:
  fecr-quiz   Start the Fast Enough Character Recognition quiz
  test-sound  Test that sound is working
  read        Read text from stdin and output it as morse code
  help        Print this message or the help of the given subcommand(s)

Options:
      --dot <DOT_DURATION>  Sets the dot duration in milliseconds [default: 60]
      --wpm <WPM>           Sets the speed in words per minute [default: 20]
      --tone <TONE_FREQ>    Sets the tone frequency in Hz [default: 440.0]
      --text                Output text rather than sound
      --sound               Output sound in addition to the --text option
  -h, --help                Print help
  -V, --version             Print version
```

Note that `--dot` and `--wpm` are mutually exclusive, you may only set
one or the other.

## Test sound

To test that your sound device is working, run this command:

```
$ morse-quest test-sound
```

You should hear an example 42s transmission at 20 WPM.

## Fast Enough Character Recognition quiz

The FECR quiz will examine your skills at recognizing single
characters from the given character set (the alphanumeric set is used
by default if not provided).

Before you begin the quiz you should test your baseline keyboard
skills and measure your personal input latency:

```
$ morse-quest fecr-quiz -B
Your calibrated baseline score is: 610
```

Run the FECR quiz by providing the set of characters you want to quiz
(e.g., `aeiou`.) and your personal baseline calibration value (e.g.,
`610`):

```
$ morse-quest fecr-quiz -b 610 aeiou
```

The quiz supports these optional named arguments:

```
      --trials <trials>     [default: 128]
      --cheat               Print the text character to the screen
      --random              True randomization of characters (not just shuffled)
```

## Read and encode from stdin

You can send text to have it encoded into morse code:

To encode plain text and play back morse code as sound:

```
$ echo "Hello World" | morse-quest read
```

To encode plain text to morse code text (no sound):

```
$ echo "Hello World" | morse-quest read --text
.... . .-.. .-.. --- / .-- --- .-. .-.. -..
```

To encode plain text and output morse code text and sound:

```
$ echo "Hello World" | morse-quest read --text --sound
```

To read plain text interactively and output morse code and sound:

```
$ morse-quest read --text --sound
## Type some text and it will be output as morse code.
## You may also pipe text to this same command.
## Press Enter after each line.
## When done, press Ctrl-D to exit.
Hello World
```

Encode text and playback as separate steps in a pipeline, playback at 10WPM:

```
## --morse expects text to already be morse encoded:
$ echo "Hello World" | morse-quest read --text | morse-quest read --morse --wpm 10
```

## Tab completion

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


## Development

See [DEVELOPMENT.md](DEVELOPMENT.md)
