use hat_source_curator::structure::{Decomposition, decompose};
use serde_json::json;

#[test]
fn locators_preserve_source_without_semantic_lookup_and_limits_fail_closed() {
    let request: Decomposition = serde_json::from_value(json!({
        "reference":"source:one","document":{"alpha":{"x1":{"marker":"@","value":"a@example.test"}}},"pointer":"/alpha/x1",
        "context":{"subject":{"id":"owner","class":"person"},"evidence":[],"at":null,"locale":"en","semantic_revision":"a".repeat(64),"context_refs":[],"context_scope":"CurrentRequest","observation_evidence":[]},
        "budget":{"max_nodes":128,"max_edges":256,"max_candidates":128,"max_steps":32768,"max_depth":32,"max_bytes":65536}
    })).unwrap();
    let result = decompose(&request).unwrap();
    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.nodes[0].locator, "/alpha/x1");
    assert!(result.constraints.is_empty());
    let encoded = serde_json::to_string(&result).unwrap();
    assert!(encoded.contains("a@example.test"));
    assert!(!encoded.contains("TermRef"));
    let mut request = request;
    request.pointer = "/absent".into();
    assert!(decompose(&request).is_err());
    request.pointer = "/alpha/x1".into();
    request.budget.max_nodes = 1;
    assert!(decompose(&request).is_err());
    request.budget.max_nodes = 128;
    request.document = json!({"alpha":{"x1":[1,2]}});
    assert_eq!(
        decompose(&request).unwrap_err(),
        "source-sequence-unsupported"
    );
    request.document = json!({"foo":{"person":{"misc":{"code":"unresolved","flag":true}}}});
    request.pointer = "/foo/person/misc".into();
    let group = decompose(&request).unwrap();
    assert_eq!(group.nodes[0].locator, "/foo/person/misc");
    assert_eq!(
        group.nodes[0].shape,
        sem_lang_frontend::current::structural::SourceShape::Record(vec!["n1".into(), "n2".into()])
    );
    assert_eq!(group.nodes[1].locator, "/foo/person/misc/code");
    assert_eq!(group.nodes[2].locator, "/foo/person/misc/flag");
    assert!(group.constraints.is_empty());
    assert!(!serde_json::to_string(&group).unwrap().contains("TermRef"));
}

#[test]
fn contribution_metadata_never_becomes_meaning_and_original_is_preserved() {
    use hat_source_curator::interpretation::{InterpretationContract, interpret};
    let reference = json!({"schema":"opaque-schema","id":"opaque-source","revision":9,"digest_sha256":"a".repeat(64)});
    let original = json!({"schema":hat_specifications::SEMANTIC_CONTRIBUTION_SCHEMA,
        "producer_hat":reference,"canonical_source":reference,"source_revision":9,"content_digest_sha256":"b".repeat(64),
        "catalogs":[],"sources":[],"entities":[{"identity":reference,"types":["pretend-email-definition"],
        "properties":{"alpha":{"kind":"string","value":"@"},"x1":{"kind":"string","value":"a@example.test"}}}],
        "relations":[],"events":[],"assertions":[],"evidence":[],"bindings":[]});
    let context = json!({"subject":{"id":"owner","class":"person"},"evidence":[],"at":null,"locale":"en","semantic_revision":"a".repeat(64),"context_refs":[],"context_scope":"CurrentRequest","observation_evidence":[]});
    let request:InterpretationContract=serde_json::from_value(json!({"source":original,"context":context,"budget":sem_lang_frontend::current::structural::StructuralBudget::default()})).unwrap();
    let result = interpret(request).unwrap();
    assert_eq!(serde_json::to_value(&result.original).unwrap(), original);
    assert_eq!(result.observations.len(), 1);
    let input = result.observations[0].input.as_ref().unwrap();
    assert_eq!(input.nodes.len(), 3);
    assert!(input.constraints.is_empty());
    assert!(
        !serde_json::to_string(input)
            .unwrap()
            .contains("pretend-email-definition")
    );
    let mut source = result.original;
    source.entities[0].properties.insert(
        "x1".into(),
        hat_specifications::SemanticScalar::Reference(source.canonical_source.clone()),
    );
    let request = InterpretationContract {
        source,
        context: serde_json::from_value(context).unwrap(),
        budget: sem_lang_frontend::current::structural::StructuralBudget::default(),
    };
    let result = interpret(request).unwrap();
    assert_eq!(
        result.observations[0].failure,
        Some("source-value-unsupported")
    );
    assert!(result.observations[0].input.is_none());
    assert!(matches!(
        result.original.entities[0].properties["x1"],
        hat_specifications::SemanticScalar::Reference(_)
    ));
}
