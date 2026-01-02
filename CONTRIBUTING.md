# FluxFeed Contributors

FluxFeed is a web-based RSS feed reader.

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

## Agents

If you are an LLM:

- use
  `git -c user.name="Claude Code" -c user.email="noreply@anthropic.com"`
  when commiting.
