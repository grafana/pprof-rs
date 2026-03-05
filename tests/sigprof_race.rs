// Regression test for the SIGPROF race condition fixed in:
// https://github.com/grafana/pprof-rs/commit/978d3aa248fa19be6cc6f8488f1472cea98bf8a2
//
// The bug: unregister_signal_handler() restored the previous sigaction (SIG_DFL).
// SIGPROF's default action is to terminate the process. If a pending SIGPROF is
// delivered in the window between unregistering the handler and re-registering it
// (during rapid start/stop cycles), the process crashes.
//
// Run with:
//   cargo test --test sigprof_race -- --test-threads 1
//
// Without the fix, this test crashes the process with SIGPROF.
// With the fix (SIG_IGN instead of SIG_DFL restore), it completes cleanly.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const TEST_DURATION: Duration = Duration::from_secs(30);

#[test]
fn test_sigprof_race_crash() {
    // Spawn background threads that burn CPU to maximize SIGPROF delivery.
    // SIGPROF is delivered based on CPU time consumed by the process, so
    // more threads burning CPU = more frequent signal delivery = higher
    // chance of hitting the race window during guard drop/recreate.
    let running = Arc::new(AtomicBool::new(true));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let running = running.clone();
        handles.push(std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                std::hint::black_box(0u64.wrapping_add(1));
            }
        }));
    }

    // Rapidly cycle the profiler for TEST_DURATION. Each iteration creates
    // a guard (registers signal handler, starts timer) and drops it (stops
    // timer, unregisters handler). The main thread burns CPU between cycles
    // so SIGPROF gets delivered to it (Linux targets the thread consuming
    // CPU time). The race window is the moment SIG_DFL is restored before
    // the next iteration re-registers the handler.
    let deadline = Instant::now() + TEST_DURATION;
    while Instant::now() < deadline {
        let _guard = pprof::ProfilerGuard::new(999).unwrap();
        for _ in 0..50_000 {
            std::hint::black_box(0u64.wrapping_add(1));
        }
    }

    running.store(false, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }
}
