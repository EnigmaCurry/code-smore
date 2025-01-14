# code-smore

[![Crates.io](https://img.shields.io/crates/v/code-smore?color=blue
)](https://crates.io/crates/code-smore)
[![Coverage](https://img.shields.io/badge/Coverage-Report-purple)](https://EnigmaCurry.github.io/code-smore/coverage/master/)

This is a morse code utility and practice tool.

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
  send        Send text from stdin as morse code
  receive     Receive morse code from an audio device, audio file, or GPIO.
  credits     Prints license information for all dependencies
  help        Print this message or the help of the given subcommand(s)

Options:
      --dot <DOT_DURATION>  Sets the dot duration in milliseconds [default: 60]
      --wpm <WPM>           Sets the speed in words per minute [default: 20]
      --tone <TONE_FREQ>    Sets the tone frequency in Hz [default: 440.0]
      --text                Output text rather than sound
      --sound               Output sound in addition to the --text option
      --gpio <pin-number>   Use GPIO instead of the sound device (select GPIO pin number)
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

Read the blog article introduction by [WA7PGE](https://wa7pge.com/home/operating_modes/cw/instant_character_recognition).

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

## Send morse code from text stdin

You can send text to have it encoded into morse code:

To encode plain text and play back morse code as sound:

```
$ echo "Hello World" | code-smore send
```

To encode plain text to morse code text (no sound):

```
$ echo "Hello World" | code-smore send --text
.... . .-.. .-.. --- / .-- --- .-. .-.. -..
```

To encode plain text and output morse code text and sound:

```
$ echo "Hello World" | code-smore send --text --sound
```

To send plain text interactively and output morse code and sound:

```
$ code-smore send --text --sound
## Type some text and it will be output as morse code.
## You may also pipe text to this same command.
## Press Enter after each line.
## When done, press Ctrl-D to exit.
Hello World
```

Encode text and playback as separate steps in a pipeline, playback at 10WPM:

```
## --morse expects text to already be morse encoded:
$ echo "Hello World" | code-smore send --text | code-smore send --morse --wpm 10
```

To encode plain text and send it as morse code to GPIO pin 4 (no sound):

```
code-smore send --gpio 4 --wpm 15
```

> **Note:** This will set the the pin high when activated, and low
> when idle. If your radio's key input is a simple switch you cannot
> use this pin directly, you will need to use a transistor to complete
> the circuit and control it with the pin.

## Receive morse code from sound

> **Note:** Decoding from sound is supported on Linux pipewire enabled
> systems only. It is an optional compile time feature and it is
> enabled by default (for Linux builds only).

code-smore can receive morse code from any sound program playing on
your computer and it will decode morse code from them. (This is a work
in progress and may only works with ideal conditions.)

```
code-smore receive --wpm 20
```

code-smore will listen to the monitor of your default sound device in
pipewire, so it should hear the same thing that you hear (You may also
manually connect code-smore to any single application by using a
pipewire patchbay tool like
[helvum](https://gitlab.freedesktop.org/pipewire/helvum)). Use the
`--wpm` argument to specify the expected (ballpark) rate of
transmission.

You can test the decoder by running `code-smore send` in
another terminal and watch it copy you. [Try playing this
video](https://youtube.com/watch?v=FxRN2nP_9dA). (try various `--wpm` 25 to 45.)

Please note that the the signal must be communication grade with no
interference. If you have any other sound playing in the background,
it will negatively affect the signal copy. Filtering signals has not
been implemented yet.

## Receive morse code from GPIO

> **Note:** The 'gpio' crate feature is enabled by default, but it
> requires special hardware normally only found in embedded systems
> (e.g., Raspberry Pi).

code-smore can also receive morse code directly from GPIO. This is
more reliable than decoding from audio because the signal is digital.

```
# This example receives morse signal on GPIO pin #17
code-smore receive --gpio 17
```

> **Note:** The receiving pin is normally high for idle, and low for
> activation. This is the opposite voltage logic of the output pin. To
> read the key input, you will need to use a pull-up resistor on the
> GPIO pin (this could also be done in software if the device has a
> pull-up resistor builtin, but this is not implemented yet.)

## Enable optional features

This crate offers the following optional Cargo feature flags:

 * `audio` (enabled by default on all platforms) this allows playing
   morse code audio to your sound device.
 * `pipewire` (enabled by default on Linux only) this allows receiving
   morse code audio from pipewire (can listen to any device or
   program).
 * `gpio` (enabled by default on Linux only) this allows receiving
   morse code signal from a GPIO logic pin (e.g., on Raspberry Pi)
 * `matrix` (enabled by default) this enables bridging between a
   Matrix channel and a morse code link.

If you are compiling code-smore yourself, you can add only the feature
flags you want:

```
just build --no-default-features --features audio,gpio,pipewire
```

## Tab completion

code-smore has optional tab completion support. Put this in your
`~/.bashrc` (assuming you use Bash):

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
