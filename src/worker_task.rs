use hat_specifications::{
    ACTION_RESULT_SCHEMA, ActionReference, HatActionResult, HatInvocationOutcome,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::worker::Lease;
use crate::worker_io::{canonical_directory, message, path, required, run_hatter, write_document};

const INPUT_SCHEMA: &str = "hathq://hat-source-curator/review-source-input/v1";
const OUTPUT_SCHEMA: &str = "hathq://hat-source-curator/review-source-output/v1";
const OPERATION_ID: &str = "hathq://vocabulary/action/review-source/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerInput {
    schema: String,
    scope_ref: String,
    revision: u64,
    subject_refs: Vec<String>,
}

#[derive(Serialize)]
struct WorkerOutput {
    schema: &'static str,
    scope_ref: String,
    revision: u64,
    subject_refs: Vec<String>,
}

pub(super) fn process(
    args: &BTreeMap<String, String>,
    common: &[String],
    control: &Path,
    lease: &Lease,
) -> Result<(), String> {
    let invocation = &lease.record.invocation;
    if invocation.operation_id != OPERATION_ID || invocation.input.owner_id != "zixcel-graph" {
        return Err("invocation is outside the hat-source-curator worker contract".into());
    }
    let input_store = canonical_directory(required(args, "input-store")?)?;
    let input_bytes = read_digest_document(&input_store, &invocation.input)?;
    let input: WorkerInput = serde_json::from_slice(&input_bytes).map_err(message)?;
    validate_input(&input, invocation.expected_projection_revision)?;
    let output = WorkerOutput {
        schema: OUTPUT_SCHEMA,
        scope_ref: input.scope_ref,
        revision: input.revision.checked_add(1).ok_or("revision overflow")?,
        subject_refs: input
            .subject_refs
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    let output_bytes = serde_json::to_vec(&output).map_err(message)?;
    let digest = hex::encode(Sha256::digest(&output_bytes));
    let output_store = canonical_directory(required(args, "output-store")?)?;
    write_document(&output_store.join(format!("{digest}.json")), &output_bytes)?;
    complete(args, common, control, lease, output.revision, &digest)?;
    println!("{{\"processed\":true,\"outputDigestSha256\":\"{digest}\"}}");
    Ok(())
}

fn complete(
    args: &BTreeMap<String, String>,
    common: &[String],
    control: &Path,
    lease: &Lease,
    revision: u64,
    digest: &str,
) -> Result<(), String> {
    let result = HatActionResult {
        schema: ACTION_RESULT_SCHEMA.into(),
        invocation_id: lease.record.invocation.invocation_id.clone(),
        operation_id: OPERATION_ID.into(),
        state_revision: lease.record.status.state_revision.saturating_add(1),
        projection_revision: revision,
        outcome: HatInvocationOutcome::Completed,
        output: Some(ActionReference {
            owner_id: "hat-source-curator".into(),
            reference: digest.into(),
            schema_id: OUTPUT_SCHEMA.into(),
            digest_sha256: digest.into(),
        }),
        reason_id: None,
        evidence_refs: Vec::new(),
    };
    let result_path = control.join(format!("result-{}.json", result.invocation_id));
    write_document(&result_path, &serde_json::to_vec(&result).map_err(message)?)?;
    run_hatter(
        args,
        "complete",
        common,
        &[
            "--worker-id",
            required(args, "worker-id")?,
            "--result-json",
            path(&result_path)?,
        ],
    )?;
    Ok(())
}

fn validate_input(value: &WorkerInput, expected_revision: u64) -> Result<(), String> {
    if value.schema != INPUT_SCHEMA
        || value.revision != expected_revision
        || value.scope_ref.is_empty()
        || value.scope_ref.len() > 96
        || value.subject_refs.len() > 256
        || value
            .subject_refs
            .iter()
            .any(|item| !hat_source_curator::valid_subject_reference(item))
    {
        return Err("input is outside the hat-source-curator operation schema".into());
    }
    Ok(())
}

fn read_digest_document(root: &Path, reference: &ActionReference) -> Result<Vec<u8>, String> {
    if reference.schema_id != INPUT_SCHEMA || reference.reference != reference.digest_sha256 {
        return Err("input reference is not content-addressed".into());
    }
    let bytes = fs::read(root.join(format!("{}.json", reference.reference))).map_err(message)?;
    if bytes.len() > 1_048_576 || hex::encode(Sha256::digest(&bytes)) != reference.digest_sha256 {
        return Err("input digest differs".into());
    }
    Ok(bytes)
}
