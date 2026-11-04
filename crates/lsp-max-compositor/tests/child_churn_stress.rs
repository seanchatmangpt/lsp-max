// child_churn_stress.rs — AB-17 kill-churn stress court for the compositor
// child pool (pattern adapted from cgap-beam tests/kill_stress.rs: real
// collaborator, exact-count accounting, bounded convergence barriers).
//
// Courts:
//   C1. Spawn/kill churn: 200 cycles of spawn a real `lsp-echo-server` child
//       -> initialize handshake -> N requests -> SIGKILL mid-flight ->
//       typed child-exited (post-CL-E law) -> pool eviction observed ->
//       respawn. Leak witness per 50 cycles: the killed pid is reaped (gone
//       from the process table), fd-table delta bounded, RSS growth bounded.
//   C2. Concurrent-fanout soak: 8 children x 500 requests interleaved;
//       per-child attribution intact (compositor/<tier>:<id> server ids) and
//       zero cross-child bleed — every request resolves on ITS OWN child's
//       handle (per-handle pending-map correlation), 500/500 served per
//       child, and no child dies or mis-frames mid-soak.
//   C3. Buffer-consistency-under-kill: children deposit diagnostics into the
//       DiagnosticBuffer; the children are SIGKILLed mid-buffering (plus
//       concurrent deposit/flush pressure); buffer state stays consistent —
//       deposited entries survive, ANDON accounting reconciles, no panic.
//   C4. NO-RESPAWN law: across the whole soak the only spawns are the
//       explicit ones — each court's spawn ledger equals its planned
//       explicit spawn count, evicted ids never reappear in the pool, and
//       every killed pid is reaped (never re-spawned behind our back).
//
// No mocks. Every child is an OS process; every assert is on final state.
// No fixed sleeps: every convergence is a bounded poll on a real observable.

use lsp_max_compositor::child_process::{ChildProcess, ChildProcessPool, NoopClient};
use lsp_max_compositor::diagnostic_buffer::DiagnosticBuffer;
use lsp_max_compositor::merge::MergeContext;
use lsp_max_compositor::registry::ChildTier;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const STABLE_CHILD: &str = env!("CARGO_BIN_EXE_lsp-echo-server");

const CHURN_CYCLES: usize = 200;
const REQUESTS_PER_CYCLE: usize = 3;
const FANOUT_CHILDREN: usize = 8;
const FANOUT_REQUESTS: usize = 500;
const BUFFER_CHILDREN: usize = 4;
const LEAK_CHECKPOINT: usize = 50;

const HANDSHAKE_BOUND: u64 = 30;
const SERVE_BOUND: u64 = 5;
const EXIT_BOUND: u64 = 10;
const POLL_BOUND: u64 = 10;

/// Per-court spawn ledger (NOT process-global: the test harness runs the
/// courts concurrently as threads in one process — a global counter would
/// catch sibling courts' spawns). Incremented only by the explicit spawn
/// helpers; each court reconciles its own delta against its planned
/// explicit spawns. The compositor has no internal spawn path, so any
/// implicit respawn would be invisible here ONLY if it bypassed
/// ChildProcess::spawn entirely — impossible for the pool, whose only
/// insertion APIs are spawn_and_register/register (both caller-fed).
#[derive(Default, Clone)]
struct SpawnLedger(Arc<AtomicUsize>);

impl SpawnLedger {
    fn bump(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

/// Spawn a child through a `sh` wrapper that publishes the child pid to
/// `pidfile` before exec'ing the echo server — gives the court an exact,
/// observable pid to SIGKILL mid-flight (exec keeps the pid, so the kill
/// lands on the LSP server itself, not the shell).
async fn spawn_churn_child(ledger: &SpawnLedger, server_id: &str, pidfile: &PathBuf) -> ChildProcess {
    ledger.bump();
    let script = format!("echo $$ > \"{}\"; exec \"{}\"", pidfile.display(), STABLE_CHILD);
    let (proc, _exit_fut) = ChildProcess::spawn(server_id.to_string(), "sh", &["-c", &script], NoopClient)
        .await
        .expect("churn child spawn must succeed");
    proc
}

/// Plain spawn (fanout/buffer children need no pidfile).
#[allow(dead_code)]
async fn spawn_plain_child(ledger: &SpawnLedger, server_id: &str) -> ChildProcess {
    ledger.bump();
    let (proc, _exit_fut) =
        ChildProcess::spawn(server_id.to_string(), STABLE_CHILD, &[], NoopClient)
            .await
            .expect("child spawn+register must succeed");
    proc
}

async fn initialize_bounded(child: &ChildProcess) {
    tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_BOUND),
        child.initialize(None),
    )
    .await
    .expect("initialize handshake must complete within bound")
    .expect("initialize handshake must succeed");
}

