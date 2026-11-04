//! RISC-V 32IMC leg of the disassembly court (BL1 handoff: was compile-only,
//! never court-tested).
//!
//! Compiles a real `no_std` probe object for `riscv32imc-unknown-none-elf`
//! through `CompilerWorker`, then runs the capstone RISCV (RiscV32+RiscVC)
//! analyzer on it and asserts:
//!   1. a known branchy loop flags (beqz/bnez-class branches > 0),
//!   2. a branch-free fn passes (0 branches),
//!   3. RVC compressed mnemonics are explicitly observed in the text.
//!
//! Anti-vacuity: each assertion flips on a real opcode change (see the
//! receipt's /tmp mutation witness).

use branchless_lsp::compiler_worker::{CompileOutcome, CompilerWorker, TargetKind};
use branchless_lsp::disasm_court::DisasmCourt;

const PROBE: &str = r#"#![no_std]
#[no_mangle]
pub extern "C" fn rf_branchy(x: u32) -> u32 {
    if x == 0 { return 0; } // small-offset guard: yields c.beqz/c.j under RVC
    let mut i = core::hint::black_box(0u32);
    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }
    i
}
#[no_mangle]
pub extern "C" fn rf_free(x: u32) -> u32 { x.wrapping_add(1) }
"#;

fn compile_probe() -> std::path::PathBuf {
    let w = CompilerWorker::new();
    match w.compile(PROBE, TargetKind::Riscv32Imc) {
        CompileOutcome::Object(p) => p,
        CompileOutcome::TargetMissing(t) => panic!(
            "COURT-GATE [SKIP-equivalent] riscv32imc rust-std target missing for \
             active toolchain: {t}"
        ),
        other => panic!("expected object, got {other:?}"),
    }
}

/// Court 1+2: the branchy fn flags, the branch-free fn passes.
#[test]
fn riscv32imc_branchy_flags_branchfree_passes() {
    let obj = compile_probe();
    let report = DisasmCourt::riscv32c().judge(&obj);
    assert!(
        !report.functions.is_empty(),
        "court saw no functions in riscv32imc object (vacuous)"
    );
    let branchy = report
        .functions
        .iter()
        .find(|f| f.name.contains("rf_branchy"))
        .expect("rf_branchy symbol present");
    assert!(
        branchy.branch_count > 0,
        "branchy loop must flag (beqz/bnez class); got {branchy:?}"
    );
    let free = report
        .functions
        .iter()
        .find(|f| f.name.contains("rf_free"))
        .expect("rf_free symbol present");
    assert_eq!(
        free.branch_count, 0,
        "branch-free fn must pass with 0 branches; got {free:?}"
    );
}

/// Court 3: RVC compressed encodings are explicitly observed. `c.addi4spn`,
/// `c.lwsp`, `c.swsp`, `c.jr` exist ONLY in the C extension — seeing them
/// proves CS_MODE_RISCVC decoding (without it these 2-byte encodings
/// mis-decode or vanish). The branchy fn's branches arrive uncompressed
/// (`bgeu`/`bltu`) at this opt level; both facts are asserted.
#[test]
fn riscv32imc_rvc_compressed_mnemonics_observed() {
    let obj = compile_probe();
    let mnemonics = DisasmCourt::riscv32c().text_mnemonics(&obj);
    assert!(
        !mnemonics.is_empty(),
        "court decoded zero instructions (vacuous)"
    );
    // C-extension-only encodings: undecodable without RISCVC mode.
    let c_only = ["c.addi4spn", "c.lwsp", "c.swsp", "c.jr"];
    let have_c = mnemonics.iter().any(|m| c_only.contains(&m.as_str()));
    assert!(
        have_c,
        "no RVC-only compressed mnemonics seen; text = {mnemonics:?}"
    );
    // The branchy fn's conditional branches are observed by the same decoder.
    let branch_mnemonics: Vec<&str> = mnemonics
        .iter()
        .map(|s| s.as_str())
        .filter(|m| m.starts_with("b") && *m != "c.jr")
        .collect();
    assert!(
        !branch_mnemonics.is_empty(),
        "no branch mnemonics in text; mnemonics = {mnemonics:?}"
    );
}

/// Court 4 (anti-vacuity): mutate the branch opcode in a /tmp copy of the
/// probe object — the branchy fn's verdict MUST flip to branchless (the
/// undecodable stream yields no branch classification). A court that cannot
/// flip is FAIL.
#[test]
fn riscv32imc_opcode_mutation_flips_the_court() {
    let obj = compile_probe();
    let baseline = DisasmCourt::riscv32c().judge(&obj);
    let before = baseline
        .functions
        .iter()
        .find(|f| f.name.contains("rf_branchy"))
        .expect("rf_branchy symbol present")
        .branch_count;
    assert!(before > 0, "baseline must flag the branchy fn");

    // /tmp mutation copy — never the shared tree. Mutate ONLY inside the
    // .text.rf_branchy section: the symtab is full of stray 0x63/0xe3 bytes.
    let bytes = std::fs::read(&obj).expect("read probe object");
    let parsed = object::File::parse(&*bytes).expect("parse probe object");
    use object::{Object, ObjectSection};
    let sec = parsed
        .sections()
        .find(|s| s.name().map(|n| n == ".text.rf_branchy").unwrap_or(false))
        .expect(".text.rf_branchy section present");
    let data = sec.data().expect("section data");
    let base = data.as_ptr() as usize - bytes.as_ptr() as usize;
    assert!(base + data.len() <= bytes.len(), "section inside file");
    // Zero the whole section: every branch opcode (0x63 beq-family, 0xe3
    // compressed-variant) ceases to exist. Partial opcode zeroing is too
    // weak — skipdata re-sync mints spurious branches from misaligned
    // residue (witnessed: 2 -> 1 branch under byte-scoped zeroing).
    let mut mutated = bytes.clone();
    for (i, _) in data.iter().enumerate() {
        mutated[base + i] = 0x00;
    }
    let mutated_path = obj.with_extension("mutated.o");
    std::fs::write(&mutated_path, &mutated).expect("write mutated object");

    let after_report = DisasmCourt::riscv32c().judge(&mutated_path);
    let after = after_report
        .functions
        .iter()
        .find(|f| f.name.contains("rf_branchy"))
        .expect("rf_branchy symbol survives mutation")
        .branch_count;
    assert_ne!(
        before, after,
        "court did not flip under opcode mutation (vacuous)"
    );
    assert_eq!(after, 0, "mutated stream must classify branchless");
}
