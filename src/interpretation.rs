//! `HatSpec` is a source schema, never a semantic authority. Preserve every
//! original field; only raw property values enter structural interpretation.
use crate::structure::{Decomposition, decompose};
use hat_specifications::{HatSemanticContribution, SemanticScalar};
use sem_lang_frontend::{
    CompileContext,
    current::structural::{StructuralBudget, StructuralInput},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterpretationContract {
    pub source: HatSemanticContribution,
    pub context: CompileContext,
    pub budget: StructuralBudget,
}
#[derive(Serialize)]
pub struct SourceObservation {
    pub locator: String,
    pub input: Option<StructuralInput>,
    pub failure: Option<&'static str>,
}
#[derive(Serialize)]
pub struct SourceRepresentation {
    /// The source owner retains this record in full. It must not be written as
    /// semantic state. Schema, labels, catalog IDs, revisions, classification,
    /// provenance and old interpretation fields remain source metadata.
    pub original: HatSemanticContribution,
    pub observations: Vec<SourceObservation>,
}

/// # Errors
/// Rejects a source with too many records or an unsupported source schema.
pub fn interpret(request: InterpretationContract) -> Result<SourceRepresentation, &'static str> {
    crate::structure::check_bytes(&request.source)?;
    let source = &request.source;
    let count = source.entities.len()
        + source.relations.len()
        + source.events.len()
        + source.assertions.len()
        + source.sources.len()
        + source.evidence.len()
        + source.bindings.len();
    if count > 256 || source.schema != hat_specifications::SEMANTIC_CONTRIBUTION_SCHEMA {
        return Err("source-contract-invalid");
    }
    let mut observations = vec![];
    let mut add = |locator: String, values: Result<Value, &'static str>| {
        let result = values.and_then(|document| {
            decompose(&Decomposition {
                reference: format!("{}#{locator}", source.canonical_source.id),
                document,
                pointer: String::new(),
                context: request.context.clone(),
                budget: request.budget.clone(),
            })
        });
        observations.push(match result {
            Ok(mut input) => {
                for node in &mut input.nodes {
                    node.locator = format!("{locator}{}", node.locator);
                }
                SourceObservation {
                    locator,
                    input: Some(input),
                    failure: None,
                }
            }
            Err(failure) => SourceObservation {
                locator,
                input: None,
                failure: Some(failure),
            },
        });
    };
    for (n, item) in source.entities.iter().enumerate() {
        add(
            format!("/entities/{n}/properties"),
            properties(&item.properties),
        );
    }
    for (n, item) in source.relations.iter().enumerate() {
        add(
            format!("/relations/{n}/properties"),
            properties(&item.properties),
        );
    }
    for (n, item) in source.events.iter().enumerate() {
        add(
            format!("/events/{n}/properties"),
            properties(&item.properties),
        );
    }
    for (n, item) in source.assertions.iter().enumerate() {
        add(
            format!("/assertions/{n}/value"),
            scalar(&item.value).map(|value| json!({"value":value})),
        );
    }
    // These categories carry identity/control/provenance, not an independently
    // interpretable observation. Keep them explicitly, never fabricate a value.
    for (kind, length) in [
        ("sources", source.sources.len()),
        ("evidence", source.evidence.len()),
        ("bindings", source.bindings.len()),
    ] {
        for n in 0..length {
            observations.push(SourceObservation {
                locator: format!("/{kind}/{n}"),
                input: None,
                failure: Some("source-metadata-only"),
            });
        }
    }
    Ok(SourceRepresentation {
        original: request.source,
        observations,
    })
}
fn properties(values: &BTreeMap<String, SemanticScalar>) -> Result<Value, &'static str> {
    values
        .iter()
        .map(|(key, value)| Ok((key.clone(), scalar(value)?)))
        .collect::<Result<serde_json::Map<_, _>, _>>()
        .map(Value::Object)
}
fn scalar(value: &SemanticScalar) -> Result<Value, &'static str> {
    Ok(match value {
        SemanticScalar::Null => Value::Null,
        SemanticScalar::Bool(v) => json!(v),
        SemanticScalar::Signed(v) => json!(v),
        SemanticScalar::Unsigned(v) => {
            json!(i64::try_from(*v).map_err(|_| "source-number-unsupported")?)
        }
        SemanticScalar::String(v) => json!(v),
        // Do not erase reference kind/units or coerce bytes/numbers into text.
        SemanticScalar::Float(_) | SemanticScalar::Bytes(_) | SemanticScalar::Reference(_) => {
            return Err("source-value-unsupported");
        }
    })
}
