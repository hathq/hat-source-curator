use hat_source_curator::{
    PACKAGE_ID, PACKAGE_JSON, REPOSITORY_ID, ReviewEvent, ReviewProjection, TERM_IDS, reduce,
    semantic_terms, valid_subject_reference,
};

#[test]
fn package_identity_and_typed_reducer_contract_are_present() {
    let value: serde_json::Value = serde_json::from_str(PACKAGE_JSON).expect("package JSON");
    assert_eq!(value["schema"], "hathq://hat/package/v2");
    assert_eq!(value["package_id"], PACKAGE_ID);
    assert_eq!(value["repository_id"], REPOSITORY_ID);
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["manifest"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        value["operations"][0]["context_plan"]["unresolved_policy"],
        "record-unresolved"
    );
    assert_eq!(
        value["operations"][0]["reducer"]["strategy"],
        "typed-domain-event"
    );
    assert_eq!(
        value["operations"][0]["reducer"]["conflict_policy"],
        "reject"
    );
    assert_eq!(value["operations"][0]["handler"]["kind"], "declarative-hat");
}

#[test]
fn reducer_owns_typed_state_and_semantic_terms_exist_only_at_the_adapter_boundary() {
    assert!(valid_subject_reference("subject:record-1"));
    assert!(!valid_subject_reference("similar human text"));
    let projection = ReviewProjection {
        revision: 4,
        observed_subjects: 0,
        reviewed_subjects: 0,
        latest_subject_ref: None,
    };
    assert_eq!(
        reduce(
            projection.clone(),
            ReviewEvent::SubjectObserved {
                previous_revision: 3,
                subject_ref: "subject:record-1",
            }
        ),
        Err("revision-conflict")
    );
    assert_eq!(
        reduce(
            projection.clone(),
            ReviewEvent::SubjectObserved {
                previous_revision: 4,
                subject_ref: "natural language is not a reference",
            }
        ),
        Err("subject-reference-invalid")
    );
    let observed = reduce(
        projection,
        ReviewEvent::SubjectObserved {
            previous_revision: 4,
            subject_ref: "subject:record-1",
        },
    )
    .expect("typed observation");
    assert_eq!(observed.revision, 5);
    assert_eq!(observed.observed_subjects, 1);
    assert_eq!(
        semantic_terms(&observed),
        [TERM_IDS[0]].into_iter().collect()
    );
    let reviewed = reduce(
        observed,
        ReviewEvent::ReviewCompleted {
            previous_revision: 5,
            subject_ref: "subject:record-1",
        },
    )
    .expect("typed review");
    assert_eq!(reviewed.revision, 6);
    assert_eq!(reviewed.reviewed_subjects, 1);
    assert_eq!(
        semantic_terms(&reviewed),
        TERM_IDS.iter().copied().collect()
    );
}
