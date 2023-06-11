# Snake runtime

This repo contains both a CLI app as well as a Rust library to run snake games.


## Rust Library
View the docs [here](https://aitournament.github.io/snake_runtime/snake_runtime/)

To use the library, add it as a git dependency to a Rust project:

```toml
[dependencies]
snake_runtime = { git = "https://github.com/aitournament/snake_runtime.git", branch = "master" }

Be sure to keep this up to date by occasionally running `cargo update`.

```

## CLI

To view a list of features / options, run
`cargo run --release -- --help`