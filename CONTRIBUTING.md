# FluxFeed Contributors

FluxFeed is a web-based RSS feed reader.

We are still in "greenfield project mode", as there is no release yet,
so never worry about backwards compatibility.

## Tech Stack and Philosophy

- Written in Rust
- SQLite DB backend
- Focus on server-side rendering
- Modern, intuitive and beautiful UI
- Snappy interface
- Be a good net citizen, go out of your way to reduce resource usage on
  the services we access

## Features

- No user accounts for now
- Configurable via web app

## Conventions

- Code should be readable, maintainable, and testable.
- Try to adhere to the DRY principle.
- Don't overly abstract. Let's be pragmatic.
- Let's stick to best practices and idiomatic patterns.
- We prefer functions to be less than 50 lines and files less than 1000
  lines, but it's not a hard limit.
- Functions should not have more than five positional arguments, but
  it's not a hard limit.

## Development

- Issues will be in `issues/new` in markdown files.
- After solving an issue, move the file to `issues/closed`.
- After solving an issue, create a git commit. In the commit message,
  focus on the "why" instead of "how". The "how" can be deduced from the
  diff. However, a short summary of the "how" can't hurt to convey
  intent.
- Before commiting, run linters, formatters, and the test suite.
- When fixing bugs, add test cases.
- When adding features, update the docs and/or README.

## Version Management

To bump the version:

1. Run the version bump script with the new version number:
   ```bash
   ./scripts/bump-version.sh X.Y.Z
   ```

2. This script will:
   - Update `Cargo.toml`
   - Update `package.json`
   - Update `package-lock.json`
   - Update `Cargo.lock`
   - Create a git commit
   - Create a git tag `vX.Y.Z`

3. Push the changes and tag to trigger the Docker build:
   ```bash
   git push origin main
   git push origin vX.Y.Z
   ```

The version in `Cargo.toml` is the single source of truth. The health
endpoint automatically reads from it via `env!("CARGO_PKG_VERSION")`.

## Agents

If you are an LLM:

- use
  `git -c user.name="Claude Code" -c user.email="noreply@anthropic.com"`
  when commiting.
