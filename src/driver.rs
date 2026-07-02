use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

use reqwest::{Client, Url};
use serde_json::{json, Map, Value};
use tokio::runtime::Runtime;

use crate::abi::{self, IrodoriConnectorBuffer};
use crate::{ABI_VERSION, CONFIG_JSON, DRIVER_LINKED, ENGINE, MANIFEST_JSON};

static CONNECTIONS: OnceLock<Mutex<HashMap<String, SnowflakeConnection>>> = OnceLock::new();
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

#[derive(Clone)]
struct SnowflakeConnection {
    client: Client,
    config: SnowflakeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnowflakeConfig {
    base_url: String,
    token: String,
    database: String,
    schema: Option<String>,
    warehouse: Option<String>,
    role: Option<String>,
    redaction_values: Vec<String>,
}

#[derive(Default)]
struct ObjectMeta {
    kind: String,
    columns: Vec<Value>,
    primary_key: Vec<Value>,
    foreign_keys: Vec<Value>,
}

type QueryRows = Vec<Vec<Value>>;
type QueryOutput = (Vec<String>, QueryRows, bool);

fn connections() -> &'static Mutex<HashMap<String, SnowflakeConnection>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn runtime() -> Result<&'static Runtime, String> {
    if let Some(runtime) = RUNTIME.get() {
        return Ok(runtime);
    }
    let runtime = Runtime::new().map_err(|err| format!("create tokio runtime failed: {err}"))?;
    let _ = RUNTIME.set(runtime);
    RUNTIME
        .get()
        .ok_or_else(|| "create tokio runtime failed.".to_string())
}

pub fn call_json(request: IrodoriConnectorBuffer) -> IrodoriConnectorBuffer {
    let request = match abi::parse_request(request) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let method = match abi::request_method(request.as_ref()) {
        Ok(method) => method,
        Err(response) => return response,
    };

    match method {
        "health" | "ping" => abi::ok(Map::from_iter([
            ("engine".to_string(), Value::String(ENGINE.to_string())),
            ("abiVersion".to_string(), json!(ABI_VERSION)),
            ("driverLinked".to_string(), Value::Bool(DRIVER_LINKED)),
        ])),
        "describe" | "capabilities" => abi::ok(Map::from_iter([
            ("engine".to_string(), Value::String(ENGINE.to_string())),
            ("abiVersion".to_string(), json!(ABI_VERSION)),
            ("driverLinked".to_string(), Value::Bool(DRIVER_LINKED)),
            (
                "manifest".to_string(),
                serde_json::from_str(MANIFEST_JSON).unwrap_or(Value::Null),
            ),
            (
                "config".to_string(),
                serde_json::from_str(CONFIG_JSON).unwrap_or(Value::Null),
            ),
        ])),
        "manifest" => abi::owned_buffer(MANIFEST_JSON.to_string()),
        "config" => abi::owned_buffer(CONFIG_JSON.to_string()),
        "connect" => connect(request.as_ref().expect("connect has request")),
        "query" => query(request.as_ref().expect("query has request")),
        "metadata" => metadata(request.as_ref().expect("metadata has request")),
        "close" => close(request.as_ref().expect("close has request")),
        other => abi::error(
            "connector.unknownMethod",
            format!("unknown connector method: {other}"),
        ),
    }
}

fn connect(request: &Value) -> IrodoriConnectorBuffer {
    let connection_id = abi::connection_id(Some(request));
    let config = match runtime()
        .and_then(|runtime| runtime.block_on(SnowflakeConfig::from_request(request)))
    {
        Ok(config) => config,
        Err(err) => return abi::error("connector.invalidRequest", err),
    };
    let connection = SnowflakeConnection {
        client: Client::new(),
        config,
    };
    let version = match runtime().and_then(|runtime| runtime.block_on(load_version(&connection))) {
        Ok(version) => version,
        Err(err) => return abi::error("connector.connectFailed", connection.config.redact(&err)),
    };

    let mut guard = match connections().lock() {
        Ok(guard) => guard,
        Err(_) => {
            return abi::error(
                "connector.statePoisoned",
                "Connector connection state is poisoned.",
            )
        }
    };
    let mut response = Map::from_iter([
        ("engine".to_string(), Value::String(ENGINE.to_string())),
        (
            "connectionId".to_string(),
            Value::String(connection_id.clone()),
        ),
        ("driverLinked".to_string(), Value::Bool(DRIVER_LINKED)),
        (
            "endpoint".to_string(),
            Value::String(connection.config.base_url.clone()),
        ),
        (
            "database".to_string(),
            Value::String(connection.config.database.clone()),
        ),
        ("serverVersion".to_string(), Value::String(version)),
    ]);
    if let Some(schema) = connection.config.schema.as_deref() {
        response.insert("schema".to_string(), Value::String(schema.to_string()));
    }
    if let Some(warehouse) = connection.config.warehouse.as_deref() {
        response.insert(
            "warehouse".to_string(),
            Value::String(warehouse.to_string()),
        );
    }
    guard.insert(connection_id, connection);
    abi::ok(response)
}

fn query(request: &Value) -> IrodoriConnectorBuffer {
    let connection_id = abi::connection_id(Some(request));
    let Some(sql) = abi::string_field(request, "sql")
        .or_else(|| abi::string_field(request, "query"))
        .or_else(|| abi::string_field(request, "statement"))
    else {
        return abi::error(
            "connector.invalidRequest",
            "query requires a string sql, query, or statement field.",
        );
    };
    let connection = match connection(&connection_id) {
        Ok(connection) => connection,
        Err(response) => return response,
    };
    match runtime()
        .and_then(|runtime| runtime.block_on(run_query(&connection, sql, abi::max_rows(request))))
    {
        Ok((columns, rows, truncated)) => abi::ok(Map::from_iter([
            ("connectionId".to_string(), Value::String(connection_id)),
            (
                "columns".to_string(),
                Value::Array(columns.into_iter().map(Value::String).collect()),
            ),
            (
                "rows".to_string(),
                Value::Array(rows.into_iter().map(Value::Array).collect()),
            ),
            ("truncated".to_string(), Value::Bool(truncated)),
        ])),
        Err(err) => abi::error("connector.queryFailed", connection.config.redact(&err)),
    }
}

fn metadata(request: &Value) -> IrodoriConnectorBuffer {
    let connection_id = abi::connection_id(Some(request));
    let connection = match connection(&connection_id) {
        Ok(connection) => connection,
        Err(response) => return response,
    };
    match runtime().and_then(|runtime| runtime.block_on(load_metadata(&connection))) {
        Ok(metadata) => abi::ok(Map::from_iter([
            ("connectionId".to_string(), Value::String(connection_id)),
            ("metadata".to_string(), metadata),
        ])),
        Err(err) => abi::error("connector.metadataFailed", connection.config.redact(&err)),
    }
}

fn close(request: &Value) -> IrodoriConnectorBuffer {
    let connection_id = abi::connection_id(Some(request));
    let mut guard = match connections().lock() {
        Ok(guard) => guard,
        Err(_) => {
            return abi::error(
                "connector.statePoisoned",
                "Connector connection state is poisoned.",
            )
        }
    };
    let existed = guard.remove(&connection_id).is_some();
    abi::ok(Map::from_iter([
        ("connectionId".to_string(), Value::String(connection_id)),
        ("closed".to_string(), Value::Bool(existed)),
    ]))
}

impl SnowflakeConfig {
    async fn from_request(request: &Value) -> Result<Self, String> {
        let raw_url = option_string(request, &["connectionString", "url", "dsn"]);
        let parsed_url = raw_url.as_deref().and_then(|url| Url::parse(url).ok());
        let host_input = option_string(request, &["host", "account"])
            .or_else(|| {
                parsed_url
                    .as_ref()
                    .and_then(|url| url.host_str().map(str::to_string))
            })
            .unwrap_or_else(|| "localhost".to_string());
        let host = snowflake_api_host(&host_input);
        let base_url = match (raw_url.as_deref(), parsed_url.as_ref()) {
            (Some(raw), Some(parsed))
                if parsed.scheme() == "http" || parsed.scheme() == "https" =>
            {
                raw.trim_end_matches('/').to_string()
            }
            (Some(_), Some(parsed)) if parsed.scheme() == "snowflake" => format!("https://{host}"),
            (Some(raw), _) => format!("https://{}", snowflake_api_host(raw)),
            (None, _) => format!("https://{host}"),
        };
        let database_input = option_string(request, &["database", "db"])
            .or_else(|| parsed_url.as_ref().and_then(database_from_url_path))
            .unwrap_or_default();
        let (database, schema) = split_database_schema(
            database_input,
            option_string(request, &["schema"])
                .or_else(|| query_value(parsed_url.as_ref(), "schema")),
        );
        let warehouse = option_string(request, &["warehouse"])
            .or_else(|| query_value(parsed_url.as_ref(), "warehouse"));
        let role =
            option_string(request, &["role"]).or_else(|| query_value(parsed_url.as_ref(), "role"));
        let user = option_string(request, &["user", "username"]).or_else(|| {
            parsed_url.as_ref().and_then(|url| {
                let username = url.username();
                (!username.is_empty()).then(|| username.to_string())
            })
        });
        let password = option_string(request, &["password"]).or_else(|| {
            parsed_url
                .as_ref()
                .and_then(|url| url.password().map(str::to_string))
        });
        let token = if let Some(password) = password.as_deref().filter(|value| !value.is_empty()) {
            login_session(
                &base_url,
                &host,
                user.as_deref().unwrap_or_default(),
                password,
            )
            .await?
        } else {
            option_string(
                request,
                &["token", "accessToken", "sessionToken", "bearerToken"],
            )
            .or_else(|| query_value(parsed_url.as_ref(), "token"))
            .ok_or_else(|| "Snowflake connect requires password login or token.".to_string())?
        };
        let mut redaction_values = Vec::new();
        push_sensitive(&mut redaction_values, password.as_deref());
        push_sensitive(&mut redaction_values, Some(&token));
        collect_url_auth(&base_url, &mut redaction_values);
        Ok(Self {
            base_url,
            token,
            database,
            schema,
            warehouse,
            role,
            redaction_values,
        })
    }

