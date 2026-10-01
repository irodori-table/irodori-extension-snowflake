# Snowflake Connector

Native Irodori Table connector extension for Snowflake.

This crate packages the connector metadata, native ABI exports, and driver implementation used by the Irodori extension marketplace.

## Connector

- Extension ID: `irodori.snowflake`
- Engine ID: `snowflake`
- Wire protocol: `snowflake`
- Default port: `443`
- Native ABI: `irodori.connector.native.v1`
- Driver linked: `yes`
- Marketplace visibility: `public`
- Package version: `0.1.6`

The package includes a desktop adapter source snapshot from `db/snowflake.rs`.

Connector metadata lives in `connector.config.json` and `irodori.extension.json`.
The Rust crate exports the native ABI from `src/lib.rs`, uses `irodori-connector-abi` for shared JSON/buffer helpers, and keeps connector behavior in `src/driver.rs`.

## Connection Metadata

- Endpoint modes: `hostPort`, `connectionString`
- Transport modes: `direct`, `sshTunnel`, `socks5Proxy`, `httpConnectProxy`, `proxyChain`
- TLS supported: `yes`
- TLS required by default: `yes`
- Custom driver options: `yes`

### Endpoint Fields

| Field | Label | Type | Required |
| --- | --- | --- | --- |
| `host` | Host | `string` | yes |
| `port` | Port | `number` | no |
| `database` | Database | `string` | no |

## Authentication

The connector advertises these authentication modes so clients can render the right credential fields. Driver-specific or provider-specific values can still be passed through `options` when needed.

| Auth method | Label | Kind | Secret purposes |
| --- | --- | --- | --- |
| `none` | No authentication | `none` | none |
| `connectionString` | Connection string / DSN | `connectionString` | none |
| `userPassword` | User/password | `userPassword` | `password` |
| `snowflakeKeyPair` | Key-pair JWT | `privateKey` | `privateKey`, `privateKeyPassphrase` |
| `oauth2` | OAuth 2.0 | `oauth2` | `token` |
| `accessToken` | Access token | `token` | `token` |
| `snowflakeProgrammaticAccessToken` | Programmatic access token | `token` | `token` |
| `snowflakeSessionToken` | Session token | `token` | `token` |
| `snowflakeWorkloadIdentity` | Workload identity federation | `iam` | `token` |
| `customDriverOptions` | Custom driver options | `custom` | `password`, `token`, `privateKey`, `privateKeyPassphrase` |

## Experience Metadata

- Domains: `warehouse`
- Result views: `worksheet`, `queryHistory`, `queryProfile`, `warehouseMonitor`, `costChart`, `copyReport`, `taskGraph`, `lineage`, `semanticModel`, `notebook`, `aiAssistant`, `table`
- Object types: `accounts`, `databases`, `schemas`, `tables`, `views`, `semanticViews`, `stages`, `fileFormats`, `warehouses`, `roles`, `users`, `shares`, `tasks`, `streams`, `dynamicTables`, `notebooks`, `queryHistory`, `queryProfile`, `cortexFunctions`
- Inspired by: Snowsight Worksheets, Snowsight Query History, Snowsight Query Profile, Snowflake SQL API, Snowflake Tasks, Snowflake Streams, Snowflake Dynamic Tables, Snowflake Semantic Views, Snowflake Cortex AISQL, sql-dialect-fmt

| Workflow | Result view | Templates |
| --- | --- | --- |
| Worksheet context | `worksheet` | `snowflake-context` |
| Query history triage | `queryHistory` | `snowflake-query-history`, `snowflake-expensive-queries`, `snowflake-result-scan` |
| Query profile drilldown | `queryProfile` | `snowflake-query-profile` |
| Warehouse monitor | `warehouseMonitor` | `snowflake-warehouse-load`, `snowflake-warehouse-metering` |
| Stage and COPY control | `copyReport` | `snowflake-copy-validate`, `snowflake-copy-into`, `snowflake-load-history` |
| Streams, tasks, and dynamic tables | `taskGraph` | `snowflake-stream-changes`, `snowflake-create-task`, `snowflake-dynamic-table` |
| Semantic model starter | `semanticModel` | `snowflake-semantic-view` |
| Cortex AISQL assistant | `aiAssistant` | `snowflake-cortex-complete`, `snowflake-cortex-summarize` |
| Role and grants audit | `table` | `snowflake-role-grants` |

| Template | Label | Language | Result view |
| --- | --- | --- | --- |
| `snowflake-context` | Current Snowflake context | `sql` | `worksheet` |
| `snowflake-query-history` | Recent query history | `sql` | `queryHistory` |
| `snowflake-expensive-queries` | Expensive queries | `sql` | `costChart` |
| `snowflake-result-scan` | Result scan | `sql` | `table` |
| `snowflake-query-profile` | Query operator stats | `sql` | `queryProfile` |
| `snowflake-warehouse-load` | Warehouse load history | `sql` | `warehouseMonitor` |
| `snowflake-warehouse-metering` | Warehouse metering history | `sql` | `costChart` |
| `snowflake-copy-validate` | Validate staged files | `sql` | `copyReport` |
| `snowflake-copy-into` | COPY INTO table | `sql` | `copyReport` |
| `snowflake-load-history` | Load history | `sql` | `copyReport` |
| `snowflake-stream-changes` | Read stream changes | `sql` | `lineage` |
| `snowflake-create-task` | Create scheduled task | `sql` | `taskGraph` |
| `snowflake-dynamic-table` | Create dynamic table | `sql` | `lineage` |
| `snowflake-semantic-view` | Semantic view starter | `sql` | `semanticModel` |
| `snowflake-cortex-complete` | Cortex complete | `sql` | `aiAssistant` |
| `snowflake-cortex-summarize` | Cortex summarize | `sql` | `aiAssistant` |
| `snowflake-role-grants` | Role grants | `sql` | `table` |

## SQL Dialect Metadata

- Dialect ID: `snowflake.sql`
- Name: Snowflake SQL
- Aliases: `snowflake`, `snowsql`, `sfsql`, `Snowflake`
- Keyword entries: `29`
- Snippets: `5`

Dialect metadata supports highlighting, completion, snippets, and query templates. It is intentionally lightweight metadata, not a complete SQL parser or syntax tree.

| Snippet | Kind |
| --- | --- |
| sf context | `snippet` |
| sf query history | `snippet` |
| sf query profile | `snippet` |
| sf copy validate | `snippet` |
| sf cortex complete | `snippet` |

## Native ABI Calls

| Method | Response |
| --- | --- |
| `health` | Returns connector health, engine id, ABI version, and driver status. |
| `describe` | Returns the embedded manifest and connector config. |
| `manifest` | Returns raw `irodori.extension.json`. |
| `config` | Returns raw `connector.config.json`. |
| `connect` | Opens and validates a native connector connection. |
| `query` | Runs a connector query and returns structured rows or JSON results. |
| `metadata` | Reads schemas, tables, columns, indexes, collections, or equivalent metadata. |
| `close` | Closes and removes a cached native connection. |

## Development

All extension crates in this checkout share `../target` so dependencies compile once across sibling repositories.

```sh
make check
make build
```

Release packages place platform-specific native artifacts under `dist/native`.
