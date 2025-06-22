# Quickbase Argument Model

## Inputs

`quickbase cmd` accepts an operation ID followed by operation-specific named arguments.

```shell
quickbase cmd createTable --appId=app123 --body='{"name":"Projects"}'
```

Arguments are classified from `src/quickbase/operations.json`:

- Path args: required values that replace `{name}` placeholders in the path.
- Query args: named values appended to the URL query string.
- JSON body: `--body` receives stringified JSON and is parsed before a request is built.
- Config-derived values: realm and auth token come from `~/.quickbase/quickbase.jsonc`, with `--realm` able to override the realm header.

## Parsing Rules

- Operation lookup is exact first, then ASCII case-insensitive.
- Named args use double-hyphen syntax: `--appId=app123` or `--appId app123`.
- Missing required path or query args fail before network I/O.
- Unknown args fail before network I/O.
- `--body` is accepted only when the registry marks the operation as having a body.
- Malformed JSON in `--body` fails before network I/O.

## Request Construction

- Start from the operation method and path.
- Percent-encode path argument values when replacing placeholders.
- Append query arguments only when supplied.
- Use `https://api.quickbase.com/v1` unless `--base-url` is supplied.
- Send `QB-Realm-Hostname`, `Authorization`, and `User-Agent` for operations that require realm/auth.

## Tests To Update

When changing this model, update focused tests in `tests/cmd_cli.rs`, `tests/cmd_against_mock.rs`, and operation registry tests in `src/quickbase/operation.rs`.
