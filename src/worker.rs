// Modified by the Hatter downstream project, 2026.
// Purpose: make this external lifecycle owner mint and validate its provider incarnation.
use hat_specifications::{HatActionStatus, HatInvocation};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread;
use std::time::{Duration, Instant};

use crate::worker_io::{
    arguments, canonical_directory, common_args, message, path, required, run_hatter,
    write_document,
};
use crate::worker_task::process;

#[derive(Deserialize)]
pub(super) struct Lease {
    pub(super) record: LeaseRecord,
}

#[derive(Deserialize)]
pub(super) struct LeaseRecord {
    pub(super) status: HatActionStatus,
    pub(super) invocation: HatInvocation,
    admission: ExecutionAdmission,
}

#[derive(Deserialize)]
struct ExecutionAdmission {
    execution: ExecutionReferences,
}
#[derive(Deserialize)]
struct ExecutionReferences {
    provider: serde_json::Value,
}

fn provider_incarnation(owner: &str) -> serde_json::Value {
    use sha2::{Digest, Sha256};
    serde_json::json!({
        "owner_ref": owner,
        "provider_ref": {"owner_id":"hat-source-curator", "reference":"hat-source-curator-worker",
            "schema_id":"hathq://hat/provider/v1",
            "digest_sha256":hex::encode(Sha256::digest(b"hat-source-curator-worker/0.10.0"))},
        "generation_id": uuid::Uuid::new_v4().to_string()
    })
}

#[derive(Deserialize)]
struct RegistrationLease {
    worker_id: String,
    revision: u64,
}

pub(super) fn run() -> Result<(), String> {
    let args = arguments()?;
    let provider = provider_incarnation(required(&args, "worker-identity-ref")?);
    let registration = serde_json::json!({
        "provider_generation": provider,
        "worker_id": required(&args, "worker-id")?,
        "worker_identity_ref": required(&args, "worker-identity-ref")?,
        "placement_selection_digest_sha256": required(&args, "placement-digest")?,
        "expected_revision": 0,
        "ttl_seconds": 15
    });
    let control = canonical_directory(required(&args, "state-dir")?)?;
    let registration_path = control.join("registration-cas.json");
    write_document(
        &registration_path,
        &serde_json::to_vec(&registration).map_err(message)?,
    )?;
    let common = common_args(&args, "hat-source-curator")?;
    let response = run_hatter(
        &args,
        "worker-register",
        &common,
        &["--registration-json", path(&registration_path)?],
    )?;
    finish_registered(&args, &common, &control, &response, &provider)
}

fn finish_registered(
    args: &BTreeMap<String, String>,
    common: &[String],
    control: &std::path::Path,
    response: &[u8],
    provider: &serde_json::Value,
) -> Result<(), String> {
    let registration: RegistrationLease = serde_json::from_slice(response).map_err(message)?;
    if registration.worker_id != required(args, "worker-id")? || registration.revision == 0 {
        return Err("worker registration response differs from the requested owner".into());
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        if let Some(lease) = wait_for_claim(args, common)? {
            if &lease.record.admission.execution.provider != provider {
                return Err("accepted provider incarnation is not this adapter instance".into());
            }
            process(args, common, control, &lease)
        } else {
            println!("{{\"processed\":false}}");
            Ok(())
        }
    }))
    .map_err(|_| "worker task panicked".to_owned())
    .and_then(std::convert::identity);
    let revision = registration.revision.to_string();
    let unregister = run_hatter(
        args,
        "worker-unregister",
        common,
        &[
            "--worker-id",
            required(args, "worker-id")?,
            "--expected-revision",
            &revision,
        ],
    )
    .map(|_| ());
    match (result, unregister) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(format!("{error}; worker unregister failed: {cleanup}")),
    }
}

fn wait_for_claim(
    args: &BTreeMap<String, String>,
    common: &[String],
) -> Result<Option<Lease>, String> {
    let seconds = args
        .get("wait-seconds")
        .map_or(Ok(10), |value| value.parse::<u64>())
        .map_err(message)?;
    if seconds > 30 {
        return Err("--wait-seconds exceeds 30".into());
    }
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        let claim = run_hatter(
            args,
            "claim",
            common,
            &["--worker-id", required(args, "worker-id")?],
        )?;
        if let Some(lease) = serde_json::from_slice::<Option<Lease>>(&claim).map_err(message)? {
            return Ok(Some(lease));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod incarnation_tests {
    #[test]
    fn external_owner_allocates_distinct_incarnations_not_from_the_endpoint() {
        let live = super::provider_incarnation("owner/test");
        let mut generations = std::collections::BTreeSet::new();
        generations.insert(live["generation_id"].to_string());
        for _ in 0..100 {
            let replacement = super::provider_incarnation("owner/test");
            assert_eq!(replacement["provider_ref"], live["provider_ref"]);
            assert!(generations.insert(replacement["generation_id"].to_string()));
        }
    }
}