    fn redact(&self, message: &str) -> String {
        self.redaction_values.iter().fold(
            message.replace(&self.base_url, "<snowflake-url>"),
            |message, secret| {
                if secret.is_empty() {
                    message
                } else {
                    message.replace(secret, "****")
                }
            },
        )
    }
}

async fn login_session(
    base_url: &str,
    host: &str,
    user: &str,
    password: &str,
) -> Result<String, String> {
    let account = host
        .to_ascii_lowercase()
        .find(".snowflakecomputing.com")
        .map(|index| host[..index].to_string())
        .unwrap_or_else(|| host.to_string());
    let payload = json!({
        "data": {
            "CLIENT_APP_ID": "IrodoriTable",
            "CLIENT_APP_VERSION": "0.1.0",
            "ACCOUNT_NAME": account,
            "LOGIN_NAME": user,
            "PASSWORD": password
        }
    });
    let response = Client::new()
        .post(format!("{base_url}/api/v1/login-request"))
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("Snowflake login request failed: {err}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("Snowflake login response read failed: {err}"))?;
    if !status.is_success() {
        return Err(format!("Snowflake login returned HTTP {status}: {text}"));
    }
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|err| format!("Snowflake login JSON parse failed: {err}: {text}"))?;
    if !value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Snowflake authentication failed.")
            .to_string());
    }
    value
        .pointer("/data/token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "Snowflake login succeeded but returned no token.".to_string())
}

