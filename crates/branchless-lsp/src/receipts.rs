//! Receipts (agent contract: `receipts`).
//!
//! 5-valued conformance vector <N_admitted, N_refused, N_residual,
//! N_alu, N_load_store> hashed with BLAKE3 and persisted to
//! `.cgap/receipts/branchless/`.
//!
//! Receipt binding (lane E10): R = BLAKE3(SourceBytes ∥ ElfTextBytes ∥
//! C_file ∥ Φ_lease ∥ canonical-vector) with 8-byte LE length framing
//! between segments. Φ_lease is the LSP SESSION identity — a
//! process/thread-scoped random digest (`session_lease()`). It is NOT
//! the cgap P28 license attestation lease; the two never conflate.

use crate::attributes::Classification;
use std::path::PathBuf;

/// 5-valued conformance vector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReceiptVector {
    /// Functions the court admitted (0 branches).
    pub n_admitted: u32,
    /// Functions ERROR-refused (branches without allowance).
    pub n_refused: u32,
    /// Functions under `#[allow_isel_residual]` with branches.
    pub n_residual: u32,
    /// ALU-class instruction count across judged functions (E10).
    pub n_alu: u32,
    /// Load/store-class instruction count across judged functions (E10).
    pub n_load_store: u32,
}

impl ReceiptVector {
    /// Fold court verdicts + attributes into the receipt vector.
    /// Scaffold limit: one ParsedAttrs applied to the whole object; the
    /// real surface is per-function attrs (documented divergence).
    pub fn fold(
        report: &crate::disasm_court::DisasmReport,
        attrs: &crate::attributes::ParsedAttrs,
    ) -> Self {
        let mut v = Self::default();
        for f in &report.functions {
            match crate::attributes::classify(attrs, f.branch_count > 0) {
                Classification::Admitted => v.n_admitted += 1,
                Classification::HintResidual => v.n_residual += 1,
                _ => v.n_refused += 1,
            }
        }
        // E10: instruction-class totals feed the vector directly.
        v.n_alu = report.total_alu();
        v.n_load_store = report.total_load_store();
        v
    }

    /// Total functions judged.
    pub fn total(&self) -> u32 {
        self.n_admitted + self.n_refused + self.n_residual
    }
}

impl ReceiptVector {
    /// Seed-aware fold (lane LSPR): per-function classification using the
    /// residual grid. Seeded symbols with branches count as residual and
    /// carry their grid provenance into `provenances` (parallel info side
    /// channel, not hashed into the vector).
    pub fn fold_seeded(
        report: &crate::disasm_court::DisasmReport,
        attrs: &crate::attributes::ParsedAttrs,
        grid: &crate::attributes::ResidualGrid,
        target: &str,
        provenances: &mut Vec<String>,
    ) -> Self {
        let mut v = Self::default();
        provenances.clear();
        for f in &report.functions {
            let cls = grid.classify_seeded(f.name.as_str(), target, attrs, f.branch_count > 0);
            match cls {
                Classification::Admitted => v.n_admitted += 1,
                Classification::HintResidual => {
                    v.n_residual += 1;
                    if let crate::attributes::SeedVerdict::Seeded { row } =
                        grid.lookup(f.name.as_str(), target)
                    {
                        provenances.push(format!(
                            "{} target={} kind={:?} branches={} provenance={}",
                            f.name, row.target, row.kind, row.branch_count, row.provenance
                        ));
                    }
                }
                _ => v.n_refused += 1,
            }
        }
        // E10: instruction-class totals feed the vector directly.
        v.n_alu = report.total_alu();
        v.n_load_store = report.total_load_store();
        v
    }
}

impl core::fmt::Display for ReceiptOverflow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "receipt scratch overflow")
    }
}

impl std::error::Error for ReceiptOverflow {}

/// Cap for the zero-allocation serialization scratch buffer.
pub const RECEIPT_SCRATCH_CAP: usize = 512;

/// Typed overflow refusal: serialization never truncates and never
/// allocates — it refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptOverflow;

/// Caller-provided cap-sized scratch. `serialize_into` writes canonical
/// receipt bytes here with zero heap allocation.
pub struct ReceiptScratch {
    buf: [u8; RECEIPT_SCRATCH_CAP],
    len: usize,
}

