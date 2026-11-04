//! Disassembly court (agent contract: `disasm_court`).
//!
//! Tier 2 authoritative verdict: parse the staged `.o` with `object`,
//! analyze function bytes with `capstone` (ARM64 / RISC-V 32C opcodes),
//! and gimli `.debug_line` PC→line/column mapping is EXECUTED (lane E9):
//! `LineMap::parse` reads `.debug_line`/`__debug_line` unit-by-unit and
//! `judge_classified` attaches line + column to every branch site; probes
//! compile with `-g` via `CompilerWorker::compile_debuginfo`.
//!
//! Anti-vacuity: `judge` on a branchy function MUST report branch_count > 0;
//! the same court on branchfree code MUST report 0. See tests.
use object::{Object, ObjectSection, ObjectSymbol};
use std::path::Path;

/// Per-function Tier 2 verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnVerdict {
    /// Function symbol name.
    pub name: String,
    /// Branch-class instruction count (capstone mnemonic classification).
    pub branch_count: u32,
    /// ALU-class instruction count.
    pub alu_count: u32,
    /// Load/store-class instruction count.
    pub load_store_count: u32,
    /// Instructions in no named class (OTHER).
    pub other_count: u32,
    /// True when branch_count == 0.
    pub is_branchless: bool,
}

/// Four-way instruction class (lane E10 conformance vector).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsnClass {
    Branch,
    Alu,
    LoadStore,
    Other,
}

/// Result of judging one object file.
#[derive(Debug, Clone, Default)]
pub struct DisasmReport {
    /// One verdict per function symbol.
    pub functions: Vec<FnVerdict>,
}

impl DisasmReport {
    /// True when every judged function is branchless.
    pub fn all_branchless(&self) -> bool {
        self.functions.iter().all(|f| f.is_branchless)
    }

    /// Total ALU instructions across all judged functions.
    pub fn total_alu(&self) -> u32 {
        self.functions.iter().map(|f| f.alu_count).sum()
    }

    /// Total load/store instructions across all judged functions.
    pub fn total_load_store(&self) -> u32 {
        self.functions.iter().map(|f| f.load_store_count).sum()
    }

    /// Total branch instructions across all judged functions.
    pub fn total_branches(&self) -> u32 {
        self.functions.iter().map(|f| f.branch_count).sum()
    }
}

/// Instruction-set selection for the court.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourtArch {
    /// ARM64 (aarch64).
    Arm64,
    /// RISC-V 32-bit with compressed extension.
    Riscv32C,
}

/// The Tier 2 court.
pub struct DisasmCourt {
    arch: CourtArch,
}

/// Classify one capstone mnemonic as control-flow.
fn is_branch_mnemonic(arch: CourtArch, m: &str) -> bool {
    classify_insn(arch, m) == InsnClass::Branch
}

