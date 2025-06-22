# quickbase Config

## Paths

- Default config path: `<repo-root>/.quickbase/quickbase.jsonc`.
- Example config path: `examples/.quickbase/quickbase.jsonc`.
- Mock data path: `<repo-root>/.quickbase/data` unless `server --data-dir` overrides it.
- Commands resolve `<repo-root>` by walking up from the current directory until a `.git` directory or `.git` file is found.
- `util make-config` also creates `<repo-root>/.quickbase/.gitignore` with `*` and `!.gitignore`.

## Shape

```jsonc
{
  "quickbaseRealm": "example.quickbase.com",
  "quickbaseUserToken": "replace-with-your-user-token",
  "mode": "dryrun"
}
```

Rules:

- `quickbaseRealm` is a hostname only; do not include `https://` or a path.
- `quickbaseUserToken` is sensitive and must not be printed in CLI output or test failures.
- `mode` is either `dryrun` or `live`.
- Examples and generated configs must stay in `dryrun` mode.

## Modes

- `dryrun`: validate command arguments and print the request that would be sent without network I/O.
- `live`: send HTTP requests to Quickbase or to the `--base-url` override.

## Status

`quickbase util status` reports the resolved `configPath`, effective `quickbaseRealm`, and `target`.
`target` is `mock` for local mock server URLs such as `localhost`, `127.0.0.1`, and `[::1]`; otherwise it is `quickbase`.

## Prompt Examples

Valid body example:

```shell
quickbase cmd getUsers --accountId=123 --body='{"emails":["a@example.com"],"appIds":["a1","a2"],"nextPageToken":""}'
```

Mock-server example:

```shell
quickbase server --port 0
quickbase cmd --base-url http://127.0.0.1:12345 createTable --appId=app123 --body='{"name":"Projects"}'
```
