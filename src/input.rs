//! Source-owner editable input bindings. The public declaration never contains paths.
//! A caller must obtain this private plan from its trusted input provider, not a browser.
use crate::structure::{Decomposition, decompose};
use sem_lang_frontend::current::structural::StructuralInput;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use zixcel_interaction::{InputDeclaration, InvokeRequest, validate_invoke};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerInput {
    pub declaration: InputDeclaration,
    pub source_revision: String,
    pub source: Decomposition,
    /// Explicit JSON pointers relative to the selected source record; owner-private.
    pub bindings: BTreeMap<String, String>,
}

impl OwnerInput {
    /// Recompute identity from exact declaration, fixed values and private bindings.
    /// This digest detects substitution; it is not installation trust or a Grant.
    /// # Errors
    /// Rejects an oversized or nonserializable provider plan.
    pub fn contract_ref(&self) -> Result<String, &'static str> {
        crate::structure::check_bytes(self)?;
        let mut declaration = self.declaration.clone();
        declaration.action.contract_revision.clear();
        let bytes = serde_json::to_vec(&(
            &declaration,
            &self.source_revision,
            &self.source,
            &self.bindings,
        ))
        .map_err(|_| "source-input-contract-invalid")?;
        Ok(format!(
            "source-input:{}",
            hex::encode(Sha256::digest(bytes))
        ))
    }

    /// # Errors
    /// Missing, aliased, nonscalar, sensitive or stale input declarations are rejected.
    pub fn validate(&self) -> Result<(), &'static str> {
        self.declaration
            .validate()
            .map_err(|_| "source-input-contract-invalid")?;
        if self.declaration.action.target != self.source.reference
            || self.declaration.action.contract_revision != self.contract_ref()?
            || !zixcel_interaction::valid_reference(&self.source_revision)
            || !self.declaration.action.expected_revision_required
            || self.bindings.keys().collect::<Vec<_>>()
                != self.declaration.fields.keys().collect::<Vec<_>>()
            || self.bindings.values().collect::<BTreeSet<_>>().len() != self.bindings.len()
            || self
                .declaration
                .fields
                .values()
                .any(|field| field.sensitive)
        {
            return Err("source-input-contract-invalid");
        }
        let record = self
            .source
            .document
            .pointer(&self.source.pointer)
            .filter(|v| v.is_object())
            .ok_or("source-selection-absent")?;
        for pointer in self.bindings.values() {
            if !pointer.starts_with('/')
                || pointer.len() > 4096
                || record
                    .pointer(pointer)
                    .is_none_or(|v| v.is_array() || v.is_object())
            {
                return Err("source-input-binding-invalid");
            }
        }
        // The existing decomposer remains the only source-shape producer.
        decompose(&self.source)?;
        Ok(())
    }

    /// Apply only declared editable values to a transient copy of the source.
    /// No mutation of source documents, semantic state, Role or control references.
    /// # Errors
    /// Refuses stale identity, undeclared fields, failed constraints and unavailable source.
    pub fn bind(&self, request: &InvokeRequest) -> Result<StructuralInput, &'static str> {
        self.validate()?;
        validate_invoke(&self.declaration.action, request, &self.source_revision)
            .map_err(|_| "source-input-request-rejected")?;
        self.declaration
            .validate_values(&request.input)
            .map_err(|_| "source-input-values-invalid")?;
        let mut document = self.source.document.clone();
        let record = document
            .pointer_mut(&self.source.pointer)
            .ok_or("source-selection-absent")?;
        for (field, value) in request
            .input
            .as_object()
            .ok_or("source-input-values-invalid")?
        {
            let pointer = self
                .bindings
                .get(field)
                .ok_or("source-input-binding-invalid")?;
            *record
                .pointer_mut(pointer)
                .ok_or("source-input-binding-invalid")? = value.clone();
        }
        decompose(&Decomposition {
            reference: self.source.reference.clone(),
            document,
            pointer: self.source.pointer.clone(),
            context: self.source.context.clone(),
            budget: self.source.budget.clone(),
        })
    }
}
