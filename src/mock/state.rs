use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    error::Result,
    quickbase::operation::{Operation, operations},
};

use super::storage::MockStorage;

#[derive(Clone, Debug)]
pub struct MockState {
    inner: Arc<Mutex<MockDataset>>,
    storage: MockStorage,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockDataset {
    pub counters: MockCounters,
    pub realms: BTreeMap<String, RealmData>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockCounters {
    pub app: u64,
    pub table: u64,
    pub field: u64,
    pub record: u64,
    pub relationship: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmData {
    pub apps: BTreeMap<String, AppData>,
    pub groups: Value,
    pub users: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub app: Value,
    pub tables: BTreeMap<String, TableData>,
    pub trustees: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableData {
    pub table: Value,
    pub fields: BTreeMap<String, Value>,
    pub records: BTreeMap<String, Value>,
    pub relationships: BTreeMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct MockRequest {
    pub path: String,
    pub query: BTreeMap<String, String>,
    pub path_params: BTreeMap<String, String>,
    pub body: Option<Value>,
    pub realm: String,
}

impl Default for MockCounters {
    fn default() -> Self {
        Self {
            app: 1,
            table: 1,
            field: 6,
            record: 1,
            relationship: 1,
        }
    }
}

impl Default for RealmData {
    fn default() -> Self {
        Self {
            apps: BTreeMap::new(),
            groups: json!([]),
            users: json!([]),
        }
    }
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            app: json!({}),
            tables: BTreeMap::new(),
            trustees: json!([]),
        }
    }
}

impl Default for TableData {
    fn default() -> Self {
        Self {
            table: json!({}),
            fields: BTreeMap::new(),
            records: BTreeMap::new(),
            relationships: BTreeMap::new(),
        }
    }
}

impl MockState {
    pub fn reset_new(storage: MockStorage) -> Result<Self> {
        storage.reset()?;
        Ok(Self {
            inner: Arc::new(Mutex::new(MockDataset::default())),
            storage,
        })
    }

    pub fn data_dir(&self) -> &std::path::Path {
        self.storage.root()
    }

    pub fn reset(&self) -> Result<()> {
        let mut dataset = self.inner.lock().expect("mock state mutex poisoned");
        *dataset = MockDataset::default();
        self.storage.reset()
    }

    pub fn operation_count(&self) -> usize {
        operations().len()
    }

    pub fn handle(
        &self,
        operation: &Operation,
        request: MockRequest,
    ) -> Result<(StatusCode, Value)> {
        let mut dataset = self.inner.lock().expect("mock state mutex poisoned");
        let body = match operation.operation_id.as_str() {
            "createApp" => create_app(&mut dataset, &request),
            "copyApp" => copy_app(&mut dataset, &request),
            "getApp" => get_app(&dataset, &request),
            "updateApp" => update_app(&mut dataset, &request),
            "deleteApp" => delete_app(&mut dataset, &request),
            "getAppTables" => get_app_tables(&dataset, &request),
            "createTable" => create_table(&mut dataset, &request),
            "getTable" => get_table(&dataset, &request),
            "updateTable" => update_table(&mut dataset, &request),
            "deleteTable" => delete_table(&mut dataset, &request),
            "createField" => create_field(&mut dataset, &request),
            "getFields" => get_fields(&dataset, &request),
            "getField" => get_field(&dataset, &request),
            "updateField" => update_field(&mut dataset, &request),
            "deleteFields" => delete_fields(&mut dataset, &request),
            "upsert" => upsert_records(&mut dataset, &request),
            "runQuery" => run_query(&dataset, &request),
            "deleteRecords" => delete_records(&mut dataset, &request),
            "recordsModifiedSince" => records_modified_since(&dataset, &request),
            _ => generic_response(operation, &request),
        };

        self.storage.persist(&dataset)?;
        Ok((StatusCode::OK, body))
    }
}

fn create_app(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let app_id = string_field(request.body.as_ref(), "id")
        .or_else(|| string_field(request.body.as_ref(), "appId"))
        .unwrap_or_else(|| next_id("app", &mut dataset.counters.app));
    let app = merge_body(
        request.body.as_ref(),
        json!({
            "id": app_id,
            "name": string_field(request.body.as_ref(), "name").unwrap_or_else(|| "Mock App".to_owned()),
            "realm": request.realm,
        }),
    );
    realm_mut(dataset, &request.realm).apps.insert(
        app_id.clone(),
        AppData {
            app: app.clone(),
            ..AppData::default()
        },
    );
    app
}

fn copy_app(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let source_app_id = required_path(request, "appId");
    let new_app_id = next_id("app", &mut dataset.counters.app);
    let realm = realm_mut(dataset, &request.realm);
    let mut app = realm
        .apps
        .get(&source_app_id)
        .cloned()
        .unwrap_or_else(AppData::default);
    app.app = merge_body(
        request.body.as_ref(),
        json!({
            "id": new_app_id,
            "copiedFromAppId": source_app_id,
            "name": string_field(request.body.as_ref(), "name").unwrap_or_else(|| "Copied Mock App".to_owned()),
            "realm": request.realm,
        }),
    );
    realm.apps.insert(new_app_id.clone(), app.clone());
    app.app
}

fn get_app(dataset: &MockDataset, request: &MockRequest) -> Value {
    let app_id = required_path(request, "appId");
    dataset
        .realms
        .get(&request.realm)
        .and_then(|realm| realm.apps.get(&app_id))
        .map(|app| app.app.clone())
        .unwrap_or_else(|| json!({ "id": app_id, "missing": true }))
}

fn update_app(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let app_id = required_path(request, "appId");
    let realm = realm_mut(dataset, &request.realm);
    let app = realm.apps.entry(app_id.clone()).or_insert_with(|| AppData {
        app: json!({ "id": app_id, "realm": request.realm }),
        ..AppData::default()
    });
    app.app = merge_body(request.body.as_ref(), app.app.clone());
    app.app.clone()
}

fn delete_app(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let app_id = required_path(request, "appId");
    let deleted = realm_mut(dataset, &request.realm)
        .apps
        .remove(&app_id)
        .is_some();
    json!({ "deleted": deleted, "appId": app_id })
}

fn create_table(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let app_id = required_query(request, "appId");
    let table_id = string_field(request.body.as_ref(), "id")
        .or_else(|| string_field(request.body.as_ref(), "tableId"))
        .or_else(|| string_field(request.body.as_ref(), "dbid"))
        .unwrap_or_else(|| next_id("table", &mut dataset.counters.table));
    let table = merge_body(
        request.body.as_ref(),
        json!({
            "id": table_id,
            "dbid": table_id,
            "appId": app_id,
            "name": string_field(request.body.as_ref(), "name").unwrap_or_else(|| "Mock Table".to_owned()),
        }),
    );
    app_mut(dataset, &request.realm, &app_id).tables.insert(
        table_id.clone(),
        TableData {
            table: table.clone(),
            ..TableData::default()
        },
    );
    table
}

fn get_app_tables(dataset: &MockDataset, request: &MockRequest) -> Value {
    let app_id = required_query(request, "appId");
    let tables = dataset
        .realms
        .get(&request.realm)
        .and_then(|realm| realm.apps.get(&app_id))
        .map(|app| {
            app.tables
                .values()
                .map(|table| table.table.clone())
                .collect()
        })
        .unwrap_or_else(Vec::new);
    Value::Array(tables)
}

fn get_table(dataset: &MockDataset, request: &MockRequest) -> Value {
    let table_id = required_path(request, "tableId");
    find_table(dataset, &request.realm, &table_id)
        .map(|table| table.table.clone())
        .unwrap_or_else(|| json!({ "id": table_id, "dbid": table_id, "missing": true }))
}

fn update_table(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let table_id = required_path(request, "tableId");
    let table = table_mut(dataset, &request.realm, None, &table_id);
    table.table = merge_body(request.body.as_ref(), table.table.clone());
    table.table.clone()
}

fn delete_table(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let table_id = required_path(request, "tableId");
    let app_id = request.query.get("appId").cloned();
    let deleted = realm_mut(dataset, &request.realm)
        .apps
        .iter_mut()
        .filter(|(candidate, _)| app_id.as_ref().is_none_or(|id| id == *candidate))
        .any(|(_, app)| app.tables.remove(&table_id).is_some());
    json!({ "deleted": deleted, "tableId": table_id })
}

fn create_field(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let table_id = required_query(request, "tableId");
    let field_id = string_field(request.body.as_ref(), "id")
        .or_else(|| string_field(request.body.as_ref(), "fieldId"))
        .unwrap_or_else(|| next_number(&mut dataset.counters.field).to_string());
    let field = merge_body(
        request.body.as_ref(),
        json!({
            "id": field_id.parse::<u64>().map(Value::from).unwrap_or_else(|_| Value::String(field_id.clone())),
            "fieldId": field_id,
            "tableId": table_id,
            "label": string_field(request.body.as_ref(), "label").unwrap_or_else(|| "Mock Field".to_owned()),
        }),
    );
    table_mut(dataset, &request.realm, None, &table_id)
        .fields
        .insert(field_id.clone(), field.clone());
    field
}

fn get_fields(dataset: &MockDataset, request: &MockRequest) -> Value {
    let table_id = required_query(request, "tableId");
    let fields = find_table(dataset, &request.realm, &table_id)
        .map(|table| table.fields.values().cloned().collect())
        .unwrap_or_else(Vec::new);
    Value::Array(fields)
}

fn get_field(dataset: &MockDataset, request: &MockRequest) -> Value {
    let table_id = required_query(request, "tableId");
    let field_id = required_path(request, "fieldId");
    find_table(dataset, &request.realm, &table_id)
        .and_then(|table| table.fields.get(&field_id))
        .cloned()
        .unwrap_or_else(
            || json!({ "id": field_id, "fieldId": field_id, "tableId": table_id, "missing": true }),
        )
}

fn update_field(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let table_id = required_query(request, "tableId");
    let field_id = required_path(request, "fieldId");
    let table = table_mut(dataset, &request.realm, None, &table_id);
    let existing = table
        .fields
        .remove(&field_id)
        .unwrap_or_else(|| json!({ "id": field_id, "fieldId": field_id, "tableId": table_id }));
    let field = merge_body(request.body.as_ref(), existing);
    table.fields.insert(field_id, field.clone());
    field
}

fn delete_fields(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let table_id = required_query(request, "tableId");
    let ids = field_ids_from_body(request.body.as_ref());
    let table = table_mut(dataset, &request.realm, None, &table_id);
    let deleted = if ids.is_empty() {
        let count = table.fields.len();
        table.fields.clear();
        count
    } else {
        ids.iter()
            .filter(|field_id| table.fields.remove(*field_id).is_some())
            .count()
    };
    json!({ "deletedFieldCount": deleted, "tableId": table_id })
}

fn upsert_records(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let table_id = table_from_body(request.body.as_ref());
    let rows = request
        .body
        .as_ref()
        .and_then(|body| body.get("data"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(Vec::new);

    let mut created_record_ids = Vec::new();
    let mut records = Vec::new();
    for row in rows {
        let record_id = next_number(&mut dataset.counters.record).to_string();
        created_record_ids.push(Value::String(record_id.clone()));
        records.push((record_id, row));
    }

    let table = table_mut(dataset, &request.realm, None, &table_id);
    for (record_id, row) in records {
        table.records.insert(
            record_id.clone(),
            json!({
                "recordId": record_id,
                "data": row,
            }),
        );
    }

    json!({
        "metadata": {
            "createdRecordIds": created_record_ids,
            "totalNumberOfRecordsProcessed": table.records.len(),
        },
        "data": table.records.values().cloned().collect::<Vec<_>>(),
    })
}

fn run_query(dataset: &MockDataset, request: &MockRequest) -> Value {
    let table_id = table_from_body(request.body.as_ref());
    let records = find_table(dataset, &request.realm, &table_id)
        .map(|table| table.records.values().cloned().collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);
    json!({
        "data": records,
        "fields": find_table(dataset, &request.realm, &table_id)
            .map(|table| table.fields.values().cloned().collect::<Vec<_>>())
            .unwrap_or_else(Vec::new),
        "metadata": {
            "tableId": table_id,
            "numRecords": records.len(),
            "totalRecords": records.len(),
        }
    })
}

fn delete_records(dataset: &mut MockDataset, request: &MockRequest) -> Value {
    let table_id = table_from_body(request.body.as_ref());
    let table = table_mut(dataset, &request.realm, None, &table_id);
    let deleted = table.records.len();
    table.records.clear();
    json!({ "numberDeleted": deleted, "tableId": table_id })
}

fn records_modified_since(dataset: &MockDataset, request: &MockRequest) -> Value {
    let table_id = table_from_body(request.body.as_ref());
    run_query(
        dataset,
        &MockRequest {
            path: request.path.clone(),
            query: request.query.clone(),
            path_params: request.path_params.clone(),
            body: Some(json!({ "from": table_id })),
            realm: request.realm.clone(),
        },
    )
}

fn generic_response(operation: &Operation, request: &MockRequest) -> Value {
    json!({
        "mock": true,
        "operationId": operation.operation_id,
        "tag": operation.tag,
        "method": operation.method,
        "path": request.path,
        "pathParams": request.path_params,
        "query": request.query,
        "body": request.body,
    })
}

fn realm_mut<'a>(dataset: &'a mut MockDataset, realm_id: &str) -> &'a mut RealmData {
    dataset.realms.entry(realm_id.to_owned()).or_default()
}

fn app_mut<'a>(dataset: &'a mut MockDataset, realm_id: &str, app_id: &str) -> &'a mut AppData {
    realm_mut(dataset, realm_id)
        .apps
        .entry(app_id.to_owned())
        .or_insert_with(|| AppData {
            app: json!({ "id": app_id, "realm": realm_id }),
            ..AppData::default()
        })
}

fn table_mut<'a>(
    dataset: &'a mut MockDataset,
    realm_id: &str,
    app_id: Option<&str>,
    table_id: &str,
) -> &'a mut TableData {
    if let Some(app_id) = app_id {
        return app_mut(dataset, realm_id, app_id)
            .tables
            .entry(table_id.to_owned())
            .or_insert_with(|| TableData {
                table: json!({ "id": table_id, "dbid": table_id, "appId": app_id }),
                ..TableData::default()
            });
    }

    let realm = realm_mut(dataset, realm_id);
    if let Some(app_id) = realm
        .apps
        .iter()
        .find_map(|(app_id, app)| app.tables.contains_key(table_id).then(|| app_id.to_owned()))
    {
        return realm
            .apps
            .get_mut(&app_id)
            .expect("app existed during table lookup")
            .tables
            .get_mut(table_id)
            .expect("table existed during table lookup");
    }

    realm
        .apps
        .entry("_orphan".to_owned())
        .or_insert_with(|| AppData {
            app: json!({ "id": "_orphan", "name": "Mock Orphan Tables", "realm": realm_id }),
            ..AppData::default()
        })
        .tables
        .entry(table_id.to_owned())
        .or_insert_with(|| TableData {
            table: json!({ "id": table_id, "dbid": table_id, "appId": "_orphan" }),
            ..TableData::default()
        })
}