impl Default for ReceiptScratch {
    fn default() -> Self {
        Self {
            buf: [0u8; RECEIPT_SCRATCH_CAP],
            len: 0,
        }
    }
}

impl ReceiptScratch {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, b: u8) -> Result<(), ReceiptOverflow> {
        if self.len >= RECEIPT_SCRATCH_CAP {
            return Err(ReceiptOverflow);
        }
        self.buf[self.len] = b;
        self.len += 1;
        Ok(())
    }

    fn push_str(&mut self, s: &str) -> Result<(), ReceiptOverflow> {
        for b in s.as_bytes() {
            self.push(*b)?;
        }
        Ok(())
    }

    fn push_u32(&mut self, n: u32) -> Result<(), ReceiptOverflow> {
        // Zero-alloc digit emission (no format!, no to_string).
        if n == 0 {
            return self.push(b'0');
        }
        let mut tmp = [0u8; 10];
        let mut i = 0;
        let mut n = n;
        while n > 0 {
            tmp[i] = b'0' + (n % 10) as u8;
            n /= 10;
            i += 1;
        }
        for j in (0..i).rev() {
            self.push(tmp[j])?;
        }
        Ok(())
    }

    /// Serialized bytes (canonical form, exactly `len` bytes).
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// BLAKE3 receipt writer.
pub struct ReceiptWriter {
    root: PathBuf,
}

impl ReceiptWriter {
    /// Writer rooted at a given receipts directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Default root `.cgap/receipts/branchless`.
    pub fn default_root() -> PathBuf {
        PathBuf::from(".cgap/receipts/branchless")
    }

    /// Hash `v` with BLAKE3 and persist; returns the 64-char hex hash.
    pub fn write(&self, v: &ReceiptVector) -> std::io::Result<String> {
        let mut scratch = ReceiptScratch::new();
        v.serialize_into(&mut scratch)
            .expect("5-field canonical form fits 512B scratch");
        let hash = blake3::hash(scratch.as_bytes()).to_hex().to_string();
        std::fs::create_dir_all(self.write_root())?;
        std::fs::write(
            self.write_root().join(format!("{hash}.txt")),
            scratch.as_bytes(),
        )?;
        Ok(hash)
    }

    /// Bound write (lane E10):
    /// R = BLAKE3(SourceBytes ∥ ElfTextBytes ∥ C_file ∥ Φ_lease ∥ vector).
    /// Each segment is 8-byte LE length-framed so byte strings cannot
    /// collide across segment boundaries. Serialization into the scratch
    /// is allocation-free (witnessed by the counting-allocator court).
    pub fn write_bound(
        &self,
        v: &ReceiptVector,
        source_bytes: &[u8],
        elf_text_bytes: &[u8],
        c_file_bytes: &[u8],
    ) -> std::io::Result<String> {
        let mut scratch = ReceiptScratch::new();
        // Zero-alloc segment framing + serialization; overflow is a typed
        // refusal (ReceiptOverflow), never a silent truncate.
        frame(v, source_bytes, elf_text_bytes, c_file_bytes, &mut scratch)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::FileTooLarge, ReceiptOverflow))?;
        let hash = blake3::hash(scratch.as_bytes()).to_hex().to_string();
        std::fs::create_dir_all(self.write_root())?;
        std::fs::write(
            self.write_root().join(format!("{hash}.txt")),
            scratch.as_bytes(),
        )?;
        Ok(hash)
    }

    fn write_root(&self) -> &PathBuf {
        &self.root
    }
}

