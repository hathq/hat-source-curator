//! Source-owner decomposition. Names, paths and grouping never become meaning.
use sem_lang_core::Value;
use sem_lang_frontend::{
    CompileContext,
    current::structural::{SourceNode, SourceShape, StructuralBudget, StructuralInput},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decomposition {
    pub reference: String,
    pub document: serde_json::Value,
    pub pointer: String,
    pub context: CompileContext,
    pub budget: StructuralBudget,
}

/// Extract one explicitly selected record. No lexical rules, synthetic term
/// identifiers, catalogs, definition adoption or hidden default selection.
/// # Errors
/// Rejects an absent selection, unsupported number or explicit resource bound.
pub fn decompose(request: &Decomposition) -> Result<StructuralInput, &'static str> {
    if request.reference.is_empty() || request.reference.len() > 512 || request.pointer.len() > 4096
    {
        return Err("source-reference-invalid");
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, &request.document).map_err(|_| "source-byte-limit")?;
    let source = request
        .document
        .pointer(&request.pointer)
        .ok_or("source-selection-absent")?;
    let mut nodes = vec![];
    let root = visit(source, &request.pointer, 0, &request.budget, &mut nodes)?;
    Ok(StructuralInput {
        reference: request.reference.clone(),
        root,
        nodes,
        constraints: vec![],
        context: request.context.clone(),
        budget: request.budget.clone(),
    })
}

/// Decode historical Graph storage scalars as source representation, without
/// treating node labels as semantic definitions. Original nodes remain at owner.
/// # Errors
/// Rejects unknown scalar encodings instead of flattening or guessing them.
pub fn decompose_graph(request: &Decomposition) -> Result<StructuralInput, &'static str> {
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
            ("string", serde_json::Value::String(_))
            | ("bool", serde_json::Value::Bool(_))
            | ("signed" | "unsigned", serde_json::Value::Number(_)) => {
                values.insert(key.clone(), raw.clone());
            }
            _ => return Err("source-scalar-unsupported"),
        }
    }
    let mut result = decompose(&Decomposition {
        reference: request.reference.clone(),
        document: serde_json::Value::Object(values),
        pointer: String::new(),
        context: request.context.clone(),
        budget: request.budget.clone(),
    })?;
    for node in &mut result.nodes {
        node.locator = format!("{}{}", request.pointer, node.locator);
    }
    Ok(result)
}

pub(crate) fn check_bytes(value: &impl Serialize) -> Result<(), &'static str> {
    serde_json::to_writer(&mut Counter(0), value).map_err(|_| "source-byte-limit")
}

struct Counter(usize);
impl std::io::Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("source bound"))?;
        if self.0 > 65_536 {
            return Err(std::io::Error::other("source bound"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn visit(
    value: &serde_json::Value,
    locator: &str,
    depth: usize,
    budget: &StructuralBudget,
    nodes: &mut Vec<SourceNode>,
) -> Result<String, &'static str> {
    if depth > budget.max_depth.min(32)
        || nodes.len() >= budget.max_nodes.min(1024)
        || locator.len() > 4096
    {
        return Err("source-structure-limit");
    }
    let id = format!("n{}", nodes.len());
    let index = nodes.len();
    nodes.push(SourceNode {
        identity: id.clone(),
        locator: locator.into(),
        shape: SourceShape::Unknown,
    });
    let shape = match value {
        serde_json::Value::Object(fields) => {
            let children = fields
                .iter()
                .map(|(name, child)| {
                    let path = format!("{locator}/{}", name.replace('~', "~0").replace('/', "~1"));
                    visit(child, &path, depth + 1, budget, nodes)
                })
                .collect::<Result<_, _>>()?;
            SourceShape::Record(children)
        }
        // Record is unordered: treating an ordered array as a record would lose
        // information. Keep the raw document outside interpretation instead.
        serde_json::Value::Array(_) => return Err("source-sequence-unsupported"),
        serde_json::Value::Null => SourceShape::Value(Value::Nil),
        serde_json::Value::Bool(v) => SourceShape::Value(Value::Bool(*v)),
        serde_json::Value::String(v) => SourceShape::Value(Value::Text(v.clone())),
        serde_json::Value::Number(v) => SourceShape::Value(if let Some(v) = v.as_i64() {
            Value::Integer(v)
        } else {
            return Err("source-number-unsupported");
        }),
    };
    nodes[index].shape = shape;
    Ok(id)
}
