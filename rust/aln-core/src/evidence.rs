// rust/aln-core/src/evidence.rs
use crate::hash::Digest256; // your own non-blacklisted hash primitive
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBinding {
    pub shard_id: String,
    pub kernel_module: String,
    pub kernel_version: String,
    pub proof_module: String,
    pub proof_hash: String,
    pub evidence_hex: String,
}

impl EvidenceBinding {
    pub fn canonical_bytes(&self, shard_payload_hash: &str) -> Vec<u8> {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.shard_id,
            self.kernel_module,
            self.kernel_version,
            self.proof_module,
            self.proof_hash,
            shard_payload_hash
        )
        .into_bytes()
    }

    pub fn verify(&self, shard_payload_hash: &str) -> Result<(), String> {
        let bytes = self.canonical_bytes(shard_payload_hash);
        let digest = Digest256::of(&bytes);
        let expected = hex::encode(digest.bytes());
        if expected != self.evidence_hex.to_lowercase() {
            Err("evidence_hex mismatch for shard binding".to_string())
        } else {
            Ok(())
        }
    }
}
