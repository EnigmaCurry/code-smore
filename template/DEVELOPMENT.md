# Development

These instructions are specific to Fedora; minor adjustments for your
platform may be required.

## Install host dependencies

```
sudo dnf install git openssh rustup
sudo dnf install @development-tools @development-libs
```

## Install rust and cargo

```
rustup-init ## just press enter when prompted for default selection
. "$HOME/.cargo/env"
```

## Clone source repository

```
git clone git@github.com:${GIT_USERNAME}/${APP}.git \
  ~/git/vendor/${GIT_USERNAME}/${APP}
cd ~/git/vendor/${GIT_USERNAME}/${APP}
```

## Install development dependencies

```
cargo install just
just deps
```

## Build and run development ${APP}

```
just run help
just run [ARGS ...]
```

## Build release binary

```
just build --release
```

## Create development alias

```
## Add this to ~/.bashrc or equivalent:
alias ${APP}='just -f ~/git/vendor/${GIT_USERNAME}/${APP}/Justfile run'
alias h=${APP}
```

Now you can run `${APP}`, or simply `h`, from any directory, with
any arguments, and it will automatically rebuild from source, and then
run it with those args.

## Testing

This project has incomplete testing. [See the latest coverage
report](https://${GIT_USERNAME}.github.io/${APP}/coverage/master/).

## Run tests

```
# Run all tests:
just test

# Run a single test:
just test test_cli_help

# Verbose logging (which normally would be hidden for passing tests)
just test-verbose test_cli_help

# Auto run tests on source change:
just test-watch
```

## Clippy

```
just clippy
just clippy --fix
```

## Release (Github actions)

### Bump release version and push new branch

The `bump-version` target will automatically update the version number
in Cargo.toml, Cargo.lock, and README.md as suggested by git-cliff.
This creates a new branch named `release-{VERSION}`, and automatically
checks it out. You just need to `git push` the branch:

```
just bump-version
# ... automatically checks out a new branch named release-{VERSION}

git push
```

### Make a new PR with the changeset

Branch protection is enabled, all changesets must come in the form of
a Pull Request. On GitHub, create a new Pull Request for the
`release-{VERSION}` branch into the master branch.

### Merge the PR and tag the release

Once the PR is merged, update your local repo, and run the release
target:

```
git checkout master
git pull
just release
```

New binaries will be automatically built by github actions, and a new
packaged release will be posted.
