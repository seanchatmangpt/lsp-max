# GC005 wasm4pm Process Conformance Handoff

> Status: ADMISSIBLE EVIDENCE AGGREGATED
> Date: 2026-06-06

## Process Context
This record serves as the input artifact for **GALL_CHECKPOINT_005**. It aggregates the evidence generated during the admission of GC001–GC004.

## Admitted Input States
- **GC001 (Boundary):** ADMITTED - Portable boundary declared.
- **GC002 (Source):** ADMITTED_BY_DOGFOOD - Tera template manufacturing.
- **GC003 (Receipts):** ADMITTED_BY_DOGFOOD - Equation-bound witnesses.
- **GC004 (Observer):** DIAGNOSTIC_PATH_ADMITTED - Protocol-level split.

## Evidence Snapshot
### Event Log Summary
The following events are captured in the aggregated receipt stream:
- `file.projected`: GC002 template manufacturing events.
- `diagnostic.published`: GC004 protocol-wire findings.
- `checkpoint.admitted`: Canonical admission records for 001-004.

### Repositories Involved
- **Producers:** `~/ggen` (Branch: `feat/ggen-lsp-source-laws`)
- **Consumer/Tracker:** `~/tower-lsp-max` (Branch: `master`)

## Raw Evidence Stream (Aggregate)
```json
{"blake3":"2fed9352ea54b471118187b6b535d8010c77413874f99e9d81ae2d931dc4edbc","boundary_digest":"384337ff0c58c233a71666f16ac5dc231e73aaa7d5ef995db6de781677a2b8b5","customization_digest":"5872f46c06377002428992a98902eb6b220ac99c95ebb63b1ae2e550ba693741","event":"file.projected","mutation_gate_decision":"admitted","object":"crates/ggen-lsp/tests/dogfood_gc004.rs","pack":"pack","pack_descriptor_digest":"6f3ee974fdd535c17c152f0165b0448d1953bd20020d6a0d6e8bc72e624a313b","pack_plan_digest":"424de8edf5b081002b0f909cc5221e13803ec8c0cfe4d94d46d5743758219d95","previous_receipt":null,"projection_engine_version":"1.0.0","receipt_id":"0f5ccc63-d98e-4cc8-abae-a3386d918879","staging_digest":"e11ce1a6fa7e05aa6e4301e26de8a155b90eaa8303abb514948320aaf65ba9af","template_digest":"76f4e80a412bbd355a9e69f5acfd792ae08006a8e57536ede779723f7be1a3b0","verification_result":"passed","verified_at":"2026-06-06T23:24:54.530013Z","workspace_digest":"58f1dccfacfeb5d1d58cb224e3b00450e22bbd8ecf404b43cf996e8e0ee1ecfb"}
{"blake3":"fa2e96bf756047e1041d984ac5efdfd6549d350eacedadb79669105752dd89b7","boundary_digest":"569e9e9d37c31feec9861d52683cf3f3f04dae2e92caf3409166c2212178644f","customization_digest":"5872f46c06377002428992a98902eb6b220ac99c95ebb63b1ae2e550ba693741","event":"file.projected","mutation_gate_decision":"admitted","object":"crates/ggen-projection/tests/dogfood_gc003.rs","pack":"pack","pack_descriptor_digest":"9b6a3afb2aeba92b5e2cc66842f63061d73a549e961de0604a111f12874e35aa","pack_plan_digest":"424de8edf5b081002b0f909cc5221e13803ec8c0cfe4d94d46d5743758219d95","previous_receipt":"5ea0884d-1708-402a-aec7-644f7d114dbb","projection_engine_version":"1.0.0","receipt_id":"e5d25256-d661-4acd-9579-440245012013","staging_digest":"e11ce1a6fa7e05aa6e4301e26de8a155b90eaa8303abb514948320aaf65ba9af","template_digest":"b639c43c9fb066bc3dd00a96a7aeb13bb7c98b364ce63c78ed10ae93ce6fc8a0","verification_result":"passed","verified_at":"2026-06-06T22:54:08.420147Z","workspace_digest":"58f1dccfacfeb5d1d58cb224e3b00450e22bbd8ecf404b43cf996e8e0ee1ecfb"}
{"blake3":"338bd906784f800617a44aa50099b591bc9bd2ec05ed114da00d98d8494a7fe2","boundary_digest":"569e9e9d37c31feec9861d52683cf3f3f04dae2e92caf3409166c2212178644f","customization_digest":"5872f46c06377002428992a98902eb6b220ac99c95ebb63b1ae2e550ba693741","event":"file.projected","mutation_gate_decision":"admitted","object":"crates/ggen-projection/tests/dogfood_gc002.rs","pack":"pack","pack_descriptor_digest":"9b6a3afb2aeba92b5e2cc66842f63061d73a549e961de0604a111f12874e35aa","pack_plan_digest":"424de8edf5b081002b0f909cc5221e13803ec8c0cfe4d94d46d5743758219d95","previous_receipt":null,"projection_engine_version":"1.0.0","receipt_id":"5ea0884d-1708-402a-aec7-644f7d114dbb","staging_digest":"e11ce1a6fa7e05aa6e4301e26de8a155b90eaa8303abb514948320aaf65ba9af","template_digest":"2661f75aba5776c117cf5af773cf4e40928b5a0cf0898857e036c53171a56b5c","verification_result":"passed","verified_at":"2026-06-06T22:54:08.415151Z","workspace_digest":"58f1dccfacfeb5d1d58cb224e3b00450e22bbd8ecf404b43cf996e8e0ee1ecfb"}
```
