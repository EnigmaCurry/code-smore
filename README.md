# code-smore

[![Crates.io](https://img.shields.io/crates/v/code-smore?color=blue
)](https://crates.io/crates/code-smore)
[![Coverage](https://img.shields.io/badge/Coverage-Report-purple)](https://EnigmaCurry.github.io/code-smore/coverage/master/)

This is a morse code practice tool.

## Install

[Download the latest release for your platform.](https://github.com/EnigmaCurry/code-smore/releases/latest)

Or install via cargo ([crates.io/crates/code-smore](https://crates.io/crates/code-smore)):

```
$ cargo install code-smore
```

## Usage

`code-smore` is a CLI program to run in your terminal (command window):

```
$ code-smore
Usage: code-smore [OPTIONS] [COMMAND]

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
$ code-smore test-sound
```

You should hear an example 42s transmission at 20 WPM.

## Fast Enough Character Recognition quiz

The FECR quiz will examine your skills at recognizing single
characters from the given character set (the alphanumeric set is used
by default if not provided).

Before you begin the quiz you may want to evaluate your baseline keyboard
skills.  The fecr-quiz provides an option to measure your keyboard reaction 
time from visual stimuli:

```
$ code-smore fecr-quiz -B
Your calibrated baseline score is: 610
Provide this score as your baseline to the FECR quiz
```

Run the FECR quiz by providing the set of characters you want to quiz
(e.g., `aeiou`.) and your personal baseline calibration value (e.g.,
`610`):
```
$ code-smore fecr-quiz -b 610 -c aeiou
```
If you choose not to provide a personal baseline value, the default of 500 milliseconds will be used.

Another technique for evaluating your baseline reaction time is to use
a simplified fecr-quiz which finds your reaction time to the simplest Morse code 
letters, E and T.  

```
$ code-smore fecr-quiz -b 0 -c ET --trials 8 --random
```

The quiz supports these optional named arguments:

```
  -c, --characters <characters>  Character set to shuffle/randomize for the quiz [default: ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890]
  -b, --baseline <baseline>      The baseline keyboard input latency in milliseconds [default: 500]
      --random    True randomization of characters (not just shuffled)
      --trials <trials>          [default: 26]
      --text                     Output text (cheat)
```

## Read and encode from stdin

You can send text to have it encoded into morse code:

To encode plain text and play back morse code as sound:

```
$ echo "Hello World" | code-smore read
```

To encode plain text to morse code text (no sound):

```
$ echo "Hello World" | code-smore read --text
.... . .-.. .-.. --- / .-- --- .-. .-.. -..
```

To encode plain text and output morse code text and sound:

```
$ echo "Hello World" | code-smore read --text --sound
```

To read plain text interactively and output morse code and sound:

```
$ code-smore read --text --sound
## Type some text and it will be output as morse code.
## You may also pipe text to this same command.
## Press Enter after each line.
## When done, press Ctrl-D to exit.
Hello World
```

Encode text and playback as separate steps in a pipeline, playback at 10WPM:

```
## --morse expects text to already be morse encoded:
$ echo "Hello World" | code-smore read --text | code-smore read --morse --wpm 10
```

## Tab completion

To install tab completion support, put this in your `~/.bashrc` (assuming you use Bash):

```
### Bash completion for code-smore (Put this in ~/.bashrc)
source <(code-smore completions bash)
```

If you don't like to type out the full name `code-smore`, you can make
a shorter alias (`h`), as well as enable tab completion for the alias
(`h`):

```
### Alias code-smore as h (Put this in ~/.bashrc):
alias h=code-smore
complete -F _code-smore -o bashdefault -o default h
```

Completion for Zsh and/or Fish has also been implemented, but the
author has not tested this:

```
### Zsh completion for code-smore (Put this in ~/.zshrc):
autoload -U compinit; compinit; source <(code-smore completions zsh)

### Fish completion for code-smore (Put this in ~/.config/fish/config.fish):
code-smore completions fish | source
```


## Development

See [DEVELOPMENT.md](DEVELOPMENT.md)
