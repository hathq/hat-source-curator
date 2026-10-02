#![forbid(unsafe_code)]

use std::collections::BTreeSet;
pub mod interpretation;
pub mod structure;
pub mod input;

pub const PACKAGE_JSON: &str = include_str!("../hat.package.json");
pub const PACKAGE_ID: &str = "hat/source-curator";
pub const REPOSITORY_ID: &str = "hat-source-curator";
pub const OPERATION_ID: &str = "hathq://vocabulary/action/review-source/v1";
pub const TERM_IDS: &[&str] = &[
    "hathq://vocabulary/entity/knowledge-source/v1",
    "hathq://vocabulary/state/source-classification/v1",
    "hathq://vocabulary/action/review-source/v1",
];

/// HAT-owned domain state. Semantic vocabulary is not business state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewProjection {
    pub revision: u64,
    pub observed_subjects: u64,
    pub reviewed_subjects: u64,
    pub latest_subject_ref: Option<String>,
}

/// Closed domain events accepted by this HAT reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewEvent<'a> {
    SubjectObserved {
        previous_revision: u64,
        subject_ref: &'a str,
    },
    ReviewCompleted {
        previous_revision: u64,
        subject_ref: &'a str,
    },
}

/// Applies one typed domain event at the exact expected projection revision.
///
/// # Errors
///
/// Returns a stable reason when the revision differs, the subject reference is
/// invalid, a count overflows, or the projection revision cannot advance.
pub fn reduce(
    mut projection: ReviewProjection,
    event: ReviewEvent<'_>,
) -> Result<ReviewProjection, &'static str> {
    let (previous_revision, subject_ref, completed) = match event {
        ReviewEvent::SubjectObserved {
            previous_revision,
            subject_ref,
        } => (previous_revision, subject_ref, false),
        ReviewEvent::ReviewCompleted {
            previous_revision,
            subject_ref,
        } => (previous_revision, subject_ref, true),
    };
    if previous_revision != projection.revision {
        return Err("revision-conflict");
    }
    if !valid_subject_reference(subject_ref) {
        return Err("subject-reference-invalid");
    }
    projection.observed_subjects = projection
        .observed_subjects
        .checked_add(1)
        .ok_or("count-overflow")?;
    if completed {
        projection.reviewed_subjects = projection
            .reviewed_subjects
            .checked_add(1)
            .ok_or("count-overflow")?;
    }
    projection.latest_subject_ref = Some(subject_ref.to_owned());
    projection.revision = projection
        .revision
        .checked_add(1)
        .ok_or("revision-overflow")?;
    Ok(projection)
}

/// Maps typed domain state to the signed semantic catalog only at the adapter boundary.
#[must_use]
pub fn semantic_terms(projection: &ReviewProjection) -> BTreeSet<&'static str> {
    let mut terms = BTreeSet::new();
    if projection.observed_subjects > 0 {
        terms.insert(TERM_IDS[0]);
    }
    if projection.reviewed_subjects > 0 {
        terms.insert(TERM_IDS[1]);
        terms.insert(TERM_IDS[2]);
    }
    terms
}

#[must_use]
pub fn valid_subject_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'/' | b'.' | b'_' | b'-')
        })
}