/// Four-way ISA classification (lane E10). Branch takes priority (a
/// load of a jump-table target is still control flow), then load/store,
/// then ALU; everything else is OTHER.
pub fn classify_insn(arch: CourtArch, mnemonic: &str) -> InsnClass {
    let m = mnemonic.to_ascii_lowercase();
    match arch {
        CourtArch::Arm64 => {
            // BRANCH (capstone groups fold onto these mnemonics: JUMP/CALL)
            if m == "b"
                || m.starts_with("b.")
                || m == "bl"
                || m == "br"
                || m.starts_with("cb")
                || m.starts_with("tb")
            {
                return InsnClass::Branch;
            }
            // LOAD/STORE
            if m == "ldr"
                || m == "ldp"
                || m == "str"
                || m == "stp"
                || m == "ldrb"
                || m == "ldrh"
                || m == "strb"
                || m == "strh"
                || m == "ldrsb"
                || m == "ldrsw"
                || m == "ldur"
                || m == "stur"
                || m == "ldxr"
                || m == "stxr"
                || m == "prfm"
            {
                return InsnClass::LoadStore;
            }
            // ALU
            if m == "add"
                || m == "adds"
                || m == "sub"
                || m == "subs"
                || m == "mul"
                || m == "madd"
                || m == "msub"
                || m == "and"
                || m == "ands"
                || m == "orr"
                || m == "eor"
                || m == "bic"
                || m == "mvn"
                || m == "lsl"
                || m == "lsr"
                || m == "asr"
                || m == "ror"
                || m == "cmp"
                || m == "cmn"
                || m == "tst"
                || m == "mov"
                || m == "movz"
                || m == "movk"
                || m == "movn"
                || m == "sxtw"
                || m == "uxtw"
                || m == "cset"
                || m == "csinc"
                || m == "csel"
                || m == "cinc"
                || m == "adrp"
                || m == "adr"
            {
                return InsnClass::Alu;
            }
            InsnClass::Other
        }
        CourtArch::Riscv32C => {
            // `c.jr ra` is the canonical return encoding (like arm64 `ret`) —
            // a return is not a branch, or every tail-calls-free fn flags.
            if m != "c.jr"
                && !m.is_empty()
                && (m.starts_with("b")
                    || m.starts_with("c.b")
                    || m.starts_with("c.j")
                    || m == "jal"
                    || m == "jalr"
                    || m == "j")
            {
                return InsnClass::Branch;
            }
            if m == "lw"
                || m == "sw"
                || m == "lb"
                || m == "lbu"
                || m == "lh"
                || m == "lhu"
                || m == "sb"
                || m == "c.lw"
                || m == "c.sw"
                || m == "c.lwsp"
                || m == "c.swsp"
            {
                return InsnClass::LoadStore;
            }
            if m == "add"
                || m == "addi"
                || m == "sub"
                || m == "mul"
                || m == "and"
                || m == "andi"
                || m == "or"
                || m == "ori"
                || m == "xor"
                || m == "xor"
                || m == "sll"
                || m == "slli"
                || m == "srl"
                || m == "srli"
                || m == "sra"
                || m == "srai"
                || m == "slt"
                || m == "slti"
                || m == "sltu"
                || m == "sltiu"
                || m == "lui"
                || m == "auipc"
                || m == "c.add"
                || m == "c.addi"
                || m == "c.li"
                || m == "c.lui"
                || m == "c.mv"
                || m == "c.addi16sp"
                || m == "c.addi4spn"
                || m == "c.slli"
                || m == "c.srai"
                || m == "c.srli"
                || m == "c.and"
                || m == "c.or"
                || m == "c.xor"
                || m == "c.sub"
            {
                return InsnClass::Alu;
            }
            InsnClass::Other
        }
    }
}

/// PC -> (line, column) map resolved from the object's DWARF `.debug_line`
/// section via gimli. This EXECUTES the mapping BL1 plumbed but never ran:
/// line numbers are attached to real disassembled instruction addresses.
///
/// Floors: multi-CU objects are walked unit-by-unit (self-delimiting
/// `unit_length`); file names are not resolved (line/column only — the
/// court pins diagnostics to the probe source already in hand). Columns are
/// surfaced only when rustc emitted them (`ColumnType::Column`); otherwise
/// `has_column_data()` is false and the court states the line-level floor.
pub struct LineMap {
    rows: Vec<Row>,
    column_data_present: bool,
}

#[derive(Debug, Clone)]
struct Row {
    addr: u64,
    line: u32,
    column: Option<u32>,
}

