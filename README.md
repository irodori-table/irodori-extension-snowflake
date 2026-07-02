# Snowflake Connector

Adds Snowflake connectivity as an installable connector extension.

This connector is listed in the public Irodori extension marketplace.

## Connector

- Extension ID: `irodori.snowflake`
- Engine ID: `snowflake`
- Wire: `snowflake`
- Default port: `443`
- Native ABI: `irodori.connector.native.v1`
- Driver linked: `true`

A desktop adapter source snapshot is staged in `native/source/` from `db/snowflake.rs`.

Connector metadata lives in `connector.config.json` and `irodori.extension.json`.
The Rust code keeps native ABI exports in `src/lib.rs`, shared buffer/JSON helpers in `src/abi.rs`, and Snowflake SQL API behavior in `src/driver.rs`.

## Connection Metadata

- Endpoint modes: `hostPort`, `connectionString`
- Transport modes: `direct`, `sshTunnel`, `socks5Proxy`, `httpConnectProxy`, `proxyChain`
- TLS supported: `true`
- Custom driver options: `true`

| Auth method | Label | Secret purposes |
|---|---|---|
| `none` | No authentication | none |
| `connectionString` | Connection string / DSN | none |
| `userPassword` | User/password | `password` |
| `snowflakeKeyPair` | Key-pair JWT | `privateKey`, `privateKeyPassphrase` |
| `oauth2` | OAuth 2.0 | `token` |
| `snowflakeSessionToken` | Session token | `token` |
| `browserSso` | Browser SSO | `token` |
| `saml` | SAML SSO | `token` |
| `externalBrowser` | External browser | `token` |
| `customDriverOptions` | Custom driver options | `password`, `token`, `privateKey`, `privateKeyPassphrase` |

## SQL Dialect

- Dialect ID: `snowflake.sql`
- Aliases: `snowflake`, `snowsql`, `sfsql`, `Snowflake`
- Formatter provider: `sql-dialect-fmt`
- Formatter command: `sql_format_snowflake`
- Keyword case: `upper`
- Identifier quote: `"`
- Line width: `100`
- Indent width: `4`

## Experience Metadata

- Domains: `warehouse`
- Result views: `worksheet`, `queryHistory`, `queryProfile`, `warehouseMonitor`, `costChart`, `copyReport`, `taskGraph`, `lineage`, `semanticModel`, `notebook`, `aiAssistant`, `table`
- Inspired by: `Snowsight Worksheets`, `Snowsight Query History`, `Snowsight Query Profile`, `Snowflake SQL API`, `Snowflake Tasks`, `Snowflake Streams`, `Snowflake Dynamic Tables`, `Snowflake Semantic Views`, `Snowflake Cortex AISQL`, `sql-dialect-fmt`

| Workflow | Result view | Templates |
|---|---|---|
| Worksheet context | worksheet | snowflake-context |
| Query history triage | queryHistory | snowflake-query-history, snowflake-expensive-queries, snowflake-result-scan |
| Query profile drilldown | queryProfile | snowflake-query-profile |
| Warehouse monitor | warehouseMonitor | snowflake-warehouse-load, snowflake-warehouse-metering |
| Stage and COPY control | copyReport | snowflake-copy-validate, snowflake-copy-into, snowflake-load-history |
| Streams, tasks, and dynamic tables | taskGraph | snowflake-stream-changes, snowflake-create-task, snowflake-dynamic-table |
| Semantic model starter | semanticModel | snowflake-semantic-view |
| Cortex AISQL assistant | aiAssistant | snowflake-cortex-complete, snowflake-cortex-summarize |
| Role and grants audit | table | snowflake-role-grants |

| Template | Label | Language | Result view |
|---|---|---|---|
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

## ABI Calls

The driver handles these JSON requests today:

| Method | Response |
|---|---|
| `health` / `ping` | Connector health, engine id, ABI version, and driver link status. |
| `describe` / `capabilities` | Embedded manifest and connector config. |
| `manifest` | Raw `irodori.extension.json`. |
| `config` | Raw `connector.config.json`. |
| `connect` | Authenticates with Snowflake and reads `CURRENT_VERSION()`. |
| `query` | Runs SQL through Snowflake SQL API. |
| `metadata` | Reads table and column metadata from Snowflake `INFORMATION_SCHEMA`. |
| `close` | Removes the cached native connection. |

## Development


Generated extension repositories share `../target` across sibling repositories so Rust dependencies are compiled once per checkout. DuckDB and MotherDuck are driver-linked by default; set `IRODORI_CONNECTOR_LINK_DUCKDB=0` only when you need metadata-only DuckDB-compatible scaffolds.


```sh
make check
make build
```

Release packages place platform-specific native artifacts under `dist/native`.