async fn load_version(connection: &SnowflakeConnection) -> Result<String, String> {
    let (_, rows, _) = run_query(connection, "SELECT CURRENT_VERSION()", 1).await?;
    Ok(rows
        .first()
        .and_then(|row| row.first())
        .and_then(Value::as_str)
        .map(|version| format!("Snowflake {version}"))
        .unwrap_or_else(|| "Snowflake".to_string()))
}

async fn run_query(
    connection: &SnowflakeConnection,
    sql: &str,
    cap: usize,
) -> Result<QueryOutput, String> {
    let mut payload = json!({
        "sqlText": sql,
        "database": connection.config.database,
    });
    if let Some(schema) = connection.config.schema.as_deref() {
        payload["schema"] = Value::String(schema.to_string());
    }
    if let Some(warehouse) = connection.config.warehouse.as_deref() {
        payload["warehouse"] = Value::String(warehouse.to_string());
    }
    if let Some(role) = connection.config.role.as_deref() {
        payload["role"] = Value::String(role.to_string());
    }
    let response = connection
        .client
        .post(format!(
            "{}/api/v1/query-request",
            connection.config.base_url
        ))
        .header(
            "Authorization",
            format!("Snowflake Token=\"{}\"", connection.config.token),
        )
        .header("Accept", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("Snowflake query request failed: {err}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| format!("Snowflake query response read failed: {err}"))?;
    if !status.is_success() {
        return Err(format!("Snowflake query returned HTTP {status}: {text}"));
    }
    let value = serde_json::from_str::<Value>(&text)
        .map_err(|err| format!("Snowflake query JSON parse failed: {err}: {text}"))?;
    if !value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err(value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Unknown Snowflake query error.")
            .to_string());
    }
    let data = value
        .get("data")
        .ok_or_else(|| "Snowflake response missing data.".to_string())?;
    let columns = data
        .get("rowtype")
        .and_then(Value::as_array)
        .map(|rowtype| {
            rowtype
                .iter()
                .filter_map(|column| column.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut rows = Vec::new();
    let mut truncated = false;
    if let Some(rowset) = data.get("rowset").and_then(Value::as_array) {
        for row in rowset {
            if rows.len() >= cap {
                truncated = true;
                break;
            }
            rows.push(row.as_array().cloned().unwrap_or_else(|| vec![row.clone()]));
        }
    }
    Ok((columns, rows, truncated))
}

async fn load_metadata(connection: &SnowflakeConnection) -> Result<Value, String> {
    let database = sql_string_literal(&connection.config.database);
    let schema_filter = metadata_schema_filter("TABLE_SCHEMA", connection.config.schema.as_deref());
    let object_sql = format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE \
         FROM INFORMATION_SCHEMA.TABLES \
         WHERE TABLE_CATALOG = {database}{schema_filter} \
         UNION ALL \
         SELECT TABLE_SCHEMA, TABLE_NAME, 'VIEW' AS TABLE_TYPE \
         FROM INFORMATION_SCHEMA.VIEWS \
         WHERE TABLE_CATALOG = {database}{schema_filter} \
         ORDER BY TABLE_SCHEMA, TABLE_NAME"
    );
    let (object_cols, object_rows, _) = run_query(connection, &object_sql, 5000).await?;
    let mut schemas: BTreeMap<String, BTreeMap<String, ObjectMeta>> = BTreeMap::new();
    for row in object_rows {
        let schema = field(&object_cols, &row, "TABLE_SCHEMA").unwrap_or_default();
        let table = field(&object_cols, &row, "TABLE_NAME").unwrap_or_default();
        let table_type = field(&object_cols, &row, "TABLE_TYPE").unwrap_or_default();
        if schema.is_empty() || table.is_empty() {
            continue;
        }
        schemas.entry(schema).or_default().insert(
            table,
            ObjectMeta {
                kind: if table_type.eq_ignore_ascii_case("VIEW") {
                    "view".to_string()
                } else {
                    "table".to_string()
                },
                columns: Vec::new(),
                primary_key: Vec::new(),
                foreign_keys: Vec::new(),
            },
        );
    }

    let column_sql = format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, DATA_TYPE, ORDINAL_POSITION, \
                IS_NULLABLE, COLUMN_DEFAULT, COMMENT \
         FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_CATALOG = {database}{schema_filter} \
         ORDER BY TABLE_SCHEMA, TABLE_NAME, ORDINAL_POSITION"
    );
    let (columns, rows, _) = run_query(connection, &column_sql, 10_000).await?;
    for row in rows {
        let schema = field(&columns, &row, "TABLE_SCHEMA").unwrap_or_default();
        let table = field(&columns, &row, "TABLE_NAME").unwrap_or_default();
        if schema.is_empty() || table.is_empty() {
            continue;
        }
        let object = schemas
            .entry(schema)
            .or_default()
            .entry(table)
            .or_insert_with(|| ObjectMeta {
                kind: "table".to_string(),
                columns: Vec::new(),
                primary_key: Vec::new(),
                foreign_keys: Vec::new(),
            });
        object.columns.push(json!({
            "name": field(&columns, &row, "COLUMN_NAME").unwrap_or_default(),
            "dataType": field(&columns, &row, "DATA_TYPE").unwrap_or_default(),
            "nullable": field(&columns, &row, "IS_NULLABLE")
                .map(|value| value.eq_ignore_ascii_case("YES"))
                .unwrap_or(true),
            "ordinal": field(&columns, &row, "ORDINAL_POSITION")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or((object.columns.len() + 1) as i64),
            "defaultValue": field(&columns, &row, "COLUMN_DEFAULT"),
            "comment": field(&columns, &row, "COMMENT")
        }));
    }
    Ok(json!({
        "schemas": schemas
            .into_iter()
            .map(|(schema, objects)| json!({
                "name": schema,
                "objects": objects
                    .into_iter()
                    .map(|(name, object)| json!({
                        "schema": schema,
                        "name": name,
                        "kind": object.kind,
                        "columns": object.columns,
                        "indexes": [],
                        "primaryKey": object.primary_key,
                        "foreignKeys": object.foreign_keys
                    }))
                    .collect::<Vec<_>>()
            }))
            .collect::<Vec<_>>()
    }))
}

fn connection(connection_id: &str) -> Result<SnowflakeConnection, IrodoriConnectorBuffer> {
    let guard = connections().lock().map_err(|_| {
        abi::error(
            "connector.statePoisoned",
            "Connector connection state is poisoned.",
        )
    })?;
    guard.get(connection_id).cloned().ok_or_else(|| {
        abi::error(
            "connector.connectionNotFound",
            format!("no open connection: {connection_id}"),
        )
    })
}

fn split_database_schema(
    database_input: String,
    explicit_schema: Option<String>,
) -> (String, Option<String>) {
    let mut database = database_input.trim().to_string();
    let mut schema = explicit_schema.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    if schema.is_none() {
        let current = database.clone();
        if let Some((db, sc)) = current.split_once('/') {
            database = db.trim().to_string();
            let trimmed = sc.trim();
            if !trimmed.is_empty() {
                schema = Some(trimmed.to_string());
            }
        } else if let Some((db, sc)) = current.split_once('.') {
            database = db.trim().to_string();
            let trimmed = sc.trim();
            if !trimmed.is_empty() {
                schema = Some(trimmed.to_string());
            }
        }
    }
    (database, schema)
}

fn snowflake_api_host(input: &str) -> String {
    let trimmed = input
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let lower = trimmed.to_lowercase();
    if lower.contains("snowflakecomputing.com")
        || lower.starts_with("localhost")
        || lower.starts_with("127.")
    {
        trimmed.to_string()
    } else {
        format!("{trimmed}.snowflakecomputing.com")
    }
}

fn database_from_url_path(url: &Url) -> Option<String> {
    let parts = url
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .take(2)
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [] => None,
        [database] => Some((*database).to_string()),
        [database, schema] => Some(format!("{database}/{schema}")),
        _ => None,
    }
}

fn query_value(url: Option<&Url>, key: &str) -> Option<String> {
    url?.query_pairs()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.trim().is_empty())
}

