---
name: quickbase-cli
description: Use when work involves using, testing, documenting, or changing the quickbase CLI, including command syntax, output modes, examples, and binary-invoking tests.
---

# quickbase CLI

## Essentials

- Binary name: `quickbase`.
- Config path: `<repo-root>/.quickbase/quickbase.jsonc`.
- Example config path: `examples/.quickbase/quickbase.jsonc`.
- The example config and generated configs must default to `"mode": "dryrun"`.
- `--json` is the default output mode. `--markdown` and `--text` are console-friendly alternatives.

## Command Families

Use `references/commands.md` for command syntax and examples.

- `cmd`: dispatches Quickbase REST operations from `src/quickbase/operations.json`.
- `server`: runs the local mock Quickbase API.
- `util make-config`: creates the default config if absent.
- `util validate-config`: validates the default config without printing the token.
- `util make-skill`: copies project skills for Codex or Claude.

## Operation Arguments

- Request bodies are passed as stringified JSON in `--body`.
- Path and query parameters use named double-hyphen arguments such as `--appId=app123`.
- `cmd --base-url` can point requests at `quickbase server` for mock testing.
- Use `references/config.md` before changing config paths, generated config defaults, or mode semantics.

## Tests

- Prefer `assert_cmd::Command::cargo_bin("quickbase")` for binary behavior.
- Keep dry-run tests from performing network I/O.
- Use temporary Git repositories in tests; never write to real user directories or real project `.quickbase/` directories.