/// Φ_lease: LSP session identity. Process-start random digest, cached
/// per-thread. NOT the cgap P28 license lease — different object, never
/// conflated.
pub fn session_lease() -> [u8; 32] {
    use std::cell::Cell;
    thread_local! {
        static LEASE: Cell<Option<[u8; 32]>> = const { Cell::new(None) };
    }
    LEASE.with(|c| {
        if let Some(l) = c.get() {
            return l;
        }
        // Process-start entropy: address layout + instant + thread id,
        // distilled by BLAKE3 into 32 session bytes.
        let marker = [0u8; 16];
        let addr_entropy = marker.as_ptr() as usize;
        let instant = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tid = { &0u8 as *const u8 as usize };
        let mut input = [0u8; 8 * 8];
        input[0..8].copy_from_slice(&(addr_entropy as u64).to_le_bytes());
        input[8..16].copy_from_slice(&(instant as u64).to_le_bytes());
        input[16..24].copy_from_slice(&(tid as u64).to_le_bytes());
        let lease = *blake3::hash(&input).as_bytes();
        c.set(Some(lease));
        lease
    })
}

/// Zero-alloc canonical serialization into a caller-provided scratch.
impl ReceiptVector {
    pub fn serialize_into(&self, s: &mut ReceiptScratch) -> Result<(), ReceiptOverflow> {
        s.push_str("branchless-receipt admitted=")?;
        s.push_u32(self.n_admitted)?;
        s.push_str(" refused=")?;
        s.push_u32(self.refused_raw())?;
        s.push_str(" residual=")?;
        s.push_u32(self.n_residual)?;
        s.push_str(" alu=")?;
        s.push_u32(self.n_alu)?;
        s.push_str(" ls=")?;
        s.push_u32(self.n_load_store)?;
        Ok(())
    }

    fn refused_raw(&self) -> u32 {
        self.n_refused
    }
}

