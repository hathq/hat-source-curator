//! HAT source adapters. Generic JSON structure belongs to Zixcel, not a language product.
use serde::Serialize;
pub use zixcel_json_structure::{
    Decomposition, SourceShape, SourceTree, StructureBudget, decompose,
};

/// Decode explicitly supplied graph storage scalars without assigning meaning.
/// # Errors
/// Rejects unknown scalar encodings and unsupported source selection.
pub fn decompose_graph(request: &Decomposition) -> Result<SourceTree, &'static str> {
    check_bytes(&request.document)?;
    let fields = request
        .document
        .pointer(&request.pointer)
        .and_then(serde_json::Value::as_object)
        .ok_or("source-selection-absent")?;
    let mut values = serde_json::Map::new();
    for (key, value) in fields {
        let object = value.as_object().ok_or("source-scalar-invalid")?;
        let kind = object
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or("source-scalar-invalid")?;
        if kind == "null" && object.len() == 1 {
            values.insert(key.clone(), serde_json::Value::Null);
            continue;
        }
        if object.len() != 2 {
            return Err("source-scalar-invalid");
        }
        let raw = object.get("value").ok_or("source-scalar-invalid")?;
        match (kind, raw) {
            ("string", serde_json::Value::String(_)) | ("bool", serde_json::Value::Bool(_)) => {
                values.insert(key.clone(), raw.clone());
            }
            ("signed", serde_json::Value::Number(number)) if number.is_i64() => {
                values.insert(key.clone(), raw.clone());
            }
            ("unsigned", serde_json::Value::Number(number)) if number.is_u64() => {
                values.insert(key.clone(), raw.clone());
            }
            _ => return Err("source-scalar-unsupported"),
        }
    }
    let mut result = decompose(&Decomposition {
        reference: request.reference.clone(),
        document: serde_json::Value::Object(values),
        pointer: String::new(),
        budget: request.budget.clone(),
    })?;
    for node in &mut result.nodes {
        node.locator = format!("{}{}", request.pointer, node.locator);
    }
    Ok(result)
}

pub(crate) fn check_bytes(value: &impl Serialize) -> Result<(), &'static str> {
    zixcel_json_structure::check_bytes(value, 65536)
}
