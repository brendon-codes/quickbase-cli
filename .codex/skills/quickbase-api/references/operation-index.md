# Quickbase Operation Index

Generated from `src/quickbase/operations.json`.

Refresh guidance:

1. Obtain the updated vendor JSON source and perform a one-time external conversion to YAML; no converter is committed in this repository.
2. Update this skill's `references/quickbase-rest-api.yaml` with the converted full spec.
3. In the `quickbase-cli` repository, embed the same YAML in `src/quickbase/reference.rs` as `QUICKBASE_REST_API_YAML`.
4. Regenerate `src/quickbase/operations.json` from the reference.
5. Regenerate this table from the registry, preserving operation IDs, methods, paths, tags, required/optional args, and body presence.
6. Run `cargo test` and any focused command/mock tests affected by changed operations.

| Operation ID | Method | Path | Tag | Path args | Query args | Body |
| --- | --- | --- | --- | --- | --- | --- |
| `copyApp` | POST | `/apps/{appId}/copy` | Apps | --appId (string, required) | - | optional |
| `createApp` | POST | `/apps` | Apps | - | - | optional |
| `deleteApp` | DELETE | `/apps/{appId}` | Apps | --appId (string, required) | - | optional |
| `getApp` | GET | `/apps/{appId}` | Apps | --appId (string, required) | - | none |
| `getAppEvents` | GET | `/apps/{appId}/events` | Apps | --appId (string, required) | - | none |
| `getRoles` | GET | `/apps/{appId}/roles` | Apps | --appId (string, required) | - | none |
| `updateApp` | POST | `/apps/{appId}` | Apps | --appId (string, required) | - | optional |
| `audit` | POST | `/audit` | Audit | - | - | optional |
| `exchangeSsoToken` | POST | `/auth/oauth/token` | Auth | - | - | optional |
| `getTempTokenDBID` | GET | `/auth/temporary/{dbid}` | Auth | --dbid (string, required) | - | none |
| `generateDocument` | GET | `/docTemplates/{templateId}/generate` | Document Templates | --templateId (number, required) | --tableId (string, required)<br>--recordId (number)<br>--filename (string, required)<br>--format (string)<br>--margin (string)<br>--unit (string)<br>--pageSize (string)<br>--orientation (string)<br>--realm (string) | none |
| `createField` | POST | `/fields` | Fields | - | --tableId (string, required) | optional |
| `deleteFields` | DELETE | `/fields` | Fields | - | --tableId (string, required) | optional |
| `getField` | GET | `/fields/{fieldId}` | Fields | --fieldId (integer, required) | --tableId (string, required)<br>--includeFieldPerms (boolean) | none |
| `getFields` | GET | `/fields` | Fields | - | --tableId (string, required)<br>--includeFieldPerms (boolean) | none |
| `getFieldsUsage` | GET | `/fields/usage` | Fields | - | --tableId (string, required)<br>--skip (integer)<br>--top (integer) | none |
| `getFieldUsage` | GET | `/fields/usage/{fieldId}` | Fields | --fieldId (integer, required) | --tableId (string, required) | none |
| `updateField` | POST | `/fields/{fieldId}` | Fields | --fieldId (integer, required) | --tableId (string, required) | optional |
| `deleteFile` | DELETE | `/files/{tableId}/{recordId}/{fieldId}/{versionNumber}` | Files | --tableId (string, required)<br>--recordId (integer, required)<br>--fieldId (integer, required)<br>--versionNumber (integer, required) | - | none |
| `downloadFile` | GET | `/files/{tableId}/{recordId}/{fieldId}/{versionNumber}` | Files | --tableId (string, required)<br>--recordId (integer, required)<br>--fieldId (integer, required)<br>--versionNumber (integer, required) | - | none |
| `runFormula` | POST | `/formula/run` | Formulas | - | - | optional |
| `addManagersToGroup` | POST | `/groups/{gid}/managers` | Groups | --gid (number, required) | - | optional |
| `addMembersToGroup` | POST | `/groups/{gid}/members` | Groups | --gid (number, required) | - | optional |
| `addSubgroupsToGroup` | POST | `/groups/{gid}/subgroups` | Groups | --gid (number, required) | - | optional |
| `removeManagersFromGroup` | DELETE | `/groups/{gid}/managers` | Groups | --gid (number, required) | - | optional |
| `removeMembersFromGroup` | DELETE | `/groups/{gid}/members` | Groups | --gid (number, required) | - | optional |
| `removeSubgroupsFromGroup` | DELETE | `/groups/{gid}/subgroups` | Groups | --gid (number, required) | - | optional |
| `platformAnalyticEventSummaries` | POST | `/analytics/events/summaries` | Platform Analytics | - | --accountId (number) | required |
| `platformAnalyticReads` | GET | `/analytics/reads` | Platform Analytics | - | --day (string) | none |
| `deleteRecords` | DELETE | `/records` | Records | - | - | optional |
| `recordsModifiedSince` | POST | `/records/modifiedSince` | Records | - | - | optional |
| `runQuery` | POST | `/records/query` | Records | - | - | optional |
| `upsert` | POST | `/records` | Records | - | - | optional |
| `getReport` | GET | `/reports/{reportId}` | Reports | --reportId (string, required) | --tableId (string, required) | none |
| `getTableReports` | GET | `/reports` | Reports | - | --tableId (string, required) | none |
| `runReport` | POST | `/reports/{reportId}/run` | Reports | --reportId (string, required) | --tableId (string, required)<br>--skip (integer)<br>--top (integer) | optional |
| `changesetSolution` | PUT | `/solutions/{solutionId}/changeset` | Solutions | --solutionId (string, required) | - | optional |
| `changesetSolutionFromRecord` | GET | `/solutions/{solutionId}/changeset/fromrecord` | Solutions | --solutionId (string, required) | --tableId (string, required)<br>--fieldId (int, required)<br>--recordId (int, required) | none |
| `createSolution` | POST | `/solutions` | Solutions | - | - | optional |
| `createSolutionFromRecord` | GET | `/solutions/fromrecord` | Solutions | - | --tableId (string, required)<br>--fieldId (int, required)<br>--recordId (int, required) | none |
| `exportSolution` | GET | `/solutions/{solutionId}` | Solutions | --solutionId (string, required) | - | none |
| `exportSolutionToRecord` | GET | `/solutions/{solutionId}/torecord` | Solutions | --solutionId (string, required) | --tableId (string, required)<br>--fieldId (int, required) | none |
| `getSolutionPublic` | GET | `/solutions/{solutionId}/resources` | Solutions | --solutionId (string, required) | - | none |
| `updateSolution` | PUT | `/solutions/{solutionId}` | Solutions | --solutionId (string, required) | - | optional |
| `updateSolutionToRecord` | GET | `/solutions/{solutionId}/fromrecord` | Solutions | --solutionId (string, required) | --tableId (string, required)<br>--fieldId (int, required)<br>--recordId (int, required) | none |
| `createRelationship` | POST | `/tables/{tableId}/relationship` | Tables | --tableId (string, required) | - | optional |
| `createTable` | POST | `/tables` | Tables | - | --appId (string, required) | optional |
| `deleteRelationship` | DELETE | `/tables/{tableId}/relationship/{relationshipId}` | Tables | --tableId (string, required)<br>--relationshipId (number, required) | - | none |
| `deleteTable` | DELETE | `/tables/{tableId}` | Tables | --tableId (string, required) | --appId (string, required) | none |
| `getAppTables` | GET | `/tables` | Tables | - | --appId (string, required) | none |
| `getRelationships` | GET | `/tables/{tableId}/relationships` | Tables | --tableId (string, required) | --skip (integer) | none |
| `getTable` | GET | `/tables/{tableId}` | Tables | --tableId (string, required) | --appId (string, required) | none |
| `updateRelationship` | POST | `/tables/{tableId}/relationship/{relationshipId}` | Tables | --tableId (string, required)<br>--relationshipId (number, required) | - | optional |
| `updateTable` | POST | `/tables/{tableId}` | Tables | --tableId (string, required) | --appId (string, required) | optional |
| `addTrustees` | POST | `/app/{appId}/trustees` | Trustees | --appId (string, required) | - | optional |
| `getTrustees` | GET | `/app/{appId}/trustees` | Trustees | --appId (string, required) | - | none |
| `removeTrustees` | DELETE | `/app/{appId}/trustees` | Trustees | --appId (string, required) | - | optional |
| `updateTrustees` | PATCH | `/app/{appId}/trustees` | Trustees | --appId (string, required) | - | optional |
| `denyUsers` | PUT | `/users/deny` | Users | - | --accountId (number) | optional |
| `denyUsersAndGroups` | PUT | `/users/deny/{shouldDeleteFromGroups}` | Users | --shouldDeleteFromGroups (boolean, required) | --accountId (number) | optional |
| `getUserApps` | GET | `/users/{userId}/apps` | Users | --userId (string, required) | --accountId (number, required) | none |
| `getUsers` | POST | `/users` | Users | - | --accountId (number) | optional |
| `undenyUsers` | PUT | `/users/undeny` | Users | - | --accountId (number) | optional |
| `cloneUserToken` | POST | `/usertoken/clone` | UserToken | - | - | optional |
| `deactivateUserToken` | POST | `/usertoken/deactivate` | UserToken | - | - | none |
| `deleteUserToken` | DELETE | `/usertoken` | UserToken | - | - | none |
| `transferUserToken` | POST | `/usertoken/transfer` | UserToken | - | - | optional |
