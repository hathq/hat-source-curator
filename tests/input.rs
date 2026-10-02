use hat_source_curator::input::OwnerInput;
use serde_json::{Value, json};
use zixcel_interaction::InvokeRequest;

fn command(mode: &str, value: &Value) -> std::process::Output {
    use std::io::Write;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_hat-source-decompose"))
        .arg(mode)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(value).unwrap())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn plan() -> OwnerInput {
    let mut p: OwnerInput=serde_json::from_value(json!({
      "declaration":{"action":{"operation_id":"source/replace","target":"source:one","contract_revision":"pending",
        "availability":{"state":"available"},"expected_revision_required":true,
        "input_schema":{"type":"object","fields":{"opaque-field":{"type":"string","max_length":80}},"required":["opaque-field"]}},
        "fields":{"opaque-field":{"label":"Contact","sensitive":false}}},
      "source_revision":"original:one",
      "source":{"reference":"source:one","document":{"private":{"marker":"@","value":"before"}},"pointer":"/private",
        "context":{"subject":{"id":"owner","class":"person"},"evidence":[],"at":null,"locale":"en",
          "semantic_revision":"exact:semantic","context_refs":[],"context_scope":"CurrentRequest","observation_evidence":[]},
        "budget":{"max_nodes":128,"max_edges":256,"max_candidates":128,"max_steps":32768,"max_depth":32,"max_bytes":65536}},
      "bindings":{"opaque-field":"/value"}})).unwrap();
    p.declaration.action.contract_revision = p.contract_ref().unwrap();
    p
}
fn request(p: &OwnerInput) -> InvokeRequest {
    serde_json::from_value(
        json!({"operation_id":"source/replace","target":"source:one",
      "contract_revision":p.declaration.action.contract_revision,"input":{"opaque-field":"after"},
      "expected_revision":"original:one","request_reference":"request:one"}),
    )
    .unwrap()
}

#[test]
fn exact_private_binding_preserves_fixed_values_and_original_source() {
    let p = plan();
    p.validate().unwrap();
    let before = serde_json::to_value(&p).unwrap();
    let output = serde_json::to_value(p.bind(&request(&p)).unwrap()).unwrap();
    let values: Vec<_> = output["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["shape"]["Value"]["Text"].as_str())
        .collect();
    assert!(values.contains(&"after"));
    assert!(values.contains(&"@"));
    assert!(!values.contains(&"before"));
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    let public = serde_json::to_string(&p.declaration).unwrap();
    assert!(!public.contains("/private"));
    assert!(!public.contains("/value"));
    assert!(!public.contains("before"));
    let declaration = command("input-contract", &before);
    assert!(declaration.status.success());
    assert!(declaration.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&declaration.stdout).unwrap(),
        serde_json::to_value(&p.declaration).unwrap()
    );
    let bound = command(
        "input-bind",
        &json!({"plan":before,"submission":request(&p)}),
    );
    assert!(bound.status.success());
    assert!(bound.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&bound.stdout).unwrap(),
        output
    );
}

#[test]
fn forged_or_stale_inputs_never_rebind_by_label_or_path() {
    let p = plan();
    let original = serde_json::to_value(request(&p)).unwrap();
    for (key, value) in [
        ("expected_revision", json!("old")),
        ("contract_revision", json!("old")),
        ("input", json!({"Contact":"after"})),
        ("input", json!({"/value":"after"})),
        ("input", json!({"opaque-field":"after","roleRef":"forged"})),
    ] {
        let mut r = original.clone();
        r[key] = value;
        assert!(p.bind(&serde_json::from_value(r).unwrap()).is_err());
    }
    let mut p = plan();
    p.bindings.insert("opaque-field".into(), "/marker".into());
    assert!(p.bind(&request(&p)).is_err());
    let mut p = plan();
    p.declaration
        .fields
        .get_mut("opaque-field")
        .unwrap()
        .sensitive = true;
    p.declaration.action.contract_revision = p.contract_ref().unwrap();
    assert!(p.validate().is_err());
    let rejected = command(
        "input-bind",
        &json!({"plan":plan(),"submission":original,"control":"forged"}),
    );
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    let _: Value = original;
}