impl LineMap {
    /// Parse every line program in the object's `.debug_line` /
    /// `__debug_line` section. Returns None (loud, never a silent pass) when
    /// the object carries no debug line info.
    pub fn parse(bytes: &[u8]) -> Option<LineMap> {
        let obj = object::File::parse(bytes).ok()?;
        let data = [".debug_line", "__debug_line"]
            .iter()
            .find_map(|n| obj.section_by_name_bytes(n.as_bytes()))
            .and_then(|s| s.data().ok().map(|d| d.to_vec()))?;
        let dline = gimli::DebugLine::new(&data, gimli::LittleEndian);
        let mut rows = Vec::new();
        let mut column_data_present = false;
        let mut off = 0usize;
        while off + 4 <= data.len() {
            // 32-bit vs 64-bit DWARF initial length.
            let u32len = u32::from_le_bytes(data[off..off + 4].try_into().ok()?) as usize;
            let (unit_len, hdr_len) = if u32len == 0xffff_ffff {
                if off + 12 > data.len() {
                    break;
                }
                let l = u64::from_le_bytes(data[off + 4..off + 12].try_into().ok()?) as usize;
                (l, 12)
            } else {
                (u32len, 4)
            };
            let start = gimli::DebugLineOffset(off);
            let program = dline.program(start, 4, None, None).ok()?.rows();
            let mut rows_it = program;
            let mut consumed_rows = false;
            while let Ok(Some((header, row))) = rows_it.next_row() {
                if row.end_sequence() {
                    continue;
                }
                let line = match row.line() {
                    Some(l) => l.get() as u32,
                    None => continue,
                };
                let column = match row.column() {
                    gimli::ColumnType::Column(c) => {
                        column_data_present = true;
                        Some(c.get() as u32)
                    }
                    gimli::ColumnType::LeftEdge => None,
                };
                rows.push(Row {
                    addr: row.address(),
                    line,
                    column,
                });
                consumed_rows = true;
                let _ = header;
            }
            if !consumed_rows {
                return None; // header parsed but zero rows: corrupt debug_line
            }
            off += hdr_len + unit_len;
        }
        rows.sort_by_key(|r| r.addr);
        Some(LineMap {
            rows,
            column_data_present,
        })
    }

    /// (line, column) for the highest row at or below `pc`.
    pub fn line_col_for_pc(&self, pc: u64) -> Option<(u32, Option<u32>)> {
        let idx = match self
            .rows
            .binary_search_by(|r| r.addr.cmp(&pc).then(std::cmp::Ordering::Equal))
        {
            Ok(i) => i,
            Err(0) => return None,
            Err(i) => i - 1,
        };
        self.rows.get(idx).map(|r| (r.line, r.column))
    }

    /// True when rustc emitted column data for at least one row.
    pub fn has_column_data(&self) -> bool {
        self.column_data_present
    }

    /// Number of row entries (court-facing, guards vacuous parses).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

impl DisasmCourt {
    /// Court for ARM64 objects.
    pub fn arm64() -> Self {
        Self {
            arch: CourtArch::Arm64,
        }
    }

    /// Court for RISC-V 32C objects.
    pub fn riscv32c() -> Self {
        Self {
            arch: CourtArch::Riscv32C,
        }
    }

    /// Court matching a rustc target.
    pub fn for_target(t: &crate::compiler_worker::TargetKind) -> Self {
        match t {
            crate::compiler_worker::TargetKind::Aarch64AppleDarwin => Self::arm64(),
            crate::compiler_worker::TargetKind::Riscv32Imc => Self::riscv32c(),
        }
    }

