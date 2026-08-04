# Protocol Court

`protocol-court.json` is the machine-readable checkpoint ledger for the LSP
3.18 and LSIF 0.6.0 surfaces. It is generated from the canonical LSP ontology
and LSIF coverage matrix; it does not promote inspected code or test names into
execution receipts.

Regenerate and verify it with:

```bash
python3 scripts/protocol-court.py
python3 scripts/protocol-court.py --check
```

Each protocol entry carries its bounded status, emittable state, positive
witness, falsifier, consumer, receipt, and replay command. Missing evidence is
represented by `null`, not inferred. The CI conformance gate refuses a stale
projection.

`crown` is true only when every LSP method is admitted, every LSIF label is
consumer-admitted, and every entry has both a positive witness and receipt.
`PARTIAL_ALIVE` is therefore an expected truthful state while gaps remain.