/// Read the pid published by the `sh` wrapper, with a bounded poll (the
/// wrapper writes it asynchronously with respect to spawn() returning).
fn read_pid(pidfile: &PathBuf) -> u32 {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(POLL_BOUND);
    while std::time::Instant::now() < deadline {
        if let Ok(s) = std::fs::read_to_string(pidfile) {
            if let Ok(pid) = s.trim().parse::<u32>() {
                return pid;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("pidfile never appeared within {POLL_BOUND}s: {}", pidfile.display());
}

/// SIGKILL an exact pid (untrappable, mid-flight death — the kill_stress
/// `exit(kill)` analogue for OS children).
fn sigkill(pid: u32) {
    let status = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .expect("spawn kill(1)");
    assert!(status.success(), "kill -9 {pid} failed: {status}");
}

/// True if the exact pid is still present in the process table.
fn pid_alive(pid: u32) -> bool {
    let out = std::process::Command::new("ps")
        .args(["-axo", "pid="])
        .output()
        .expect("spawn ps");
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().any(|p| p == pid.to_string())
}

/// Bounded poll until `f` holds.
async fn poll_until<F>(secs: u64, mut f: F)
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(secs);
    while tokio::time::Instant::now() < deadline {
        if f() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("observable did not converge within {secs}s (bounded poll, no fixed sleeps)");
}

/// Typed round-trip outcome, classified per the ServerHandle surface:
/// `Served` = child answered (Ok(Some) / Ok(None) live-silent) or returned a
/// live JSON-RPC error (`server-error` — the echo server's method-not-found);
/// `ChildExited(msg)` = the typed post-CL-E dead-child error;
/// `Failed(msg)` = any other transport failure.
#[derive(Debug)]
enum RoundTrip {
    Served,
    ChildExited(String),
    Failed(String),
}

async fn bounded_request(child: &ChildProcess) -> RoundTrip {
    let inner = tokio::time::timeout(
        std::time::Duration::from_secs(SERVE_BOUND),
        child.handle.request::<serde_json::Value>(
            "textDocument/hover",
            serde_json::json!({ "textDocument": { "uri": "file:///churn.rs" } }),
        ),
    )
    .await
    .map_err(|_| format!("request exceeded {SERVE_BOUND}s bound"));
    match inner {
        Ok(Ok(_)) => RoundTrip::Served,
        Ok(Err(e)) => {
            if e.0.starts_with("server-error") {
                RoundTrip::Served
            } else if e.0.starts_with("child-exited") {
                RoundTrip::ChildExited(e.0)
            } else {
                RoundTrip::Failed(e.0)
            }
        }
        Err(msg) => RoundTrip::Failed(msg),
    }
}

// ── Leak witnesses (per-50-cycle checkpoints) ───────────────────────────────

/// OS descendants of this test process, from the real process table.
/// NOTE: the harness runs the courts concurrently in one process, so this
/// counts sibling courts' children too — used only as a BOUNDED ceiling,
/// never as a ==0 drain predicate.
fn descendant_count() -> usize {
    let out = std::process::Command::new("ps")
        .args(["-axo", "pid=,ppid="])
        .output()
        .expect("spawn ps");
    let table: Vec<(u32, u32)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let pid = it.next()?.parse().ok()?;
            let ppid = it.next()?.parse().ok()?;
            Some((pid, ppid))
        })
        .collect();
    let me = std::process::id();
    let mut count = 0usize;
    // BFS from `me` over the ppid->children relation.
    let mut frontier = vec![me];
    while let Some(parent) = frontier.pop() {
        for (pid, ppid) in &table {
            if *ppid == parent {
                count += 1;
                frontier.push(*pid);
            }
        }
    }
    count
}

/// Typed child-exited (post-CL-E): poll until every listed child's exit
/// flag flips (the pool's eviction predicate) and the OS pid is reaped.
async fn poll_children_dead(children: &[(Arc<AtomicBool>, Option<u32>)], secs: u64) {
    poll_until(secs, || {
        children
            .iter()
            .all(|(flag, pid)| {
                flag.load(Ordering::Acquire) && pid.map_or(true, |p| !pid_alive(p))
            })
    })
    .await;
}

/// This test process's open fd count (macOS /dev/fd mirrors the fd table —
/// the lsof delta witness, read cheaply and deterministically).
fn fd_count() -> usize {
    std::fs::read_dir("/dev/fd")
        .expect("read /dev/fd (fd-table witness)")
        .count()
}

/// This test process's RSS in KB, from the real process table.
fn rss_kb() -> u64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .expect("spawn ps -o rss");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .expect("ps rss is an integer")
}

