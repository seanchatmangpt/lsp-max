//! Session conformance seal (lane G07).
//!
//! Accumulates the 5-value conformance vector
//! ⟨admitted, refused, residual, alu, load_store⟩ across a whole LSP
//! session (every `did_save` court run folds into the totals) and seals
//! the session totals into a fixed 80-byte BLAKE3 receipt at session
//! end (stop/shutdown path).
//!
//! 80-byte seal layout (fixed framing, lane G07):
//!
//! ```text
//! bytes 0..4    magic "CLSP"
//! bytes 4..8    framing version, u32 LE (= 1)
//! bytes 8..48   session totals: 5 x u64 LE
//!               (admitted, refused, residual, alu, load_store)
//! bytes 48..80  BLAKE3-256 digest over bytes 0..48 (binding tag)
//! ```
//!
//! The digest binds the magic, version, and counters, so flipping any
//! single counter flips the seal bytes (flip court below).

use crate::receipts::ReceiptVector;

/// Seal magic bytes.
pub const SEAL_MAGIC: [u8; 4] = *b"CLSP";
/// Seal framing version.
pub const SEAL_VERSION: u32 = 1;
/// Fixed seal size in bytes.
pub const SEAL_LEN: usize = 80;
/// Size of the digest-binding prefix (magic + version + counters).
pub const SEAL_PREFIX_LEN: usize = 48;

/// Session-total accumulator for the 5-value conformance vector.
///
/// Counter adds saturate: a runaway session can never panic the server
/// on receipt accounting (refusal-by-saturation, not abort).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionConformance {
    /// Functions the court admitted across the session.
    pub n_admitted: u64,
    /// Functions ERROR-refused across the session.
    pub n_refused: u64,
    /// Residual-allowed functions across the session.
    pub n_residual: u64,
    /// ALU-class instruction total across the session.
    pub n_alu: u64,
    /// Load/store-class instruction total across the session.
    pub n_load_store: u64,
}

impl SessionConformance {
    /// New zeroed accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one court-run vector into the session totals (saturating).
    pub fn record(&mut self, v: &ReceiptVector) {
        self.n_admitted = self.n_admitted.saturating_add(v.n_admitted as u64);
        self.n_refused = self.n_refused.saturating_add(v.n_refused as u64);
        self.n_residual = self.n_residual.saturating_add(v.n_residual as u64);
        self.n_alu = self.n_alu.saturating_add(v.n_alu as u64);
        self.n_load_store = self.n_load_store.saturating_add(v.n_load_store as u64);
    }

    /// Build the 48-byte seal prefix: magic + version + 5 x u64 LE.
    fn seal_prefix(&self) -> [u8; SEAL_PREFIX_LEN] {
        let mut p = [0u8; SEAL_PREFIX_LEN];
        p[0..4].copy_from_slice(&SEAL_MAGIC);
        p[4..8].copy_from_slice(&SEAL_VERSION.to_le_bytes());
        p[8..16].copy_from_slice(&self.n_admitted.to_le_bytes());
        p[16..24].copy_from_slice(&self.n_refused.to_le_bytes());
        p[24..32].copy_from_slice(&self.n_residual.to_le_bytes());
        p[32..40].copy_from_slice(&self.n_alu.to_le_bytes());
        p[40..48].copy_from_slice(&self.n_load_store.to_le_bytes());
        p
    }

    /// Seal the session totals into the fixed 80-byte BLAKE3 receipt.
    ///
    /// Stack-only: a fixed-array prefix plus `blake3::Hasher` streaming
    /// (the blake3 crate's hasher is heap-free), so the serialization
    /// path performs zero heap allocation (witnessed by the counting
    /// allocator court below).
    pub fn seal(&self) -> [u8; SEAL_LEN] {
        let prefix = self.seal_prefix();
        let digest = *blake3::Hasher::new().update(&prefix).finalize().as_bytes();
        let mut seal = [0u8; SEAL_LEN];
        seal[0..SEAL_PREFIX_LEN].copy_from_slice(&prefix);
        seal[SEAL_PREFIX_LEN..SEAL_LEN].copy_from_slice(&digest);
        seal
    }

    /// Decode counters back out of a seal (round-trip check surface).
    pub fn from_seal(seal: &[u8; SEAL_LEN]) -> Result<Self, SealDecodeError> {
        if seal[0..4] != SEAL_MAGIC {
            return Err(SealDecodeError::BadMagic);
        }
        if u32::from_le_bytes(seal[4..8].try_into().expect("fixed 4")) != SEAL_VERSION {
            return Err(SealDecodeError::BadVersion);
        }
        let rd = |r: std::ops::Range<usize>| -> u64 {
            u64::from_le_bytes(seal[r].try_into().expect("fixed 8"))
        };
        let tag = blake3::Hasher::new()
            .update(&seal[0..SEAL_PREFIX_LEN])
            .finalize();
        if seal[SEAL_PREFIX_LEN..SEAL_LEN] != *tag.as_bytes() {
            return Err(SealDecodeError::BadDigest);
        }
        Ok(Self {
            n_admitted: rd(8..16),
            n_refused: rd(16..24),
            n_residual: rd(24..32),
            n_alu: rd(32..40),
            n_load_store: rd(40..48),
        })
    }
}

