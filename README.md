# quickbase-cli

`quickbase-cli` is a Rust CLI for querying and operating against the Quickbase
REST API. The installed command is `quickbase`.

The implementation is staged by the plans in `plans/initial/*.md`. The current
version includes JSONC configuration loading, utility commands for creating and
validating `<repo-root>/.quickbase/quickbase.jsonc`, and `quickbase cmd` dispatch
for the 67 operations in the checked-in Quickbase REST API registry. It also
includes a local mock Quickbase API server for development and tests.

## Build

Build and test from the repository root:

```shell
cargo build
cargo test
```

Run the CLI directly during development:

```shell
cargo run -- --help
```

Install the current checkout into Cargo's binary directory:

```shell
cargo install --path .
```

## Config

Create the default JSONC config:

```shell
quickbase util make-config
```

The config is written to `<repo-root>/.quickbase/quickbase.jsonc`, where
`<repo-root>` is the root of the current Git repository. If the current
directory is inside a subdirectory of a Git repository, the CLI walks up to the
repo root. `make-config` also creates `<repo-root>/.quickbase/.gitignore` so
config, tokens, and mock data are ignored by default.

The config is copied from `examples/.quickbase/quickbase.jsonc`. Example and
generated configs default to dry-run mode:

```jsonc
{
  "quickbaseRealm": "example.quickbase.com",
  "quickbaseAppId": "replace-with-your-app-id",
  "quickbaseUserToken": "replace-with-your-user-token",
  "mode": "dryrun"
}
```

Use a realm hostname only, without `https://` or a path. `quickbaseAppId` is
used as the fallback for operations that accept `--appId`. Validate the config
without printing the token:

```shell
quickbase util validate-config
```

Set `"mode": "live"` only when you intend to send requests to Quickbase or to a
mock server selected with `cmd --base-url`. Live mode performs HTTP requests for
mutating operations.

## `cmd`

Run a Quickbase operation by operation ID:

```shell
quickbase cmd --json createTable --body='{"name":"My Table"}'
```

`--json` is the default output format. `--markdown` and `--text` render a
console-friendly fenced JSON block. Use `--base-url` to point at a mock server
and `--realm` to override the configured `QB-Realm-Hostname` header.
Operations that accept `--appId` use `quickbaseAppId` from config when the flag
is omitted.

When the config mode is `dryrun`, `cmd` validates arguments and prints the
request that would be sent without performing network I/O. The authorization
token is redacted in output.

Prompt-style examples should pass JSON bodies as one string:

```shell
quickbase cmd getUsers --accountId=123 --body='{"emails":["a@example.com"],"appIds":["a1","a2"],"nextPageToken":""}'
quickbase cmd --markdown createTable --appId=app123 --body='{"name":"Projects"}'
```

## `server`

Run a local mock Quickbase REST API server:

```shell
quickbase server --host 127.0.0.1 --port 0
```

The command prints the chosen `baseUrl` before serving requests. Use that URL
with `cmd --base-url` and a config whose mode is `live` to exercise requests
against the mock instead of the real Quickbase API.

Mock data is stored as JSON under `<repo-root>/.quickbase/data/` by default.
Tests and local experiments can override that location:

```shell
quickbase server --data-dir /tmp/quickbase-mock-data
```

On startup and graceful shutdown, the server resets only its managed
`state.json` file and `realms/` tree inside the configured data root. App,
table, field, and record flows persist across requests during one server run;
all other registered operations return deterministic mock JSON that echoes the
matched operation, path parameters, query parameters, and body.

## `util`

Create the default config if it does not exist:

```shell
quickbase util make-config
```

Validate the default config without printing the user token:

```shell
quickbase util validate-config
```

Check configured or overridden app connectivity and table count:

```shell
quickbase util status --json
quickbase util status --appId=app123 --text
```

Status output includes the resolved `configPath`, the effective
`quickbaseRealm`, and `target`, which is `quickbase` for the real Quickbase API
or `mock` when a local mock server URL is supplied with `--base-url`.

Copy the checked-in project skills for Codex or Claude:

```shell
quickbase util make-skill codex
quickbase util make-skill claude
```

Codex skills are written to `<repo-root>/.codex/skills/<skill>/`; Claude skills
are written to `<repo-root>/.claude/skills/<skill>/`. Existing generated skill
directories with the same skill names are replaced.