fn metadata_schema_filter(column: &str, schema: Option<&str>) -> String {
    schema
        .map(str::trim)
        .filter(|schema| !schema.is_empty())
        .map(|schema| format!(" AND {column} = {}", sql_string_literal(schema)))
        .unwrap_or_default()
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn field(columns: &[String], row: &[Value], name: &str) -> Option<String> {
    columns
        .iter()
        .position(|column| column.eq_ignore_ascii_case(name))
        .and_then(|index| row.get(index))
        .and_then(|value| match value {
            Value::Null => None,
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.is_empty())
}

fn request_containers(request: &Value) -> Vec<&Value> {
    [
        Some(request),
        request.get("profile"),
        request.get("options"),
        request.get("auth"),
        request.get("secrets"),
        request
            .get("profile")
            .and_then(|profile| profile.get("options")),
        request
            .get("profile")
            .and_then(|profile| profile.get("auth")),
        request
            .get("profile")
            .and_then(|profile| profile.get("secrets")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn option_string(request: &Value, fields: &[&str]) -> Option<String> {
    request_containers(request)
        .into_iter()
        .find_map(|container| {
            fields.iter().find_map(|field| {
                container
                    .get(*field)
                    .map(|value| match value {
                        Value::String(value) => value.clone(),
                        Value::Number(value) => value.to_string(),
                        Value::Bool(value) => value.to_string(),
                        _ => String::new(),
                    })
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
        })
}

fn push_sensitive(values: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        if !values.iter().any(|existing| existing == value) {
            values.push(value.to_string());
        }
    }
}

fn collect_url_auth(url: &str, values: &mut Vec<String>) {
    let Some(after_scheme) = url.split_once("://").map(|(_, rest)| rest) else {
        return;
    };
    let Some(auth) = after_scheme
        .split('/')
        .next()
        .and_then(|host| host.split('@').next())
    else {
        return;
    };
    if auth.contains(':') {
        for part in auth.split(':') {
            push_sensitive(values, Some(part));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_account_host() {
        assert_eq!(
            snowflake_api_host("abc123"),
            "abc123.snowflakecomputing.com"
        );
        assert_eq!(
            snowflake_api_host("abc123.snowflakecomputing.com"),
            "abc123.snowflakecomputing.com"
        );
    }

    #[test]
    fn splits_database_and_schema() {
        assert_eq!(
            split_database_schema("DB.PUBLIC".to_string(), None),
            ("DB".to_string(), Some("PUBLIC".to_string()))
        );
    }
}
