// rust/aln-core/src/admission.rs
use crate::{evidence::EvidenceBinding, lyapunov::LyapunovKernel};

pub fn admit_proof_carrying_shard<K: LyapunovKernel>(
    binding: &EvidenceBinding,
    shard_payload: &[u8],
    kernel: &K,
) -> Result<(), String> {
    let payload_hash = K::payload_hash(shard_payload);
    binding.verify(&payload_hash)?;
    kernel.check_pre_admission(shard_payload)
}
