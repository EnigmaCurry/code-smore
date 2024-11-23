# ${APP}

[![Crates.io](https://img.shields.io/crates/v/${APP}?color=blue
)](https://crates.io/crates/${APP})
[![Coverage](https://img.shields.io/badge/Coverage-Report-purple)](https://${GIT_USERNAME}.github.io/${APP}/coverage/master/)


## Install

[Download the latest release for your platform.](https://github.com/${GIT_USERNAME}/${APP}/releases)

Or install via cargo ([crates.io/crates/${APP}](https://crates.io/crates/${APP})):

```
cargo install ${APP}
```

### Tab completion

To install tab completion support, put this in your `~/.bashrc` (assuming you use Bash):

```
### Bash completion for ${APP} (Put this in ~/.bashrc)
source <(${APP} completions bash)
```

If you don't like to type out the full name `${APP}`, you can make
a shorter alias (`h`), as well as enable tab completion for the alias
(`h`):

```
### Alias ${APP} as h (Put this in ~/.bashrc):
alias h=${APP}
complete -F _${APP} -o bashdefault -o default h
```

Completion for Zsh and/or Fish has also been implemented, but the
author has not tested this:

```
### Zsh completion for ${APP} (Put this in ~/.zshrc):
autoload -U compinit; compinit; source <(${APP} completions zsh)

### Fish completion for ${APP} (Put this in ~/.config/fish/config.fish):
${APP} completions fish | source
```

## Usage

```
$ ${APP}

Usage: ${APP} [OPTIONS] [COMMAND]

Commands:

Options:
  -h, --help                  Print help
  -V, --version               Print version
```

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md)
