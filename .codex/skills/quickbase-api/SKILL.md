---
name: quickbase-api
description: Use when work involves Quickbase REST API behavior, operation IDs, endpoint mapping, request or response shapes, auth headers, route paths, mock API behavior, or updates to the API command registry.
---

# Quickbase API

## Source Of Truth

- Use this skill's `references/quickbase-rest-api.yaml` for Quickbase REST API lookups during agent workflows.
- In the `quickbase-cli` repository, Rust code and tests use `src/quickbase/reference.rs`, which embeds the same full spec YAML as `QUICKBASE_REST_API_YAML`.
- `src/quickbase/operations.json` is the generated 67-operation command registry used by `quickbase cmd`.
- When the API reference changes, perform a one-time external conversion from the vendor JSON source to YAML, update both full-spec artifacts, then update `src/quickbase/operations.json`, mock behavior, and tests that assert operation counts or endpoint mappings.

## Swagger Details

- The checked-in Quickbase reference is Swagger 2.0.
- The reference declares `host` as `api.quickbase.com/v1` and `basePath` as `/`.
- Runtime requests should resolve to `https://api.quickbase.com/v1` unless `cmd --base-url` overrides the base URL for tests or the mock server.

## Request Headers

- `QB-Realm-Hostname`: derived from config `quickbaseRealm` unless `cmd --realm` overrides it.
- `Authorization`: sent as `QB-USER-TOKEN <token>` from config `quickbaseUserToken`; redact tokens in logs, output, and tests.
- `User-Agent`: keep a deterministic `quickbase-cli` client user agent when changing HTTP code.
- Quickbase QBL operations may define `QBL-Version` and `X-QBL-Errors-As-Success`; preserve these as explicit header arguments if they are added to the command registry.

## Argument Model

Use `references/argument-model.md` before changing command parsing or request construction.

- Path args fill `{name}` placeholders in the operation path.
- Query args become URL query pairs.
- `--body` receives a stringified JSON request body and must be parsed before network I/O.
- Auth and realm are config-derived unless a CLI override exists.

## Operation Index

Use `references/operation-index.md` for the generated operation list. Regenerate it from `src/quickbase/operations.json`; do not manually invent endpoint mappings.