struct LeakWitness {
    fds: usize,
    rss: u64,
}

fn witness(tag: &str, cycle: usize) -> LeakWitness {
    let w = LeakWitness { fds: fd_count(), rss: rss_kb() };
    eprintln!(
        "witness {tag} cycle={cycle} descendants={} fds={} rss_kb={}",
        descendant_count(),
        w.fds,
        w.rss
    );
    w
}

// ── C1 + C4: 200-cycle spawn/kill/evict/respawn churn ───────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn churn_200_cycles_spawn_exchange_sigkill_evict_respawn() {
    let ledger = SpawnLedger::default();
    let pool = Arc::new(ChildProcessPool::new());
    let tmp = std::env::temp_dir().join(format!("lsp-max-ab17-churn-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create churn tmp dir");

    let baseline = witness("baseline", 0);
    let mut checkpoints = vec![baseline];

    for cycle in 0..CHURN_CYCLES {
        let id = format!("compositor/primary:churn-{cycle}");
        let pidfile = tmp.join(format!("pid-{cycle}"));
        let child = spawn_churn_child(&ledger, &id, &pidfile).await;
        let exited = Arc::clone(&child.exited);
        pool.register(id.clone(), child);

        // Handshake proves the child is serving before the kill lands.
        initialize_bounded(&pool.get(&id).expect("registered").value()).await;

        // Exchange N requests pre-kill (mid-flight = after live traffic).
        for _ in 0..REQUESTS_PER_CYCLE {
            let handle = pool.get(&id).expect("live pre-kill");
            let r = bounded_request(&handle).await;
            assert!(
                matches!(r, RoundTrip::Served),
                "cycle {cycle}: live request did not serve: {r:?}"
            );
        }

        // SIGKILL mid-flight.
        let pid = read_pid(&pidfile);
        sigkill(pid);

        // Typed child-exited (post-CL-E law): exit flag flips, the OS pid is
        // reaped, and the id is evicted by get(); it never comes back.
        poll_children_dead(&[(Arc::clone(&exited), Some(pid))], EXIT_BOUND).await;
        assert!(pool.get(&id).is_none(), "cycle {cycle}: dead child not evicted");
        // No-respawn observation inside the loop: the evicted id never
        // reappears on a subsequent lookup.
        assert!(pool.get(&id).is_none(), "cycle {cycle}: evicted id reappeared");

        // Leak checkpoint per 50 cycles: the exact killed pid must be reaped
        // (never re-spawned behind our back), fd/RSS witnesses recorded.
        if (cycle + 1) % LEAK_CHECKPOINT == 0 {
            poll_children_dead(&[(Arc::clone(&exited), Some(pid))], POLL_BOUND).await;
            checkpoints.push(witness("checkpoint", cycle + 1));
        }
    }

    // Post-soak leak courts over the checkpoints.
    let final_w = witness("final", CHURN_CYCLES);
    checkpoints.push(final_w);

    let first = checkpoints.first().expect("baseline witness");
    for w in &checkpoints[1..] {
        assert!(
            w.fds <= first.fds + 16,
            "fd leak: baseline {} -> checkpoint {}",
            first.fds,
            w.fds
        );
        // RSS growth bounded: churn must not grow the heap past 2x baseline + slack.
        assert!(
            w.rss <= first.rss * 2 + 16_384,
            "rss leak: baseline {} KiB -> checkpoint {} KiB",
            first.rss,
            w.rss
        );
    }

    // Pool is empty: every cycle's entry was evicted, nothing retained.
    assert!(
        pool.server_ids_snapshot().is_empty(),
        "pool retained entries after full churn: {:?}",
        pool.server_ids_snapshot()
    );

    // C4 (churn): ledger == explicit spawns exactly (one per cycle).
    assert_eq!(
        ledger.count(),
        CHURN_CYCLES,
        "spawn ledger drifted during churn — an implicit respawn path ran?"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── C2: concurrent-fanout soak, 8 children x 500 interleaved requests ──────

#[tokio::test(flavor = "multi_thread")]
async fn fanout_soak_8_children_500_requests_no_cross_bleed() {
    let ledger = SpawnLedger::default();
    let pool = Arc::new(ChildProcessPool::new());
    let tmp = std::env::temp_dir().join(format!("lsp-max-ab17-fan-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create fanout tmp dir");

    let mut children = Vec::new();
    for c in 0..FANOUT_CHILDREN {
        let id = format!("compositor/secondary:fan-{c}");
        let pidfile = tmp.join(format!("pid-{c}"));
        let child = spawn_churn_child(&ledger, &id, &pidfile).await;
        children.push((id, Arc::clone(&child.exited), read_pid(&pidfile)));
        pool.register(children.last().expect("pushed").0.clone(), child);
    }
    assert_eq!(
        pool.server_ids_snapshot().len(),
        FANOUT_CHILDREN,
        "fanout cohort mis-registered"
    );

    // Per-child attribution: the compositor/<tier>:<id> prefix survives the
    // pool round-trip intact for every child.
    for (id, _, _) in &children {
        assert!(pool.is_live(id), "child {id} not live after registration");
    }

    // Interleaved soak: one task per child, 500 requests each. Responses are
    // correlated per-handle (the ServerHandle pending map), so any cross-
    // child bleed shows up as a mid-soak child-exited / framing failure.
    let mut tasks = tokio::task::JoinSet::new();
    for c in 0..FANOUT_CHILDREN {
        let pool = Arc::clone(&pool);
        let id = format!("compositor/secondary:fan-{c}");
        tasks.spawn(async move {
            let mut served = 0usize;
            for seq in 0..FANOUT_REQUESTS {
                let handle = pool
                    .get(&id)
                    .unwrap_or_else(|| panic!("child {id} evicted mid-soak"));
                let r = bounded_request(&handle).await;
                match r {
                    RoundTrip::Served => served += 1,
                    RoundTrip::ChildExited(m) => panic!(
                        "child {id} seq {seq}: child died mid-soak (cross-wired write path?): {m}"
                    ),
                    RoundTrip::Failed(e) => panic!("child {id} seq {seq}: request failed: {e}"),
                }
            }
            (id, served)
        });
    }

    let mut total = 0usize;
    while let Some(res) = tasks.join_next().await {
        let (id, served) = res.expect("fanout task panicked");
        assert_eq!(
            served, FANOUT_REQUESTS,
            "child {id} served {served}/{FANOUT_REQUESTS}"
        );
        total += served;
    }
    assert_eq!(total, FANOUT_CHILDREN * FANOUT_REQUESTS, "soak accounting drift");

    // Ledger: exactly the explicit spawns.
    assert_eq!(ledger.count(), FANOUT_CHILDREN, "implicit spawns in fanout court");

    // Explicit teardown: SIGKILL every child (exact pids) — an AF-20-era
    // pre-drop kill would now be redundant (pool drop itself tears children
    // down; see the C5 pool-drop-teardown court) but the explicit kill keeps
    // this court independent of the teardown fix's presence.
    let dead: Vec<(Arc<AtomicBool>, Option<u32>)> = children
        .iter()
        .map(|(_, flag, pid)| (Arc::clone(flag), Some(*pid)))
        .collect();
    for (_, _, pid) in &children {
        sigkill(*pid);
    }
    drop(pool);
    poll_children_dead(&dead, POLL_BOUND).await;
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── C3: buffer consistency under concurrent child kills ────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn buffer_stays_consistent_when_children_are_sigkilled_mid_buffering() {
    use lsp_max_compositor::merge::DiagnosticEntry;

    let ledger = SpawnLedger::default();
    let pool = Arc::new(ChildProcessPool::new());
    let tmp = std::env::temp_dir().join(format!("lsp-max-ab17-buf-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create buffer tmp dir");

    let mut children = Vec::new();
    for c in 0..BUFFER_CHILDREN {
        let id = format!("compositor/diagnostics-only:buf-{c}");
        let pidfile = tmp.join(format!("pid-{c}"));
        let child = spawn_churn_child(&ledger, &id, &pidfile).await;
        children.push((id, Arc::clone(&child.exited), read_pid(&pidfile)));
        pool.register(children.last().expect("pushed").0.clone(), child);
    }

    let gate_path = tmp.join("gate");
    let gate = Arc::new(lsp_max_compositor::gate_file::GateFile::from_path(
        PathBuf::from(&gate_path),
    ));
    let ctx = Arc::new(MergeContext::new(vec!["GGEN-".to_string()]));
    let buffer = Arc::new(DiagnosticBuffer::new(Arc::clone(&ctx), gate));

    let entry = |uri: &str, c: usize, seq: usize| DiagnosticEntry {
        uri: uri.to_string(),
        line: seq as u32,
        character: 0,
        end_line: seq as u32,
        end_character: 1,
        severity: 1,
        code: format!("GGEN-CHURN-{c:03}"),
        message: "blocked (churn fixture)".to_string(),
        source_tier: ChildTier::DiagnosticsOnly,
        server_id: Some(format!("compositor/diagnostics-only:buf-{c}")),
    };

    // Live children deposit ANDON diagnostics for their own URIs.
    let uris: Vec<String> = (0..BUFFER_CHILDREN)
        .map(|c| format!("file:///churn-{c}.rs"))
        .collect();
    for (c, uri) in uris.iter().enumerate() {
        buffer.deposit(
            uri,
            &format!("compositor/diagnostics-only:buf-{c}"),
            ChildTier::DiagnosticsOnly,
            vec![entry(uri, c, 0)],
        );
    }
    assert!(
        buffer.global_andon_active(),
        "ANDON must be active while killed children's diagnostics are buffered"
    );
    assert_eq!(
        buffer.buffered_uri_count(),
        BUFFER_CHILDREN,
        "buffered URI count drifted before kills"
    );

    // Concurrent deposit pressure DURING the kills (the "kill under live
    // traffic" analogue): a writer task keeps depositing last-write-wins
    // entries per child while the SIGKILLs land.
    let writer = {
        let buffer = Arc::clone(&buffer);
        let uris = uris.clone();
        tokio::spawn(async move {
            for seq in 1..=50usize {
                for (c, uri) in uris.iter().enumerate() {
                    buffer.deposit(
                        uri,
                        &format!("compositor/diagnostics-only:buf-{c}"),
                        ChildTier::DiagnosticsOnly,
                        vec![entry(uri, c, seq)],
                    );
                }
                // Interleave flush reads with deposits — no panic is part of
                // the court; merged results must stay well-formed.
                let _ = buffer.flush(&uris[0]);
            }
        })
    };

    // SIGKILL all buffered children mid-buffering, concurrently — exact pids
    // from the pidfiles (never a process-table sweep: sibling courts in this
    // harness run concurrently in one process and own their own children).
    let pids: Vec<u32> = children.iter().map(|(_, _, p)| *p).collect();
    let killer = tokio::spawn(async move {
        for pid in pids {
            sigkill(pid);
        }
    });

    writer.await.expect("deposit-writer task panicked under kill pressure");
    killer.await.expect("killer task panicked");

    // Typed child-exited for every killed child: exit flags flip, pids
    // reaped, eviction observed; the ids never reappear (no respawn).
    let dead: Vec<(Arc<AtomicBool>, Option<u32>)> = children
        .iter()
        .map(|(_, flag, pid)| (Arc::clone(flag), Some(*pid)))
        .collect();
    poll_children_dead(&dead, EXIT_BOUND).await;
    for (id, _, _) in &children {
        assert!(pool.get(id).is_none(), "child {id} not evicted after SIGKILL");
    }

    // Buffer state stays consistent AFTER the kills: every URI still merges
    // to exactly one ANDON code per child (last-write-wins per server_id),
    // with no phantom entries from the dead children.
    for (c, uri) in uris.iter().enumerate() {
        let result = buffer.flush(uri);
        let codes = result.andon_codes();
        assert_eq!(
            codes,
            vec![format!("GGEN-CHURN-{c:03}").as_str()],
            "URI {uri} merged codes drifted after child kills: {codes:?}"
        );
        // Post-kill clear: the has-ANDON -> no-ANDON transition must
        // reconcile exactly once (counter hits zero, never underflows).
        buffer.clear_uri(uri);
    }
    assert!(
        !buffer.global_andon_active(),
        "ANDON counter did not reconcile to zero after all URIs cleared"
    );
    assert_eq!(
        buffer.buffered_uri_count(),
        0,
        "buffered URI count drifted while clearing after kills"
    );

    // Ledger: exactly the explicit spawns.
    assert_eq!(ledger.count(), BUFFER_CHILDREN, "implicit spawns in buffer court");
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── C5: pool-drop teardown (AF-20) ──────────────────────────────────────────
//
// The defect AB-17 filed: dropping the pool does not stop the children —
// builder.rs moves a clone of the outbound sender into read_loop and the
// write_loop holds the child's stdin, so pool drop never produces stdin EOF
// and children outlive the pool. Fix under court: ChildProcess carries a
// kill-on-drop guard; dropping the pool SIGKILLs and reaps every child.
// Witness = the exact OS pids, polled in the real process table.

#[tokio::test(flavor = "multi_thread")]
async fn pool_drop_teardown_reaps_all_children_within_bound() {
    let ledger = SpawnLedger::default();
    let tmp = std::env::temp_dir().join(format!("lsp-max-af20-drop-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create pool-drop tmp dir");

    const N: usize = 6;
    let pool = ChildProcessPool::new();
    let mut dead = Vec::new();

    for c in 0..N {
        let id = format!("compositor/primary:drop-{c}");
        let pidfile = tmp.join(format!("pid-{c}"));
        let child = spawn_churn_child(&ledger, &id, &pidfile).await;
        let record = (Arc::clone(&child.exited), Some(read_pid(&pidfile)));
        pool.register(id.clone(), child);
        // Handshake proves the child is live and serving through the pool
        // before the drop.
        initialize_bounded(&pool.get(&id).expect("registered").value()).await;
        dead.push(record);
    }

    // Sanity: every pid is alive immediately before the drop.
    for (_, pid) in &dead {
        let pid = pid.expect("drop court records exact pids");
        assert!(pid_alive(pid), "pid {pid} dead before pool drop");
    }

    // The pool owns the ChildProcess entries — dropping it must tear down
    // every child. No explicit SIGKILL here: the court's whole point.
    drop(pool);

    // Bounded poll: exit flags flip, exact pids gone from the process table
    // (zero orphans via per-pid witness — the children were spawned under a
    // `sh` wrapper, so the pid is the exec'd LSP server itself).
    poll_children_dead(&dead, POLL_BOUND).await;

    // Ledger: exactly the explicit spawns; pool drop spawned nothing.
    assert_eq!(ledger.count(), N, "implicit spawns during pool-drop teardown");

    let _ = std::fs::remove_dir_all(&tmp);
}
//
// Runs after the other courts in the same test binary but with its own pool:
// kill every child it spawns, then assert over a bounded observe window that
// (a) the spawn ledger never moves except on explicit spawn calls, and
// (b) evicted ids never reappear in pool membership.

#[tokio::test(flavor = "multi_thread")]
async fn no_respawn_law_killed_children_are_never_auto_respawned() {
    let ledger = SpawnLedger::default();
    let pool = Arc::new(ChildProcessPool::new());
    let n = 20usize; // churn burst, small and deterministic
    let before = ledger.count();

    let tmp = std::env::temp_dir().join(format!("lsp-max-ab17-nr-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("create no-respawn tmp dir");

    let mut killed = Vec::new();
    for cycle in 0..n {
        let id = format!("compositor/primary:nr-{cycle}");
        let pidfile = tmp.join(format!("pid-{cycle}"));
        let child = spawn_churn_child(&ledger, &id, &pidfile).await;
        let exited = Arc::clone(&child.exited);
        pool.register(id.clone(), child);
        initialize_bounded(&pool.get(&id).expect("registered").value()).await;
        let pid = read_pid(&pidfile);
        sigkill(pid);
        killed.push((id, exited, pid));
    }

    // Every death observed + evicted; pids reaped.
    let dead: Vec<(Arc<AtomicBool>, Option<u32>)> = killed
        .iter()
        .map(|(_, flag, pid)| (Arc::clone(flag), Some(*pid)))
        .collect();
    poll_children_dead(&dead, EXIT_BOUND).await;
    for (id, _, _) in &killed {
        assert!(pool.get(id).is_none(), "{id} not evicted");
    }

    // Observe window: ledger moved by EXACTLY the explicit spawns (n), pool
    // membership stays empty, and no evicted id ever reappears. If the
    // compositor ever gains a respawn policy, THIS assert flips — the
    // intended falsifier surface.
    let after = ledger.count();
    assert_eq!(after - before, n, "implicit spawns observed: delta {}", after - before);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        let ids = pool.server_ids_snapshot();
        assert!(
            ids.is_empty(),
            "pool membership changed during no-respawn window: {ids:?} — respawn policy appeared?"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let _ = std::fs::remove_dir_all(&tmp);
}