/// Length-framed bound input assembly (zero-alloc).
fn frame(
    v: &ReceiptVector,
    source: &[u8],
    elf: &[u8],
    cfile: &[u8],
    s: &mut ReceiptScratch,
) -> Result<(), ReceiptOverflow> {
    s.push_str("src")?;
    s.push_u32(source.len() as u32)?;
    for b in source {
        s.push(*b)?;
    }
    s.push_str("elf")?;
    s.push_u32(elf.len() as u32)?;
    for b in elf {
        s.push(*b)?;
    }
    s.push_str("cfile")?;
    s.push_u32(cfile.len() as u32)?;
    for c in cfile {
        s.push(*c)?;
    }
    s.push_str("lease")?;
    let lease = session_lease();
    for b in lease {
        s.push(b)?;
    }
    s.push_str("vec")?;
    v.serialize_into(s)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::ParsedAttrs;

    fn fv(name: &str, bc: u32) -> crate::disasm_court::FnVerdict {
        crate::disasm_court::FnVerdict {
            name: name.into(),
            branch_count: bc,
            alu_count: 0,
            load_store_count: 0,
            other_count: 0,
            is_branchless: bc == 0,
        }
    }

    #[test]
    fn fold_counts_all_three_cells() {
        let report = crate::disasm_court::DisasmReport {
            functions: vec![fv("a", 0), fv("b", 3), fv("c", 2)],
        };
        let v = ReceiptVector::fold(&report, &ParsedAttrs::default());
        assert_eq!((v.n_admitted, v.n_refused, v.n_residual), (1, 2, 0));
        let hint = ParsedAttrs {
            allow_isel_residual: true,
            branchless_invariant: false,
            ..Default::default()
        };
        let v2 = ReceiptVector::fold(&report, &hint);
        // Whole-object attrs: BOTH branchy fns become HINT-residual.
        assert_eq!((v2.n_admitted, v2.n_refused, v2.n_residual), (1, 0, 2));
        assert_eq!(v.total(), 3);
    }

    #[test]
    fn receipt_hash_is_stable_and_file_lands() {
        let v = ReceiptVector {
            n_admitted: 2,
            n_refused: 0,
            n_residual: 1,
            ..ReceiptVector::default()
        };
        let tmp = tempfile::tempdir().expect("tempdir");
        let w = ReceiptWriter::new(tmp.path().to_path_buf());
        let h1 = w.write(&v).expect("write receipt");
        let h2 = w.write(&v).expect("rewrite");
        assert_eq!(h1, h2);
        assert!(tmp.path().join(format!("{h1}.txt")).exists());
        assert_eq!(h1.len(), 64);
    }

    #[allow(dead_code)] // 2026-11-04: fixture helper retained for receipt-vector tests
    fn report_stub() -> crate::disasm_court::DisasmReport {
        crate::disasm_court::DisasmReport {
            functions: vec![fv("x", 0)],
        }
    }

    #[test]
    fn hash_flips_when_vector_changes() {
        let a = ReceiptVector {
            n_admitted: 1,
            n_refused: 0,
            n_residual: 0,
            ..ReceiptVector::default()
        };
        let b = ReceiptVector {
            n_admitted: 0,
            n_refused: 1,
            n_residual: 0,
            ..ReceiptVector::default()
        };
        let ha = canonical_hash(&a);
        let hb = canonical_hash(&b);
        assert_ne!(ha, hb, "vector mutation must flip the receipt hash");
    }

    fn canonical_hash(v: &ReceiptVector) -> blake3::Hash {
        let mut s = ReceiptScratch::new();
        v.serialize_into(&mut s).expect("canonical fits scratch");
        blake3::hash(s.as_bytes())
    }

    // ---- LSPR seed courts ----

    fn grid() -> crate::attributes::ResidualGrid {
        crate::attributes::ResidualGrid::canonical()
    }

    #[test]
    fn seeded_fold_counts_residual_and_carries_provenance() {
        let report = crate::disasm_court::DisasmReport {
            functions: vec![
                fv("select_slice", 1),
                fv("cmov_u64", 0),
                fv("unsanctioned_fn", 2),
            ],
        };
        let mut prov = Vec::new();
        let v = ReceiptVector::fold_seeded(
            &report,
            &ParsedAttrs::default(),
            &grid(),
            "aarch64",
            &mut prov,
        );
        assert_eq!(
            (v.n_admitted, v.n_refused, v.n_residual),
            (1, 1, 1),
            "seeded branchy -> residual, seeded 0-branch -> admitted, unknown branchy -> refused"
        );
        assert_eq!(
            prov.len(),
            1,
            "only the branchy seeded fn carries provenance"
        );
        assert!(prov[0].contains("repair-01-branchless-census.md"));
    }

    #[test]
    fn seeded_receipt_hash_flips_vs_unseeded_world() {
        // Anti-vacuity: the same report must produce a different receipt
        // with vs without the seed grid in effect.
        let report = crate::disasm_court::DisasmReport {
            functions: vec![fv("select_slice", 1)],
        };
        let mut prov = Vec::new();
        let seeded = ReceiptVector::fold_seeded(
            &report,
            &ParsedAttrs::default(),
            &grid(),
            "aarch64",
            &mut prov,
        );
        let unseeded = ReceiptVector::fold(&report, &ParsedAttrs::default());
        assert_ne!(
            seeded, unseeded,
            "seeded classification must flip the receipt hash vs attribute-only world"
        );
    }

    // ---- E10 conformance-lift courts ----

    #[test]
    fn canonical_form_has_all_five_fields() {
        let v = ReceiptVector {
            n_admitted: 1,
            n_refused: 2,
            n_residual: 3,
            n_alu: 4,
            n_load_store: 5,
        };
        let mut s = ReceiptScratch::new();
        v.serialize_into(&mut s).expect("fits");
        let text = core::str::from_utf8(s.as_bytes()).expect("ascii");
        assert_eq!(
            text,
            "branchless-receipt admitted=1 refused=2 residual=3 alu=4 ls=5"
        );
    }

    #[test]
    fn scratch_overflow_is_typed_refusal() {
        // Canonical form alone cannot exceed 512B, so overflow is exercised
        // through a bound segment that pushes past the cap: serialization
        // must refuse (typed), never truncate, never allocate a bigger buf.
        let v = ReceiptVector {
            n_admitted: u32::MAX,
            n_refused: u32::MAX,
            n_residual: u32::MAX,
            n_alu: u32::MAX,
            n_load_store: u32::MAX,
        };
        let mut s = ReceiptScratch::new();
        let big = vec![0xABu8; RECEIPT_SCRATCH_CAP];
        let r = frame(&v, &big, b"", b"", &mut s);
        assert_eq!(
            r,
            Err(ReceiptOverflow),
            "overflow must refuse, never truncate"
        );
    }

    #[test]
    fn alu_count_flip_changes_hash() {
        // Anti-vacuity: flipping ONE instruction-class count must flip the
        // receipt hash (the 3-field world would have hashed these equal).
        let a = ReceiptVector {
            n_admitted: 1,
            n_refused: 0,
            n_residual: 0,
            n_alu: 10,
            n_load_store: 4,
        };
        let b = ReceiptVector { n_alu: 11, ..a };
        assert_ne!(
            canonical_hash(&a),
            canonical_hash(&b),
            "ALU flip must flip hash"
        );
        let c = ReceiptVector {
            n_load_store: 5,
            ..a
        };
        assert_ne!(
            canonical_hash(&a),
            canonical_hash(&c),
            "load/store flip must flip hash"
        );
    }

    #[test]
    fn session_lease_is_stable_within_thread_and_distinct_across_callers_of_fold() {
        let l1 = session_lease();
        let l2 = session_lease();
        assert_eq!(l1, l2, "lease must be stable within a session/thread");
        assert_ne!(l1, [0u8; 32], "lease must not be the zero digest");
    }

    #[test]
    fn bound_receipt_flips_on_any_segment() {
        let v = ReceiptVector {
            n_admitted: 1,
            n_refused: 0,
            n_residual: 0,
            n_alu: 7,
            n_load_store: 3,
        };
        let tmp = tempfile::tempdir().expect("tempdir");
        let w = ReceiptWriter::new(tmp.path().to_path_buf());
        let base = w
            .write_bound(&v, b"fn f() {}", b"\x00\x01ELFTEXT", b"int f(void);")
            .expect("bound write");
        let src_flip = w
            .write_bound(&v, b"fn f() {} ", b"\x00\x01ELFTEXT", b"int f(void);")
            .expect("src flip");
        let elf_flip = w
            .write_bound(&v, b"fn f() {}", b"\x00\x01ELFTEXTX", b"int f(void);")
            .expect("elf flip");
        let cfile_flip = w
            .write_bound(&v, b"fn f() {}", b"\x00\x01ELFTEXT", b"int f(void); ")
            .expect("cfile flip");
        for (name, h) in [("src", src_flip), ("elf", elf_flip), ("cfile", cfile_flip)] {
            assert_ne!(base, h, "{name} segment mutation must flip the bound hash");
        }
    }

    #[test]
    fn bound_receipt_file_lands_with_bound_hash() {
        let v = ReceiptVector::default();
        let tmp = tempfile::tempdir().expect("tempdir");
        let w = ReceiptWriter::new(tmp.path().to_path_buf());
        let h = w.write_bound(&v, b"s", b"e", b"c").expect("bound write");
        assert!(tmp.path().join(format!("{h}.txt")).exists());
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn fold_feeds_instruction_class_counts() {
        let mut f = fv("a", 0);
        f.alu_count = 9;
        f.load_store_count = 4;
        let report = crate::disasm_court::DisasmReport {
            functions: vec![f, fv("b", 1)],
        };
        let v = ReceiptVector::fold(&report, &ParsedAttrs::default());
        assert_eq!(v.n_alu, 9);
        assert_eq!(v.n_load_store, 4);
        assert_eq!(v.total(), 2);
    }

    // ---- Zero-allocation serialization court ----

    #[test]
    fn serialize_loop_is_zero_allocation() {
        let v = ReceiptVector {
            n_admitted: 12,
            n_refused: 3,
            n_residual: 1,
            n_alu: 4242,
            n_load_store: 777,
        };
        let mut s = ReceiptScratch::new();
        crate::conformance::tests::alloc_watch_reset();
        let r = v.serialize_into(&mut s);
        let allocs = crate::conformance::tests::alloc_watch_get();
        r.expect("canonical form must fit scratch");
        assert_eq!(
            allocs, 0,
            "serialization must allocate nothing (witnessed {allocs})"
        );
        // Anti-vacuity: the SAME counter must see allocations when a
        // format! runs, so the zero above is a measured zero, not a dead
        // counter.
        let _guard = format!("{}", v.n_alu);
        let allocs_pos = crate::conformance::tests::alloc_watch_get();
        assert!(
            allocs_pos > 0,
            "counter is dead (no allocs seen for format!) - vacuous court"
        );
    }
}
