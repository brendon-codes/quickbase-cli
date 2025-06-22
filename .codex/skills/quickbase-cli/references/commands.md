# quickbase Commands

## `cmd`

Run a generated Quickbase operation:

```shell
quickbase cmd createTable --appId=app123 --body='{"name":"Projects"}'
```

Output mode flags:

- `--json`: default.
- `--markdown`: render console-friendly output.
- `--text`: alias-style console-friendly output.

Global `cmd` options:

- `--base-url <url>`: override the Quickbase API base URL, usually to target `quickbase server`.
- `--realm <hostname>`: override the config-derived `QB-Realm-Hostname`.

Operation syntax:

- First positional value after `cmd` is the operation ID.
- Operation lookup is exact first, then ASCII case-insensitive.
- Path and query args use named double-hyphen syntax.
- `--body` receives stringified JSON.

## `server`

Run a local mock Quickbase REST API server:

```shell
quickbase server --host 127.0.0.1 --port 0
```

Options:

- `--host`: bind host, default `127.0.0.1`.
- `--port`: bind port, default `0` for an OS-assigned port.
- `--data-dir`: mock storage root, default `<repo-root>/.quickbase/data`.

Mock usage:

```shell
quickbase server --port 0
quickbase cmd --base-url http://127.0.0.1:12345 createTable --appId=app123 --body='{"name":"Projects"}'
```

Use a live-mode config when intentionally sending `cmd` requests to the mock server; dry-run mode stops before network I/O.

## `util make-config`

Create `<repo-root>/.quickbase/quickbase.jsonc` from `examples/.quickbase/quickbase.jsonc` if the file does not exist:

```shell
quickbase util make-config
```

The command must not overwrite an existing config. It also creates `<repo-root>/.quickbase/.gitignore`.

## `util validate-config`

Validate `<repo-root>/.quickbase/quickbase.jsonc`:

```shell
quickbase util validate-config
```

The command reports validity, realm, and mode without printing `quickbaseUserToken`.

## `util status`

Check configured or overridden app connectivity and table count:

```shell
quickbase util status --json
quickbase util status --base-url http://127.0.0.1:12345 --appId=app123 --text
```

The command reports `configPath`, effective `quickbaseRealm`, `target`, app details, table count, and status. `target` is `mock` for local mock server URLs and `quickbase` otherwise.

## `util make-skill`

Copy both project skills for an agent:

```shell
quickbase util make-skill codex
quickbase util make-skill claude
```

Destinations:

- Codex: `<repo-root>/.codex/skills/<skill>/`.
- Claude: `<repo-root>/.claude/skills/<skill>/`.

The command replaces existing generated skill directories with the same skill names.
