# language-stats-rs

Axum service that serves a PNG chart of GitHub repository language usage.

## Requirements

- [Rust](https://rustup.rs/) (stable toolchain)
- A GitHub username (and optionally a [personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens))

## Build

```bash
cargo build --release
```

## Run

```bash
cp .env.example .env
# Set GITHUB_USERNAME (and GITHUB_TOKEN if needed)
cargo run
```

The server listens on `http://localhost:3000`.

## Test

```bash
curl http://localhost:3000/languages --output languages.png
```

## License

MIT — see [LICENSE](LICENSE).
