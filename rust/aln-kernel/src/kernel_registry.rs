// Filename: rust/aln-kernel/src/kernel_registry.rs

use std::collections::HashMap;
use crate::error::ValidationError;

#[derive(Debug, Clone)]
pub struct KernelDescriptor {
    pub name:          &'static str,
    pub version:       &'static str,
    pub invariants:    Vec<&'static str>,
    pub proof_modules: Vec<&'static str>,
}

#[derive(Debug, Default)]
pub struct KernelRegistry {
    by_name_version: HashMap<(&'static str, &'static str), KernelDescriptor>,
}

impl KernelRegistry {
    pub fn register(&mut self, desc: KernelDescriptor) {
        self.by_name_version
            .insert((desc.name, desc.version), desc);
    }

    pub fn require(
        &self,
        kernel_module: &str,
        kernel_version: &str,
        proof_module: &str,
        invariant_ids: &[String],
    ) -> Result<(), ValidationError> {
        let key = (kernel_module, kernel_version);
        let desc = self.by_name_version.get(&key).ok_or_else(|| {
            ValidationError::semantic(
                "TITAN_KERNEL_UNKNOWN",
                format!(
                    "kernel {}@{} is not registered",
                    kernel_module, kernel_version
                ),
            )
        })?;

        if !desc.proof_modules.iter().any(|m| *m == proof_module) {
            return Err(ValidationError::semantic(
                "TITAN_PROOF_MODULE_MISMATCH",
                format!(
                    "proof module {} not registered for kernel {}@{}",
                    proof_module, kernel_module, kernel_version
                ),
            ));
        }

        for id in invariant_ids {
            if !desc.invariants.iter().any(|inv| inv == id) {
                return Err(ValidationError::semantic(
                    "TITAN_INVARIANT_UNSUPPORTED",
                    format!(
                        "invariant {} not supported by kernel {}@{}",
                        id, kernel_module, kernel_version
                    ),
                ));
            }
        }

        Ok(())
    }
}
