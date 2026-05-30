# language-stats-rs

Axum service that serves an SVG chart of GitHub repository language usage.

I made it because [github-readme-stats](https://github.com/anuraghazra/github-readme-stats) only shows your first 100 repos and I have a lot more.

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

## Configuration

### Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `GITHUB_USERNAME` | Yes | GitHub username whose repository languages are visualized. |
| `GITHUB_TOKEN` | No | Personal access token. Increases API rate limits and allows access to private repositories. |
| `RUST_LOG` | No | Log filter for the server (default: `info`). Example: `RUST_LOG=debug cargo run`. |

### Query parameters

All options apply to `GET /languages`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `username` | `GITHUB_USERNAME` | GitHub user whose repository languages are visualized. When this differs from `GITHUB_USERNAME`, or when no `GITHUB_TOKEN` is configured, only public repositories are included. |
| `exclude` | _(none)_ | Languages to omit from the chart. Comma-separated in one param, or repeated params. Matching is case-insensitive. |
| `includeOrg` | `true` | When `true`, include organization repositories. When `false`, use personal repositories only. |
| `includePrivate` | `true` | When `true`, include private repositories (requires `GITHUB_TOKEN` and `username` matching `GITHUB_USERNAME`). When `false`, use public repositories only. |
| `showUsername` | `true` | When `true`, show `@username` in the full chart header. Ignored when `minimal=true`. |
| `minimal` | `false` | When `true`, render a compact 300px-wide badge instead of the full 1200×630 card. |

**Exclude examples**

```bash
# Comma-separated list
curl 'http://localhost:3000/languages?exclude=HTML,CSS,JavaScript' --output languages.svg

# Repeated params
curl 'http://localhost:3000/languages?exclude=Python&exclude=TypeScript' --output languages.svg

# Names with special characters (percent-encode; a literal # is treated as a URL fragment)
curl 'http://localhost:3000/languages?exclude=C%2B%2B&exclude=C%23' --output languages.svg

# Multi-word language names
curl 'http://localhost:3000/languages?exclude=Jupyter%20Notebook' --output languages.svg
```

**Chart layout examples**

```bash
# Another user's public repositories
curl 'http://localhost:3000/languages?username=torvalds' --output languages.svg

# Full card without username
curl 'http://localhost:3000/languages?showUsername=false' --output languages.svg

# Personal repos only
curl 'http://localhost:3000/languages?includeOrg=false' --output languages.svg

# Public repos only (excludes private repos when token is configured)
curl 'http://localhost:3000/languages?includePrivate=false' --output languages.svg

# Compact badge
curl 'http://localhost:3000/languages?minimal=true' --output languages.svg

# Combined (comma-separated flags are supported for badge URLs)
curl 'http://localhost:3000/languages?minimal=true,showUsername=false,includeOrg=false&exclude=HTML,CSS' --output languages.svg
```

Responses are cached for 30 minutes (`Cache-Control: public, max-age=1800`) and support conditional requests via `ETag` / `If-None-Match`. GitHub data is refreshed in the background every 30 minutes.

## Test

```bash
curl http://localhost:3000/languages --output languages.svg
```

## License

MIT — see [LICENSE](LICENSE).

## Credits

Heavily inspired by [github-readme-stats](https://github.com/anuraghazra/github-readme-stats).
