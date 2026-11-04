# ZP-20 / Gate-09: fd/RSS churn court 3-run rerun

Lane: ZP-20 (IMPLEMENTATION, PL9). Date: 2026-11-04.
Namespacing: this is **lsp-max Gate-09 (fd-rss-churn)** — not cgap dflss GATE-09. Repos: /Users/sac/lsp-max (runs + this receipt), /Users/sac/ggen (PLANS wording). cgap: read-only.

## 1. Quiet precondition (zd-8 law)

Poll used `pgrep -fl lsp-echo-server`, never `pgrep -f "cargo test"` (self-match lesson zd-8).

- `pgrep -fl lsp-echo-server` → **no match, rc=1**.
- `ps aux | grep -E 'cargo|rustc' | grep -v grep` → one hit: pid 2831, a zsh wrapper for a `/tmp/aoq_kat` `cargo build` poll loop (sleep-15 idle loop), **no active cargo/rustc child**. Load average 9.18 (16 cores) — moderate.
- Disposition: proceeded without waiting; all numbers below are **upper bounds under disclosed load**.

## 2. 3-run rerun (serial, identical command)

`cargo test -p lsp-max-compositor --test child_churn_stress churn_200_cycles_spawn_exchange_sigkill_evict_respawn -- --nocapture`

fds was 10 at baseline and at every checkpoint in all three runs (delta 0).

| run | exit | rss base | cp50 | cp100 | cp150 | cp200/final | max ΔRSS KiB |
|-----|------|----------|------|-------|-------|-------------|--------------|
| 1 | 0 | 8736 | 11744 | 11792 | 11792 | 11792 | 3056 |
| 2 | 0 | 8784 | 11696 | 11728 | 11728 | 11776 | 2992 |
| 3 | 0 | 8816 | 11760 | 11792 | 11792 | 11808 | 2992 |

Cross-run observed max ΔRSS = **3056 KiB** (run 1).

All 3 runs PASS. fds flat at 10 (delta 0) — fd bound (+16) has effectively unbounded margin, ≥10× satisfied.

## 3. Decision rule (stated BEFORE interpreting, per PL9)

- Tighten-in-place iff all 3 pass AND ≥10× margin on both bounds under proposed tighter thresholds.
- RSS threshold candidate = max(observed_max_delta×10, 2MB) = max(30_560, 2_048) = **30_560 KiB**.
- **2MB fails the rule on all runs**: measured max deltas 2992–3056 KiB **exceed** 2MB (2048 KiB) outright, so 2MB is not 10×-margin-clearing and is in fact falsified by this rerun.
- 30_560 KiB is *looser* than the in-place slack 16_384 KiB (which gives 16_384/3056 = **5.4×** margin), so no "tightening" exists that satisfies the 10× rule without loosening the current bound.

**Outcome**: fd bound (+16) lands with trivial margin (delta 0, ~2 orders). RSS: neither 2MB (falsified: observed 3056 > 2048) nor the 10×-rule number (would loosen) is a lawful tighten; **keep in-place bound `first.rss*2 + 16_384` unchanged** at 5.4× margin; record PRD's **2MB as not-yet-derivable** (measured warm-up exceeds it; delta is a one-time heap warm-up — flat plateau cp50→cp200 within 64 KiB across all runs).

## 4. Anti-vacuity legs

Topology-guard deviation, disclosed: the /tmp sandbox copy was REFUSED by the repo-copy guard (one canonical checkout law), so both mutations ran as **in-place shim courts with sha-verified byte-exact restore** (protocol: baseline sha → shim → run → restore → sha diff).

Test file sha256 before/after, identical: `34331d54ca5db34ff6d8745c17e023b647a37bed0f47708f2ea397a0157a756a`.

- (a) Slack-widening (+16→+16_000, +16_384→+16_384_000): shim compiled and governed the asserts (grep witness at :334/:341 confirmed mutated bounds); court **PASSED** (exit 0). Honest labeling: with a passing baseline this leg cannot flip; it proves only that the bounds are the exercised code path, not sensitivity. **Minimum leg, honestly labeled.**
- (b) Leak-injection (kill skipped: `sigkill(pid);` → shim comment at :309/:665): court **FAILED** exit 101 (`poll_until` bounded poll panics — children never die). **Flip achieved.** Restore byte-exact.
- Post-restore confirmation run: PASS (1 passed, 13.24s).

## 5. Wording fix (ggen PLANS.md:27)

`C1 kill-mid-stream` → `C1 kill-mid-request` landed. Sweep-of-record `grep -rn 'stream' ggen/docs/jira/v26.11.4` returns 5 hits, none a court-name residue: PLANS.md:113 + PRD.md:156/:181 are the instruction lines themselves ("streams" → "requests" quotes); SEED.md:98 and zp-16 receipt:105 are `upstream`/`downstream` substrings. Task said "only the instruction line" — disclosed: 5 hits, all benign classes above.

## 6. Threshold verdict (PL9 §8 conditional checklist)

| condition | status |
|---|---|
| Rerun ✓ | 3/3 PASS, exit 0 |
| Anti-vacuity ✓ | leg (a) pass-only min leg; leg (b) flip achieved (101 on leak injection, byte-exact restore) |
| Wording ✓ | PLANS.md:27 kill-mid-request landed, sweep clean |
| Poll ✓ | pgrep -fl lsp-echo-server rc=1, noise disclosed (upper bounds) |
| Name ✓ | lsp-max/Gate-09 fd-rss-churn, not cgap dflss GATE-09 |
| RSS number measured-not-asserted | measured: 2992–3056 KiB one-time warm-up; PRD 2MB not-yet-derivable; bound kept 16_384 KiB |

Standing: **landed conditional** — fd bound landed as-is; RSS bound unchanged at 5.4× margin; 2MB PRD number falsified by rerun and must not be cited as a passing threshold.

Commands/exits: runs 0/0/0; mutated-wide 0; leak-injected 101; post-restore 0. Logs: /tmp/zp20_run{1,2,3}.log, /tmp/zp20_mutated.log, /tmp/zp20_leakinj.log.
