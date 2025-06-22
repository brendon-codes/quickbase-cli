use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

static OPERATIONS_JSON: &str = include_str!("operations.json");
static OPERATIONS: OnceLock<Vec<Operation>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub tag: String,
    pub summary: String,
    pub description: String,
    pub path_params: Vec<Parameter>,
    pub query_params: Vec<Parameter>,
    pub has_body: bool,
    pub body_required: bool,
    pub requires_realm: bool,
    pub requires_auth: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parameter {
    pub name: String,
    pub required: bool,
    #[serde(rename = "type")]
    pub kind: String,
}

pub fn operations() -> &'static [Operation] {
    OPERATIONS
        .get_or_init(|| {
            serde_json::from_str(OPERATIONS_JSON)
                .expect("checked-in Quickbase operation registry must be valid JSON")
        })
        .as_slice()
}

pub fn find_operation(operation_id: &str) -> Option<&'static Operation> {
    operations()
        .iter()
        .find(|operation| operation.operation_id == operation_id)
        .or_else(|| {
            operations()
                .iter()
                .find(|operation| operation.operation_id.eq_ignore_ascii_case(operation_id))
        })
}

pub fn operation_count() -> usize {
    operations().len()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::Value;

    use super::*;

    #[test]
    fn registry_contains_expected_operation_count() {
        assert_eq!(operation_count(), 67);
    }

    #[test]
    fn registry_operation_ids_match_reference_paths() {
        let reference: Value =
            yaml_serde::from_str(crate::quickbase::reference::QUICKBASE_REST_API_YAML)
                .expect("reference YAML parses");
        let paths = reference
            .get("paths")
            .and_then(Value::as_object)
            .expect("reference has paths");
        let methods = ["get", "post", "put", "delete", "patch"];
        let reference_ids = paths
            .values()
            .flat_map(|path_item| {
                methods.iter().filter_map(|method| {
                    path_item
                        .get(method)
                        .and_then(|operation| operation.get("operationId"))
                        .and_then(Value::as_str)
                })
            })
            .collect::<BTreeSet<_>>();
        let registry_ids = operations()
            .iter()
            .map(|operation| operation.operation_id.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(registry_ids, reference_ids);
    }

    #[test]
    fn lookup_accepts_exact_and_ascii_case_insensitive_ids() {
        assert_eq!(
            find_operation("getUsers").map(|operation| operation.operation_id.as_str()),
            Some("getUsers")
        );
        assert_eq!(
            find_operation("getusers").map(|operation| operation.operation_id.as_str()),
            Some("getUsers")
        );
    }
}