fn find_table<'a>(
    dataset: &'a MockDataset,
    realm_id: &str,
    table_id: &str,
) -> Option<&'a TableData> {
    dataset
        .realms
        .get(realm_id)?
        .apps
        .values()
        .find_map(|app| app.tables.get(table_id))
}

fn next_id(prefix: &str, counter: &mut u64) -> String {
    format!("{prefix}_{}", next_number(counter))
}

fn next_number(counter: &mut u64) -> u64 {
    let value = *counter;
    *counter += 1;
    value
}

fn merge_body(body: Option<&Value>, base: Value) -> Value {
    let mut object = match base {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    if let Some(Value::Object(body)) = body {
        for (key, value) in body {
            object.insert(key.clone(), value.clone());
        }
    }
    Value::Object(object)
}

fn string_field(body: Option<&Value>, name: &str) -> Option<String> {
    body.and_then(|body| body.get(name)).and_then(|value| {
        value
            .as_str()
            .map(ToOwned::to_owned)
            .or_else(|| value.as_u64().map(|number| number.to_string()))
    })
}

fn table_from_body(body: Option<&Value>) -> String {
    string_field(body, "to")
        .or_else(|| string_field(body, "from"))
        .or_else(|| string_field(body, "tableId"))
        .unwrap_or_else(|| "_records".to_owned())
}

fn field_ids_from_body(body: Option<&Value>) -> Vec<String> {
    let Some(body) = body else {
        return Vec::new();
    };
    if let Some(array) = body.as_array() {
        return array.iter().filter_map(value_to_string).collect();
    }
    body.get("fieldIds")
        .or_else(|| body.get("ids"))
        .and_then(Value::as_array)
        .map(|array| array.iter().filter_map(value_to_string).collect())
        .unwrap_or_default()
}

fn value_to_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_u64().map(|number| number.to_string()))
}

fn required_query(request: &MockRequest, name: &str) -> String {
    request
        .query
        .get(name)
        .cloned()
        .unwrap_or_else(|| format!("missing-{name}"))
}

fn required_path(request: &MockRequest, name: &str) -> String {
    request
        .path_params
        .get(name)
        .cloned()
        .unwrap_or_else(|| format!("missing-{name}"))
}
