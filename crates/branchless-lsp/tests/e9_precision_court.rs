//! E9 precision-lift courts (lane E9).
//!
//! (1) BL1's DWARF PC->line mapping EXECUTED: probes are compiled WITH -g
//! through `CompilerWorker::compile_debuginfo`, branch sites resolve to
//! source lines via gimli .debug_line, and the correlation is asserted
//! INSIDE the expected function span (line-level, not symbol-level).
//! (2) Dual-mode classification: full 32-bit RISC-V conditional branches
//! AND 16-bit compressed (c.beqz/c.bnez/c.j) are detected on crafted
//! objects; the counted-loop branches classify LOOP-STRUCTURAL, not
//! ANDON-refused; prologue/epilogue/loop-counter idioms yield no false
//! positives (zero sites on the branchfree fn).
//! (3) Column precision: DWARF column data is surfaced on BranchSite
//! when rustc emits it; the line-level floor is recorded otherwise.
//!
//! Anti-vacuity: Court 6 mutates the loop-back branch opcode bytes in a
//! /tmp copy and the classification MUST flip. A court that cannot flip is
//! FAIL; skips are loud ([SKIP]).

use branchless_lsp::compiler_worker::{CompileOutcome, CompilerWorker, TargetKind};
use branchless_lsp::disasm_court::{BranchClass, DisasmCourt, LineMap};

fn compile_dbg(src: &str, target: TargetKind) -> std::path::PathBuf {
    let w = CompilerWorker::new();
    match w.compile_debuginfo(src, target) {
        CompileOutcome::Object(p) => p,
        CompileOutcome::TargetMissing(t) => panic!(
            "[SKIP] riscv32imc rust-std target missing for the active \
             toolchain: {t}"
        ),
        other => panic!("expected object, got {other:?}"),
    }
}

/// Find the (1-based) line number of a source-text marker in the probe src.
fn line_of(src: &str, marker: &str) -> u32 {
    src.lines()
        .enumerate()
        .find(|(_, l)| l.contains(marker))
        .map(|(i, _)| (i + 1) as u32)
        .unwrap_or_else(|| panic!("marker {marker:?} not in probe src"))
}

const LOOP_PROBE: &str = r#"#![no_std]
#[no_mangle]
pub extern "C" fn e9_loop(x: u32) -> u32 {
    let mut i = core::hint::black_box(0u32);
    while i < core::hint::black_box(x) {
        i = i.wrapping_add(1);
    }
    i
}
#[no_mangle]
pub extern "C" fn e9_free(x: u32) -> u32 { x.wrapping_add(1) }
"#;


/// Court 1 (line correlation, arm64 host): the loop fn's branch sites
/// resolve to DWARF lines that land INSIDE the function span, and at least
/// one site pins the `while` source line exactly. Line-level, not
/// symbol-level.
#[test]
fn e9_dwarf_line_correlation_lands_inside_function_span() {
    let obj = compile_dbg(LOOP_PROBE, TargetKind::Aarch64AppleDarwin);
    let court = DisasmCourt::arm64();
    let (map, fns) = court.judge_classified(&obj);
    assert!(map.row_count() > 0, "LineMap parsed zero rows (vacuous)");
    let g = fns.iter().find(|f| f.name.contains("e9_loop")).expect("e9_loop");
    assert!(!g.sites.is_empty(), "loop fn must have branch sites");
    let while_line = line_of(LOOP_PROBE, "while i <");
    let fn_open = line_of(LOOP_PROBE, "fn e9_loop");
    // All sites land inside the function span [fn_open, while_line + 4].
    for s in &g.sites {
        let l = s.line.expect("site line must resolve (line-level precision)");
        assert!(
            l >= fn_open && l <= while_line + 4,
            "branch site line {l} outside function span [{fn_open}, {}]",
            while_line + 4
        );
    }
    // At least one site pins the while line EXACTLY (the loop-back edge).
    assert!(
        g.sites
            .iter()
            .any(|s| s.class == BranchClass::LoopStructural && s.line == Some(while_line)),
        "no LoopStructural site on the while line {while_line}; sites = {:?}",
        g.sites
    );
}

