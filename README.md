# rust-cli-template

This is my Rust template for new CLI apps.

## Features

 * [Just](https://github.com/casey/just) enabled project build targets.
 * [Clap](https://docs.rs/clap/latest/clap/) CLI argument parser.
 * Bash / Fish / Zsh shell (tab)
   [completion](https://docs.rs/clap_complete/latest/clap_complete/).
 * GitHub actions for tests and releases.
 * Test coverage report published to GitHub pages.
 * Publishing to crates.io.

## Use this template

 * [Create a new repository using this template](https://github.com/new?template_name=rust-cli-template&template_owner=EnigmaCurry).
 * The `Repository name` you choose will also be used as your app name.
 * Go to the repository `Settings` page:
   * Find `Actions`.
   * Find `General`.
   * Find `Workflow Permissions`.
   * Set `Read and Write permissions`.
   * Click `Save`.
 * Clone your new repository to your workstation.
 
## Render the template

```
./setup.sh
```
 
This will render the template into the project root and then
self-destruct this README.md and the template.
