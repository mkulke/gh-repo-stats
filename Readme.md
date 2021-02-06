# gh-repo-stats

Retrieve all repositories of a GitHub organization in a paginated fashion and print them as CSV to stdout.

## Build

The tool is written/test w/ Rust 1.46.0

```
cargo t
cargo b --release
```

## Run

```
cargo r -- -h
gh-repo-stats 0.1.0
Retrieve GitHub repo stats

USAGE:
    gh-repo-stats [FLAGS] [OPTIONS] --github-token <github-token>

FLAGS:
    -a, --archived    Consider archived repositories
    -h, --help        Prints help information
    -V, --version     Prints version information

OPTIONS:
    -g, --github-token <github-token>     [env: GITHUB_TOKEN]
    -o, --org <org>                      Organization [default: microsoft]
```