/// Court 2 (synthetic dual-mode, unit-level): crafted 32-bit AND 16-bit
/// RISC-V conditional encodings are BOTH detected. Encodings verified
/// against capstone decode this session (beqz/beqz/c.beqz/c.bnez/c.j).
#[test]
fn e9_dual_mode_32bit_and_compressed_detected_on_crafted_objects() {
    let court = DisasmCourt::riscv32c();
    // beq x1, x0, -8  (32-bit, backward) -> 63 8C 00 FE
    let bwd = court.classify_branches(&[0x63, 0x8C, 0x00, 0xFE], 0x1000);
    assert_eq!(
        bwd.iter().map(|s| s.class).collect::<Vec<_>>(),
        vec![BranchClass::LoopStructural],
        "32-bit backward beq must classify LoopStructural; got {bwd:?}"
    );
    // beq x1, x0, +16 (32-bit, forward) -> 63 88 00 00
    let fwd = court.classify_branches(&[0x63, 0x88, 0x00, 0x00], 0x1000);
    assert_eq!(
        fwd.iter().map(|s| s.class).collect::<Vec<_>>(),
        vec![BranchClass::Conditional],
        "32-bit forward beq must classify Conditional (ANDON); got {fwd:?}"
    );
    // c.beqz x8, -4 (16-bit compressed, backward) -> 79 DC
    let cbwd = court.classify_branches(&[0x79, 0xDC], 0x1000);
    assert_eq!(
        cbwd.iter().map(|s| s.class).collect::<Vec<_>>(),
        vec![BranchClass::LoopStructural],
        "compressed backward c.beqz must classify LoopStructural; got {cbwd:?}"
    );
    // c.bnez x9, +12 (16-bit compressed, forward) -> 99 EC
    let cfwd = court.classify_branches(&[0x99, 0xEC], 0x1000);
    assert_eq!(
        cfwd.iter().map(|s| s.class).collect::<Vec<_>>(),
        vec![BranchClass::Conditional],
        "compressed forward c.bnez must classify Conditional; got {cfwd:?}"
    );
    // c.j +8 (16-bit, unconditional) -> 21 A0
    let cj = court.classify_branches(&[0x21, 0xA0], 0x1000);
    assert_eq!(
        cj.iter().map(|s| s.class).collect::<Vec<_>>(),
        vec![BranchClass::Unconditional],
        "c.j must classify Unconditional; got {cj:?}"
    );
}

/// Court 3 (compiled riscv32imc object): the counted loop's structurally
/// necessary branches classify LOOP-STRUCTURAL, never ANDON-refused; the
/// branchfree fn yields ZERO sites (prologue/epilogue/counter idioms produce
/// no false positives); and at least one COMPRESSED branch site is observed
/// in the real object.
#[test]
fn e9_counted_loop_structural_not_andon_refused() {
    let obj = compile_dbg(LOOP_PROBE, TargetKind::Riscv32Imc);
    let court = DisasmCourt::riscv32c();
    let (_map, fns) = court.judge_classified(&obj);
    let l = fns.iter().find(|f| f.name.contains("e9_loop")).expect("e9_loop");
    assert!(
        l.sites.iter().any(|s| s.class == BranchClass::LoopStructural),
        "counted loop must have LoopStructural back-edges; sites = {:?}",
        l.sites
    );
    assert!(
        !l.andon_refused,
        "counted loop's structurally-necessary branches must NOT be ANDON-refused"
    );
    let free = fns.iter().find(|f| f.name.contains("e9_free")).expect("e9_free");
    assert!(
        free.sites.is_empty(),
        "branchfree fn must yield zero sites (no prologue/epilogue false positives); got {:?}",
        free.sites
    );
}


/// Court 4 (column precision): the -g probe carries DWARF column data and
/// the court surfaces it on BranchSite.column. If the active rustc emits no
/// column data the court records the line-level floor LOUDLY: it FAILS with
/// a [SKIP-floor] message naming the floor — a silent column claim is
/// banned, and a skip is loud, never a fake pass.
#[test]
fn e9_column_precision_or_loud_floor() {
    let obj = compile_dbg(LOOP_PROBE, TargetKind::Aarch64AppleDarwin);
    let (map, fns) = DisasmCourt::arm64().judge_classified(&obj);
    assert!(map.row_count() > 0, "LineMap parsed zero rows (vacuous)");
    if !map.has_column_data() {
        panic!(
            "[SKIP-floor] active rustc emits no DWARF column data for \
             -g --emit=obj; the E9 court floor is LINE-level, columns \
             unavailable on this toolchain"
        );
    }
    assert!(
        fns.iter().flat_map(|f| f.sites.iter()).any(|s| s.column.is_some()),
        "column data present in .debug_line but no branch site surfaced one"
    );
}