/// Typed seal decode refusals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealDecodeError {
    /// Magic bytes are not "CLSP".
    BadMagic,
    /// Framing version mismatch.
    BadVersion,
    /// Digest tag does not bind the prefix (tampered counters).
    BadDigest,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn rv(a: u32, r: u32, res: u32, alu: u32, ls: u32) -> ReceiptVector {
        ReceiptVector {
            n_admitted: a,
            n_refused: r,
            n_residual: res,
            n_alu: alu,
            n_load_store: ls,
        }
    }

    #[test]
    fn session_totals_accumulate_across_runs() {
        let mut s = SessionConformance::new();
        s.record(&rv(2, 1, 0, 10, 4));
        s.record(&rv(1, 0, 3, 7, 9));
        assert_eq!(s.n_admitted, 3);
        assert_eq!(s.n_refused, 1);
        assert_eq!(s.n_residual, 3);
        assert_eq!(s.n_alu, 17);
        assert_eq!(s.n_load_store, 13);
    }

    #[test]
    fn seal_is_80_bytes_with_fixed_framing() {
        let mut s = SessionConformance::new();
        s.record(&rv(5, 2, 1, 100, 40));
        let seal = s.seal();
        assert_eq!(seal.len(), SEAL_LEN);
        assert_eq!(&seal[0..4], &SEAL_MAGIC);
        assert_eq!(u32::from_le_bytes(seal[4..8].try_into().unwrap()), 1);
        let decoded = SessionConformance::from_seal(&seal).expect("self-seal decodes");
        assert_eq!(decoded, s);
    }

    /// Anti-vacuity: perturbing ONE session counter must flip the seal
    /// bytes AND be refused by the decoder of the other seal.
    #[test]
    fn perturbing_one_counter_flips_seal_bytes() {
        let a = SessionConformance {
            n_admitted: 12,
            n_refused: 3,
            n_residual: 1,
            n_alu: 4242,
            n_load_store: 777,
        };
        let b = SessionConformance { n_alu: 4243, ..a };
        let sa = a.seal();
        let sb = b.seal();
        assert_ne!(
            sa, sb,
            "one-counter perturbation must change the receipt bytes"
        );
        // The digest tag binds the counters: decoding seal-a is fine, but
        // the perturbed prefix under seal-a's tag must be refused.
        let mut tampered = sa;
        tampered[32..40].copy_from_slice(&4243u64.to_le_bytes());
        assert_eq!(
            SessionConformance::from_seal(&tampered),
            Err(SealDecodeError::BadDigest),
            "digest must bind the counter prefix (non-vacuous binding)"
        );
    }

    /// Saturating accumulation: a near-u64::MAX counter saturates instead
    /// of panicking the server.
    #[test]
    fn counter_saturation_refuses_by_saturation() {
        let mut s = SessionConformance {
            n_admitted: u64::MAX,
            ..Default::default()
        };
        s.record(&rv(7, 0, 0, 0, 0));
        assert_eq!(s.n_admitted, u64::MAX);
    }

    // ---- Zero-allocation seal court (counting global allocator) ----
    //
    // Crate-single counting allocator (only one #[global_allocator] is
    // legal per compile unit): the receipts.rs zero-alloc court shares
    // this one via the pub(crate) watch helpers.

    pub(crate) struct CountingAlloc;
    thread_local! {
        static SEAL_ALLOC_WATCH: core::cell::Cell<u64> =
            const { core::cell::Cell::new(0) };
    }
    unsafe impl std::alloc::GlobalAlloc for CountingAlloc {
        unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
            SEAL_ALLOC_WATCH.with(|c| c.set(c.get() + 1));
            unsafe { std::alloc::System.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
            unsafe { std::alloc::System.dealloc(p, l) }
        }
    }

    #[global_allocator]
    static SEAL_GLOBAL: CountingAlloc = CountingAlloc;

    /// Reset the crate-wide allocation counter (court-scoped).
    pub(crate) fn alloc_watch_reset() {
        SEAL_ALLOC_WATCH.with(|c| c.set(0));
    }

    /// Read the crate-wide allocation counter (court-scoped).
    pub(crate) fn alloc_watch_get() -> u64 {
        SEAL_ALLOC_WATCH.with(|c| c.get())
    }

    #[test]
    fn seal_path_is_zero_allocation() {
        let s = SessionConformance {
            n_admitted: 9,
            n_refused: 2,
            n_residual: 1,
            n_alu: 800,
            n_load_store: 300,
        };
        alloc_watch_reset();
        let seal = s.seal();
        let allocs = alloc_watch_get();
        assert_eq!(seal.len(), SEAL_LEN);
        assert_eq!(
            allocs, 0,
            "seal serialization must allocate nothing (witnessed {allocs})"
        );
        // Anti-vacuity: the SAME counter must see allocations when a
        // Vec runs, so the zero above is a measured zero, not a dead
        // counter.
        let _v = vec![0u8; 64];
        let allocs_pos = SEAL_ALLOC_WATCH.with(|c| c.get());
        assert!(
            allocs_pos > 0,
            "counter is dead (no allocs seen for Vec) - vacuous court"
        );
    }
}
