use std::collections::BTreeMap;

use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get},
};
use serde_json::{Value, json};

use crate::quickbase::operation::{Operation, operations};

use super::state::{MockRequest, MockState};

const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn router(state: MockState) -> Router {
    Router::new()
        .route("/health", get(health))
        .fallback(any(handle))
        .with_state(state)
}

async fn health(State(state): State<MockState>) -> impl IntoResponse {
    json_response(
        StatusCode::OK,
        json!({
            "ok": true,
            "operationCount": state.operation_count(),
            "dataDir": state.data_dir(),
        }),
    )
}

async fn handle(State(state): State<MockState>, request: Request<Body>) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let path = uri.path().to_owned();
    let query = parse_query(uri.query());
    let Some((operation, path_params)) = find_route(method.as_str(), &path) else {
        return json_response(
            StatusCode::NOT_FOUND,
            json!({
                "message": "mock route not found",
                "method": method.as_str(),
                "path": path,
            }),
        );
    };

    if let Some(response) = auth_error(operation, &headers) {
        return response;
    }

    let body = match read_body(request).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let realm =
        header_string(&headers, "QB-Realm-Hostname").unwrap_or_else(|| "mock.realm".to_owned());
    match state.handle(
        operation,
        MockRequest {
            path,
            query,
            path_params,
            body,
            realm,
        },
    ) {
        Ok((status, body)) => json_response(status, body),
        Err(error) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "message": error.to_string(),
            }),
        ),
    }
}

fn find_route(method: &str, path: &str) -> Option<(&'static Operation, BTreeMap<String, String>)> {
    operations()
        .iter()
        .filter(|operation| operation.method == method)
        .filter_map(|operation| {
            match_template(&operation.path, path).map(|(literal_count, params)| {
                (
                    operation,
                    literal_count,
                    operation.path.matches('/').count(),
                    params,
                )
            })
        })
        .max_by_key(|(_, literal_count, segment_count, _)| (*literal_count, *segment_count))
        .map(|(operation, _, _, params)| (operation, params))
}

fn match_template(template: &str, path: &str) -> Option<(usize, BTreeMap<String, String>)> {
    let template_segments = split_path(template);
    let path_segments = split_path(path);
    if template_segments.len() != path_segments.len() {
        return None;
    }

    let mut literal_count = 0;
    let mut params = BTreeMap::new();
    for (template_segment, path_segment) in template_segments.iter().zip(path_segments.iter()) {
        if let Some(name) = template_segment
            .strip_prefix('{')
            .and_then(|segment| segment.strip_suffix('}'))
        {
            params.insert(name.to_owned(), (*path_segment).to_owned());
        } else if template_segment == path_segment {
            literal_count += 1;
        } else {
            return None;
        }
    }

    Some((literal_count, params))
}

fn split_path(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn auth_error(operation: &Operation, headers: &HeaderMap) -> Option<Response> {
    if operation.requires_realm && header_string(headers, "QB-Realm-Hostname").is_none() {
        return Some(json_response(
            StatusCode::UNAUTHORIZED,
            json!({
                "message": "missing required QB-Realm-Hostname header",
                "operationId": operation.operation_id,
            }),
        ));
    }

    if operation.requires_auth && header_string(headers, "Authorization").is_none() {
        return Some(json_response(
            StatusCode::UNAUTHORIZED,
            json!({
                "message": "missing required Authorization header",
                "operationId": operation.operation_id,
            }),
        ));
    }

    None
}

async fn read_body(request: Request<Body>) -> Result<Option<Value>, Response> {
    let bytes = to_bytes(request.into_body(), MAX_BODY_BYTES)
        .await
        .map_err(|error| {
            json_response(
                StatusCode::BAD_REQUEST,
                json!({
                    "message": format!("failed to read request body: {error}"),
                }),
            )
        })?;
    if bytes.is_empty() {
        return Ok(None);
    }

    parse_json_body(bytes)
}

fn parse_json_body(bytes: Bytes) -> Result<Option<Value>, Response> {
    serde_json::from_slice(&bytes).map(Some).map_err(|error| {
        json_response(
            StatusCode::BAD_REQUEST,
            json!({
                "message": format!("request body must be valid JSON: {error}"),
            }),
        )
    })
}

fn parse_query(query: Option<&str>) -> BTreeMap<String, String> {
    query
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode_url_component(name), decode_url_component(value))
        })
        .collect()
}

fn decode_url_component(value: &str) -> String {
    let mut decoded = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte as char);
                    index += 3;
                } else {
                    decoded.push('%');
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte as char);
                index += 1;
            }
        }
    }
    decoded
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn json_response(status: StatusCode, value: Value) -> Response {
    (status, axum::Json(value)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_routes_win_over_parameter_routes() {
        let (operation, params) = find_route("GET", "/fields/usage").expect("route exists");
        assert_eq!(operation.operation_id, "getFieldsUsage");
        assert!(params.is_empty());
    }

    #[test]
    fn parameter_routes_extract_values() {
        let (operation, params) = find_route("GET", "/tables/table_1").expect("route exists");
        assert_eq!(operation.operation_id, "getTable");
        assert_eq!(params.get("tableId").map(String::as_str), Some("table_1"));
    }
}