    /// Judge one object file: enumerate defined function symbols, slice
    /// `.text`/`__text` bytes, disassemble each function, count branches.
    pub fn judge(&self, obj_path: &Path) -> DisasmReport {
        let mut report = DisasmReport::default();
        let bytes = match std::fs::read(obj_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("COURT-SKIP unreadable object {obj_path:?}: {e}");
                return report;
            }
        };
        let obj = match object::File::parse(&*bytes) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("COURT-SKIP unparseable object: {e}");
                return report;
            }
        };
        // Collect ALL code-bearing sections: Mach-O folds functions into
        // `__text`, but ELF targets (riscv32imc) emit per-function sections
        // (`.text.<fn>`) — the arm64-only `.text` lookup found none of them.
        let mut text_sections: Vec<(object::SectionIndex, u64, &[u8])> = Vec::new();
        for sec in obj.sections() {
            let Ok(name) = sec.name() else { continue };
            if name == "__text" || name == ".text" || name.starts_with(".text.") {
                if let Ok(d) = sec.data() {
                    text_sections.push((sec.index(), sec.address(), d));
                }
            }
        }
        if text_sections.is_empty() {
            eprintln!("COURT-SKIP no text section in {obj_path:?}");
            return report;
        }
        // Mach-O symbol sizes are unreliable (nm reports 0) — derive each
        // function's extent from the gap to the next text symbol, per section.
        for (sec_idx, sec_addr, sec_data) in &text_sections {
            let mut fns: Vec<(u64, String)> = Vec::new();
            for sym in obj.symbols() {
                if !sym.is_definition() {
                    continue;
                }
                if sym.section_index() != Some(*sec_idx) {
                    continue;
                }
                if let Ok(name) = sym.name() {
                    // Skip assembler scaffolding (ltmp*/ldata*/$x/$d mapping syms).
                    if name.starts_with("ltmp")
                        || name.starts_with("ldata")
                        || name.starts_with('$')
                    {
                        continue;
                    }
                    fns.push((sym.address(), name.to_string()));
                }
            }
            fns.sort();
            fns.dedup();
            for (i, (addr, name)) in fns.iter().enumerate() {
                let start = (*addr - sec_addr) as usize;
                let end = if i + 1 < fns.len() {
                    ((fns[i + 1].0 - sec_addr) as usize).min(sec_data.len())
                } else {
                    sec_data.len()
                };
                let Some(data) = sec_data.get(start..end.max(start)) else {
                    continue;
                };
                let (branch_count, alu_count, load_store_count, other_count) =
                    self.count_classes(data, *addr);
                report.functions.push(FnVerdict {
                    name: name.clone(),
                    branch_count,
                    alu_count,
                    load_store_count,
                    other_count,
                    is_branchless: branch_count == 0,
                });
            }
        }
        report
    }

    /// Disassemble one code range; returns branch instruction count.
    pub fn count_branches(&self, code: &[u8], addr: u64) -> u32 {
        self.count_classes(code, addr).0
    }

    /// Disassemble one code range; returns per-class instruction counts
    /// (branch, alu, load/store, other) — the E10 conformance feed.
    pub fn count_classes(&self, code: &[u8], addr: u64) -> (u32, u32, u32, u32) {
        let Some(cs) = self.capstone_skipdata() else {
            // Loud typed refusal: the court never returns a silent pass.
            eprintln!("COURT-SKIP capstone init failed");
            return (0, 0, 0, 0);
        };
        let mut counts = (0u32, 0u32, 0u32, 0u32);
        if let Ok(insns) = cs.disasm_all(code, addr) {
            for ins in insns.iter() {
                let class = classify_insn(self.arch, ins.mnemonic().unwrap_or(""));
                match class {
                    InsnClass::Branch => counts.0 += 1,
                    InsnClass::Alu => counts.1 += 1,
                    InsnClass::LoadStore => counts.2 += 1,
                    InsnClass::Other => counts.3 += 1,
                }
            }
        }
        counts
    }

    fn capstone(&self) -> Result<capstone::Capstone, capstone::Error> {
        use capstone::prelude::*;
        match self.arch {
            CourtArch::Arm64 => capstone::Capstone::new()
                .arm64()
                .mode(capstone::arch::arm64::ArchMode::Arm)
                .build(),
            // RiscV32 + RiscVC: without CS_MODE_RISCVC the compressed (2-byte)
            // encodings that riscv32imc actually emits mis-decode or vanish.
            CourtArch::Riscv32C => capstone::Capstone::new()
                .riscv()
                .mode(capstone::arch::riscv::ArchMode::RiscV32)
                .extra_mode([capstone::arch::riscv::ArchExtraMode::RiscVC].into_iter())
                .build(),
        }
    }

    /// Capstone handle with skipdata enabled: relocatable ELF objects carry
    /// relocation placeholder bytes mid-stream; without skipdata a single
    /// undecodable halfword aborts `disasm_all` and the court drops the whole
    /// section (witnessed: `.text.rf_branchy` mnemonics vanished wholesale).
    fn capstone_skipdata(&self) -> Option<capstone::Capstone> {
        let mut cs = self.capstone().ok()?;
        if cs.set_skipdata(true).is_err() {
            return None;
        }
        Some(cs)
    }

    /// All mnemonics in the object's text section, in order (used by the
    /// RVC courts to assert compressed encodings are actually seen).
    pub fn text_mnemonics(&self, obj_path: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let bytes = match std::fs::read(obj_path) {
            Ok(b) => b,
            Err(_) => return out,
        };
        let obj = match object::File::parse(&*bytes) {
            Ok(o) => o,
            Err(_) => return out,
        };
        let Some(cs) = self.capstone_skipdata() else {
            return out;
        };
        for sec in obj.sections() {
            let Ok(name) = sec.name() else { continue };
            if !(name == "__text" || name == ".text" || name.starts_with(".text.")) {
                continue;
            }
            let Ok(d) = sec.data() else { continue };
            if let Ok(insns) = cs.disasm_all(d, sec.address()) {
                for ins in insns.iter() {
                    out.push(ins.mnemonic().unwrap_or("").to_string());
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonic_classification_flips() {
        // Anti-vacuity: each class boundary flips the predicate.
        for m in ["b", "b.eq", "bl", "br", "cbz", "tbnz"] {
            assert!(
                is_branch_mnemonic(CourtArch::Arm64, m),
                "{m} should be branch"
            );
        }
        for m in ["add", "mov", "bic", "ret", "stp"] {
            assert!(
                !is_branch_mnemonic(CourtArch::Arm64, m),
                "{m} must not be branch"
            );
        }
        for m in ["beq", "jal", "jalr", "c.j", "c.beqz", "c.bnez"] {
            assert!(
                is_branch_mnemonic(CourtArch::Riscv32C, m),
                "{m} should be branch"
            );
        }
        // c.jr ra is the return encoding — never a branch.
        for m in ["addi", "c.addi", "c.jr"] {
            assert!(
                !is_branch_mnemonic(CourtArch::Riscv32C, m),
                "{m} must not be branch"
            );
        }
    }

    #[test]
    fn four_way_classification_flips_every_boundary() {
        // Anti-vacuity: each class boundary flips classify_insn.
        let arm = CourtArch::Arm64;
        assert_eq!(classify_insn(arm, "add"), InsnClass::Alu);
        assert_eq!(classify_insn(arm, "stp"), InsnClass::LoadStore);
        assert_eq!(classify_insn(arm, "ldr"), InsnClass::LoadStore);
        assert_eq!(classify_insn(arm, "bl"), InsnClass::Branch);
        assert_eq!(classify_insn(arm, "msr"), InsnClass::Other);
        assert_eq!(classify_insn(arm, "nop"), InsnClass::Other);
        let rvc = CourtArch::Riscv32C;
        assert_eq!(classify_insn(rvc, "addi"), InsnClass::Alu);
        assert_eq!(classify_insn(rvc, "c.lwsp"), InsnClass::LoadStore);
        assert_eq!(classify_insn(rvc, "sw"), InsnClass::LoadStore);
        assert_eq!(classify_insn(rvc, "jal"), InsnClass::Branch);
        assert_eq!(classify_insn(rvc, "fence"), InsnClass::Other);
        assert_eq!(classify_insn(rvc, "ecall"), InsnClass::Other);
    }

    #[test]
    fn known_class_mix_object_court() {
        // Crafted object with a known class mix: a load/store-heavy fn and a
        // branchy fn. The court must see BOTH classes non-zero, and the
        // branchy fn must have more branches than the branchfree one.
        let w = crate::compiler_worker::CompilerWorker::new();
        let src = r#"
#[no_mangle]
pub extern "C" fn ls_heavy(x: u32) -> u32 {
    let a = core::hint::black_box(&x).wrapping_add(1);
    let b = core::hint::black_box(&x).wrapping_sub(2);
    let c = core::hint::black_box(&x).wrapping_mul(3);
    a.wrapping_add(b).wrapping_add(c)
}
#[no_mangle]
pub extern "C" fn br_fn(x: u32) -> u32 {
    let mut i = core::hint::black_box(0u32);
    while i < core::hint::black_box(x) { i = i.wrapping_add(1); }
    i
}"#;
        let obj = match w.compile(src, crate::compiler_worker::TargetKind::Aarch64AppleDarwin) {
            crate::compiler_worker::CompileOutcome::Object(p) => p,
            other => panic!("expected object, got {other:?}"),
        };
        let report = DisasmCourt::arm64().judge(&obj);
        let ls = report
            .functions
            .iter()
            .find(|f| f.name.contains("ls_heavy"))
            .expect("probe ls_heavy");
        let br = report
            .functions
            .iter()
            .find(|f| f.name.contains("br_fn"))
            .expect("probe br_fn");
        assert!(
            ls.load_store_count > 0,
            "ls_heavy must show load/store insns"
        );
        assert!(ls.alu_count > 0, "ls_heavy must show ALU insns");
        assert_eq!(ls.branch_count, 0, "ls_heavy is branchfree");
        assert!(br.branch_count > 0, "br_fn must flip the branch counter");
        assert!(
            report.total_load_store() > 0 && report.total_alu() > 0 && report.total_branches() > 0,
            "report totals must aggregate all three classes"
        );
    }

    #[test]
    fn branchfree_host_object_is_admitted() {
        let w = crate::compiler_worker::CompilerWorker::new();
        // #[no_mangle] keeps the symbol alive in --emit=obj output.
        let src = "#[no_mangle]\npub extern \"C\" fn f(x: u32) -> u32 { x.wrapping_add(1) }";
        let obj = match w.compile(src, crate::compiler_worker::TargetKind::Aarch64AppleDarwin) {
            crate::compiler_worker::CompileOutcome::Object(p) => p,
            other => panic!("expected object, got {other:?}"),
        };
        let court = DisasmCourt::arm64();
        let report = court.judge(&obj);
        assert!(
            !report.functions.is_empty(),
            "court saw no functions (vacuous)"
        );
        let f = report
            .functions
            .iter()
            .find(|f| f.name.contains("f"))
            .expect("probe fn f");
        assert_eq!(f.branch_count, 0, "branchfree source must yield 0 branches");
    }

    #[test]
    fn branchy_host_object_is_flagged() {
        let w = crate::compiler_worker::CompilerWorker::new();
        // while-loop forces a real conditional branch at -O (an if/else may
        // lower to a branchless cmov — the court sees the real machine code).
        let src = "#[no_mangle]\npub extern \"C\" fn g(x: u32) -> u32 { let mut i = core::hint::black_box(0u32); while i < core::hint::black_box(x) { i = i.wrapping_add(1); } i }";
        let obj = match w.compile(src, crate::compiler_worker::TargetKind::Aarch64AppleDarwin) {
            crate::compiler_worker::CompileOutcome::Object(p) => p,
            other => panic!("expected object, got {other:?}"),
        };
        let report = DisasmCourt::arm64().judge(&obj);
        let g = report
            .functions
            .iter()
            .find(|f| f.name.contains("g"))
            .expect("probe fn g");
        assert!(g.branch_count > 0, "branchy source must flip the court");
    }
}

/// Fine-grained branch classification (lane E9).
///
/// Classification rule (stated, mechanical):
/// - A conditional branch (b.cond / cbz / cbnz / tbz / tbnz, `b*`/`c.b*`
///   family) whose decoded target is STRICTLY BELOW its own address, with
///   the target inside the same function span, is `LoopStructural`: a
///   backward conditional in-function edge is the necessary and sufficient
///   shape of a counted or while loop. These are admitted, never
///   ANDON-refused.
/// - Forward conditional edges are `Conditional` (ANDON-refused).
/// - `bl`/`jal` are `Call`; `b`/`br`/`j`/`c.j`/`jalr`/`ret-class` are
///   `Unconditional` (not ANDON-refused).
/// Prologue/epilogue idioms (sp-adjust, saves, pc-relative literal loads)
/// contain no conditional in-function backward edge, so the rule yields no
/// false positives there by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchClass {
    /// Backward conditional in-function edge: the loop-back branch of a
    /// counted/while loop. Admitted, never ANDON-refused.
    LoopStructural,
    /// Forward conditional edge: real data-dependent divergence.
    Conditional,
    /// Direct call (arm64 `bl`, RISC-V `jal`).
    Call,
    /// Unconditional pc-relative jump (b/br/j/c.j/jalr).
    Unconditional,
}

/// One classified branch site, with DWARF line/column when available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSite {
    /// Instruction address (vaddr in the object).
    pub addr: u64,
    /// Encoded instruction size in bytes (2 compressed / 4 full on riscv).
    pub size: u32,
    /// Resolved target address when parseable.
    pub target: Option<u64>,
    /// Capstone mnemonic.
    pub mnemonic: String,
    /// Mechanical class per the rule above.
    pub class: BranchClass,
    /// DWARF source line (None when the object carries no line info or the
    /// pc has no row).
    pub line: Option<u32>,
    /// DWARF column when rustc emitted one.
    pub column: Option<u32>,
}

/// Per-function classification verdict.
#[derive(Debug, Clone)]
pub struct FnClassification {
    /// Function symbol name.
    pub name: String,
    /// Every classified branch site in the function span.
    pub sites: Vec<BranchSite>,
    /// True when any site is a forward Conditional edge (ANDON-refused).
    pub andon_refused: bool,
}

/// Parse the branch target token out of capstone op_str and resolve it to
/// an address. Empirical target semantics (witnessed on compiled probes
/// this session):
/// - arm64: ABSOLUTE `#`-prefixed targets (`b.lo #0x28`).
/// - riscv32c: RELATIVE offsets (`bgeu a2, a1, 0x14` at pc 14 resolves to
///   34; `bltu a2, a3, -0xa` at pc 30 resolves to 20).
/// Unparseable op_str -> None -> classifier conservatively refuses
/// (Conditional = ANDON-refused); fail-closed over fail-open.
fn parse_target_addr(op_str: &str, pc: u64, arch: CourtArch) -> Option<u64> {
    let tok = op_str.split(',').next_back()?.trim();
    let tok = tok.trim_start_matches('#');
    if let Some(rest) = tok.strip_prefix('-') {
        let h = rest.strip_prefix("0x").unwrap_or(rest);
        let v = u64::from_str_radix(h, 16).ok()?;
        return pc.checked_sub(v);
    }
    let h = tok.strip_prefix("0x").unwrap_or(tok);
    let v = u64::from_str_radix(h, 16).ok()?;
    Some(match arch {
        CourtArch::Arm64 => v,
        CourtArch::Riscv32C => pc.wrapping_add(v),
    })
}

impl DisasmCourt {
    /// Classify every branch in one code range. Dual-mode by construction:
    /// the RISC-V handle is built with RISCVC extra mode, so 16-bit
    /// compressed (c.beqz/c.bnez/c.j) decode alongside full 32-bit
    /// (beq/bne/bgeu/bltu) — both classes detected on the same object.
    pub fn classify_branches(&self, code: &[u8], addr: u64) -> Vec<BranchSite> {
        let Some(cs) = self.capstone_skipdata() else {
            eprintln!("COURT-SKIP capstone init failed");
            return Vec::new();
        };
        // Pass 1: collect (addr, size, mnemonic, op_str) for branch-class insns.
        let mut raw: Vec<(u64, usize, String, String)> = Vec::new();
        if let Ok(insns) = cs.disasm_all(code, addr) {
            for ins in insns.iter() {
                let mnemonic = ins.mnemonic().unwrap_or("");
                if !is_branch_mnemonic(self.arch, mnemonic) {
                    continue;
                }
                raw.push((
                    ins.address(),
                    ins.bytes().len(),
                    mnemonic.to_string(),
                    ins.op_str().unwrap_or("").to_string(),
                ));
            }
        }
        // Pass 2: resolve targets; classify.
        let mut sites: Vec<BranchSite> = raw
            .iter()
            .map(|(addr, size, mnem, op)| {
                let target = parse_target_addr(op, *addr, self.arch);
                let class = match mnem.as_str() {
                    "bl" | "jal" => BranchClass::Call,
                    "b" | "br" | "j" | "c.j" | "ret" => BranchClass::Unconditional,
                    _ => match target {
                        Some(t) if t < *addr => BranchClass::LoopStructural,
                        _ => BranchClass::Conditional,
                    },
                };
                BranchSite {
                    addr: *addr,
                    size: *size as u32,
                    target,
                    mnemonic: mnem.clone(),
                    class,
                    line: None,
                    column: None,
                }
            })
            .collect();
        // Pass 3 (loop-exit pairing): a forward conditional edge whose
        // target lands immediately past a back-edge is the exit of the same
        // loop — structurally necessary, LoopStructural, never
        // ANDON-refused.
        let back_edges: Vec<(u64, u32)> = sites
            .iter()
            .filter(|s| s.class == BranchClass::LoopStructural)
            .map(|s| (s.addr, s.size))
            .collect();
        let back_end = |t: u64| back_edges.iter().any(|(a, sz)| t == a + *sz as u64);
        for s in sites.iter_mut() {
            if s.class == BranchClass::Conditional {
                if let Some(t) = s.target {
                    if back_end(t) {
                        s.class = BranchClass::LoopStructural
                    }
                }
            }
        }
        sites
    }
}

impl DisasmCourt {
    /// Judge with fine-grained classification: per-function branch sites
    /// resolved to DWARF source lines (and columns when emitted). This is
    /// the E9 precision lift: symbol-level verdicts become line-level ones.
    /// The object must carry .debug_line (compiled with -g); a missing map
    /// panics with an explicit floor message rather than degrading silently.
    pub fn judge_classified(&self, obj_path: &Path) -> (LineMap, Vec<FnClassification>) {
        let bytes = std::fs::read(obj_path).expect("read probe object");
        let line_map = LineMap::parse(&bytes).expect(
            "probe object must carry .debug_line (compiled with -g);              floor: without it the court is symbol-level only",
        );
        let obj = object::File::parse(&*bytes).expect("parse probe object");
        let mut out = Vec::new();
        for sec in obj.sections() {
            let Ok(name) = sec.name() else { continue };
            if !(name == "__text" || name == ".text" || name.starts_with(".text.")) {
                continue;
            }
            let Ok(sec_data) = sec.data() else { continue };
            let sec_addr = sec.address();
            let mut fns: Vec<(u64, String)> = Vec::new();
            for sym in obj.symbols() {
                if !sym.is_definition() || sym.section_index() != Some(sec.index()) {
                    continue;
                }
                if let Ok(n) = sym.name() {
                    if n.starts_with("ltmp") || n.starts_with("ldata") || n.starts_with('$') {
                        continue;
                    }
                    fns.push((sym.address(), n.to_string()));
                }
            }
            fns.sort();
            fns.dedup();
            for (i, (addr, name)) in fns.iter().enumerate() {
                let start = (*addr - sec_addr) as usize;
                let end = if i + 1 < fns.len() {
                    ((fns[i + 1].0 - sec_addr) as usize).min(sec_data.len())
                } else {
                    sec_data.len()
                };
                let Some(data) = sec_data.get(start..end.max(start)) else {
                    continue;
                };
                let mut sites = self.classify_branches(data, *addr);
                for s in sites.iter_mut() {
                    if let Some((l, c)) = line_map.line_col_for_pc(s.addr) {
                        s.line = Some(l);
                        s.column = c;
                    }
                }
                let andon = sites.iter().any(|s| s.class == BranchClass::Conditional);
                out.push(FnClassification {
                    name: name.clone(),
                    sites,
                    andon_refused: andon,
                });
            }
        }
        (line_map, out)
    }
}
