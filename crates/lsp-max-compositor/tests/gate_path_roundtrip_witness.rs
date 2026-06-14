// Cross-process gate round-trip witness.
//
// Proves the path the compositor reads (GateFile::for_workspace) is the path the
// shared authoritative fn (lsp_max::primitives::gate_file_path) computes, and that
// a byte written to that path is read back unchanged. The witness terminates in
// the compositor crate — OUTSIDE the producer of the shared fn (the lsp-max root).

use lsp_max_compositor::GateFile;

#[test]
fn gate_path_cross_author_roundtrip_witness() {
    // (a) Determinism: the shared fn computes the same path twice.
    let p1 = lsp_max::primitives::gate_file_path();
    let p2 = lsp_max::primitives::gate_file_path();
    assert_eq!(p1, p2, "gate_file_path() is not deterministic across calls");

    // (b) Cross-author agreement: the compositor's reader path equals the shared fn.
    let compositor_path = GateFile::for_workspace().path().to_path_buf();
    assert_eq!(
        compositor_path, p1,
        "compositor GateFile path diverges from shared gate_file_path()"
    );

    // (c) write -> read round-trip on the agreed path.
    std::fs::write(&p1, b"1").expect("write b\"1\" to gate path failed");
    let read_back = std::fs::read(&compositor_path).expect("read gate path failed");
    assert_eq!(
        read_back, b"1",
        "byte written via shared path != byte read via compositor path"
    );

    // Restore CLEAR so we do not leave an ANDON byte behind.
    let _ = std::fs::write(&p1, b"0");

    eprintln!("WITNESS shared_fn_path={}", p1.display());
    eprintln!("WITNESS compositor_path={}", compositor_path.display());
    eprintln!("WITNESS paths_match={}", compositor_path == p1);
    eprintln!("WITNESS write_read_ok={}", read_back == b"1");
}