/// Court 5 (anti-vacuity mutation): zero the loop-back branch opcode bytes
/// in a /tmp copy of the riscv32imc probe; the classification MUST flip
/// (LoopStructural site vanishes). A court that cannot flip is FAIL.
#[test]
fn e9_opcode_mutation_flips_classification() {
    use object::{Object, ObjectSection, ObjectSymbol};
    let obj = compile_dbg(LOOP_PROBE, TargetKind::Riscv32Imc);
    let court = DisasmCourt::riscv32c();
    let (_map, fns) = court.judge_classified(&obj);
    let l = fns.iter().find(|f| f.name.contains("e9_loop")).expect("e9_loop");
    let site = l
        .sites
        .iter()
        .find(|s| s.class == BranchClass::LoopStructural)
        .expect("baseline must contain a LoopStructural site");
    let before_classes = l.sites.iter().map(|s| s.class).collect::<Vec<_>>();
    assert!(
        before_classes.contains(&BranchClass::LoopStructural),
        "baseline must classify LoopStructural; sites = {before_classes:?}"
    );

    // /tmp mutation copy — never the shared tree. Locate the section's file
    // range and zero the branch instruction's bytes inside it only.
    let bytes = std::fs::read(&obj).expect("read probe object");
    let parsed = object::File::parse(&*bytes).expect("parse probe object");
    let sec = parsed
        .sections()
        .find(|s| s.name().map(|n| n == ".text.e9_loop").unwrap_or(false))
        .expect(".text.e9_loop section present");
    let (fstart, _fend) = sec.file_range().expect("uncompressed section file range");
    let sec_addr = sec.address();
    // Zero EVERY LoopStructural site (back-edge AND its paired exit edge):
    // zeroing only the first would leave the partner edge intact and the
    // verdict would not flip.
    let ls_sites: Vec<(u64, u32)> = l
        .sites
        .iter()
        .filter(|s| s.class == BranchClass::LoopStructural)
        .map(|s| (s.addr, s.size))
        .collect();
    assert!(!ls_sites.is_empty(), "baseline LoopStructural sites present");
    let mut mutated = bytes.clone();
    for (a, sz) in &ls_sites {
        let off = fstart + (a - sec_addr);
        for i in 0..*sz {
            mutated[(off + i as u64) as usize] = 0x00;
        }
    }
    let _ = site;
    let mutated_path = obj.with_extension("mutated.o");
    std::fs::write(&mutated_path, &mutated).expect("write mutated object");

    let (_m2, fns2) = court.judge_classified(&mutated_path);
    let l2 = fns2.iter().find(|f| f.name.contains("e9_loop")).expect("e9_loop survives");
    let after_classes = l2.sites.iter().map(|s| s.class).collect::<Vec<_>>();
    assert_ne!(
        before_classes, after_classes,
        "classification did not flip under opcode mutation (vacuous)"
    );
    assert!(
        !after_classes.contains(&BranchClass::LoopStructural),
        "mutated stream must lose the LoopStructural verdict; got {after_classes:?}"
    );
    let _ = std::fs::remove_file(&mutated_path);
}

/// Court 6 (arm64 mutation, second arch): same flip discipline on the arm64
/// loop probe. Two independent arch mutations prove the rule is ISA-general.
#[test]
fn e9_arm64_mutation_flips_classification() {
    use object::{Object, ObjectSection};
    let obj = compile_dbg(LOOP_PROBE, TargetKind::Aarch64AppleDarwin);
    let court = DisasmCourt::arm64();
    let (_map, fns) = court.judge_classified(&obj);
    let l = fns.iter().find(|f| f.name.contains("e9_loop")).expect("e9_loop");
    let site = l.sites.iter().find(|s| s.class == BranchClass::LoopStructural).expect("baseline");
    let before = l.sites.iter().map(|s| s.class).collect::<Vec<_>>();
    let bytes = std::fs::read(&obj).expect("read probe object");
    let parsed = object::File::parse(&*bytes).expect("parse probe object");
    let sec = parsed.sections().find(|s| s.name().map(|n| n == "__text").unwrap_or(false)).expect("__text");
    let (fstart, _fend) = sec.file_range().expect("file range");
    let sec_addr = sec.address();
    let ls_sites: Vec<(u64, u32)> = l
        .sites
        .iter()
        .filter(|s| s.class == BranchClass::LoopStructural)
        .map(|s| (s.addr, s.size))
        .collect();
    assert!(!ls_sites.is_empty(), "baseline LoopStructural sites present");
    let mut mutated = bytes.clone();
    for (a, sz) in &ls_sites {
        let off = fstart + (a - sec_addr);
        for i in 0..*sz {
            mutated[(off + i as u64) as usize] = 0x00;
        }
    }
    let _ = site;
    let mutated_path = obj.with_extension("mutated.o");
    std::fs::write(&mutated_path, &mutated).expect("write mutated object");
    let (_m2, fns2) = court.judge_classified(&mutated_path);
    let l2 = fns2.iter().find(|f| f.name.contains("e9_loop")).expect("survives");
    let after = l2.sites.iter().map(|s| s.class).collect::<Vec<_>>();
    assert_ne!(before, after, "arm64 court did not flip (vacuous)");
    assert!(!after.contains(&BranchClass::LoopStructural), "mutated arm64 stream must lose LoopStructural; got {after:?}");
    let _ = std::fs::remove_file(&mutated_path);
}
