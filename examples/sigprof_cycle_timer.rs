// Fourth isolation step: install SIGPROF handler exactly ONCE and then only
// cycle setitimer(ITIMER_PROF) on/off at the same cadence as pprof-rs.
//
// - sigprof_minimal        install-once + timer-always-on  -> no crash
// - sigprof_cycle_noop     cycle sigaction + cycle timer   -> crashes
// - sigprof_cycle_timer    install-once + cycle timer      -> THIS FILE
// - sigbus_repro           pprof-rs                        -> crashes
//
// If THIS crashes, cycling setitimer alone is enough (the "install handler
// once" workaround would not help).
// If this survives, cycling sigaction specifically is the trigger, and the
// workaround does help.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

extern "C" {
    fn backtrace(array: *mut *mut libc::c_void, size: libc::c_int) -> libc::c_int;
    fn backtrace_symbols_fd(array: *const *mut libc::c_void, size: libc::c_int, fd: libc::c_int);
    fn setitimer(
        which: libc::c_int,
        new: *const Itimerval,
        old: *mut Itimerval,
    ) -> libc::c_int;
}

const ITIMER_PROF: libc::c_int = 2;

#[repr(C)]
#[derive(Clone)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Clone)]
struct Itimerval {
    it_interval: Timeval,
    it_value: Timeval,
}

static SIGPROF_COUNT: AtomicU64 = AtomicU64::new(0);

fn write_all(fd: libc::c_int, buf: &[u8]) {
    let mut p = buf.as_ptr();
    let mut left = buf.len();
    while left > 0 {
        let n = unsafe { libc::write(fd, p as *const libc::c_void, left) };
        if n <= 0 {
            return;
        }
        p = unsafe { p.add(n as usize) };
        left -= n as usize;
    }
}

fn write_hex(fd: libc::c_int, val: u64) {
    let mut buf = [0u8; 18];
    buf[0] = b'0';
    buf[1] = b'x';
    for i in 0..16 {
        let nibble = ((val >> ((15 - i) * 4)) & 0xf) as u8;
        buf[2 + i] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + (nibble - 10)
        };
    }
    write_all(fd, &buf);
}

fn write_dec(fd: libc::c_int, mut val: u64) {
    if val == 0 {
        write_all(fd, b"0");
        return;
    }
    let mut tmp = [0u8; 20];
    let mut n = 0;
    while val > 0 {
        tmp[n] = b'0' + (val % 10) as u8;
        val /= 10;
        n += 1;
    }
    let mut out = [0u8; 20];
    for i in 0..n {
        out[i] = tmp[n - 1 - i];
    }
    write_all(fd, &out[..n]);
}

extern "C" fn noop_sigprof(
    _sig: libc::c_int,
    _info: *mut libc::siginfo_t,
    _ucontext: *mut libc::c_void,
) {
    SIGPROF_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[allow(unused_variables)]
extern "C" fn fatal_signal_handler(
    sig: libc::c_int,
    info: *mut libc::siginfo_t,
    ucontext: *mut libc::c_void,
) {
    const STDERR: libc::c_int = 2;
    write_all(STDERR, b"\n=== fatal signal ");
    write_dec(STDERR, sig as u64);
    write_all(STDERR, b" captured ===\n");
    write_all(STDERR, b"sigprof_count=");
    write_dec(STDERR, SIGPROF_COUNT.load(Ordering::Relaxed));
    write_all(STDERR, b"\n");

    if !info.is_null() {
        write_all(STDERR, b"si_addr=");
        write_hex(STDERR, unsafe { (*info).si_addr() } as usize as u64);
        write_all(STDERR, b" si_code=");
        write_hex(STDERR, unsafe { (*info).si_code } as i64 as u64);
        write_all(STDERR, b"\n");
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let (pre_pc, pre_fp, pre_sp) = unsafe {
        if ucontext.is_null() {
            (0u64, 0u64, 0u64)
        } else {
            let uc = ucontext as *const libc::ucontext_t;
            let mcontext = (*uc).uc_mcontext;
            if mcontext.is_null() {
                (0, 0, 0)
            } else {
                let ss = &(*mcontext).__ss;
                (ss.__rip as u64, ss.__rbp as u64, ss.__rsp as u64)
            }
        }
    };
    #[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
    let (pre_pc, pre_fp, pre_sp) = (0u64, 0u64, 0u64);

    write_all(STDERR, b"pre_signal pc=");
    write_hex(STDERR, pre_pc);
    write_all(STDERR, b" fp=");
    write_hex(STDERR, pre_fp);
    write_all(STDERR, b" sp=");
    write_hex(STDERR, pre_sp);
    write_all(STDERR, b"\n");

    let mut frames: [*mut libc::c_void; 128] = [std::ptr::null_mut(); 128];
    let n = unsafe { backtrace(frames.as_mut_ptr(), 128) };
    write_all(STDERR, b"backtrace(3) (");
    write_dec(STDERR, n as u64);
    write_all(STDERR, b" frames):\n");
    unsafe { backtrace_symbols_fd(frames.as_ptr(), n, STDERR) };

    write_all(STDERR, b"=== end fatal signal ===\n");
    unsafe { libc::_exit(128 + sig) };
}

fn install_crash_handlers() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = fatal_signal_handler as usize;
        sa.sa_flags = libc::SA_SIGINFO;
        libc::sigemptyset(&mut sa.sa_mask);
        for sig in &[libc::SIGBUS, libc::SIGSEGV, libc::SIGILL, libc::SIGABRT] {
            libc::sigaction(*sig, &sa, std::ptr::null_mut());
        }
    }
}

fn install_sigprof_once() {
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = noop_sigprof as usize;
        sa.sa_flags = libc::SA_SIGINFO | libc::SA_RESTART;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGPROF, &sa, std::ptr::null_mut());
    }
}

fn start_timer(frequency: i64) {
    let interval = 1_000_000 / frequency;
    let tv = Timeval {
        tv_sec: interval / 1_000_000,
        tv_usec: interval % 1_000_000,
    };
    let it = Itimerval {
        it_interval: tv.clone(),
        it_value: tv,
    };
    unsafe {
        setitimer(ITIMER_PROF, &it, std::ptr::null_mut());
    }
}

fn stop_timer() {
    let it = Itimerval {
        it_interval: Timeval { tv_sec: 0, tv_usec: 0 },
        it_value: Timeval { tv_sec: 0, tv_usec: 0 },
    };
    unsafe {
        setitimer(ITIMER_PROF, &it, std::ptr::null_mut());
    }
}

fn main() {
    install_crash_handlers();
    install_sigprof_once();

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

    // 8000 cycles: start timer -> 50k busy loop -> stop timer.
    // sigaction(SIGPROF, …) is NEVER called inside the loop.
    for i in 0..8000 {
        start_timer(999);
        for _ in 0..50_000 {
            std::hint::black_box(0u64.wrapping_add(1));
        }
        stop_timer();
        if i % 500 == 0 {
            write_all(2, b"iter ");
            write_dec(2, i as u64);
            write_all(2, b" sigprofs=");
            write_dec(2, SIGPROF_COUNT.load(Ordering::Relaxed));
            write_all(2, b"\n");
        }
    }

    running.store(false, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }

    write_all(2, b"sigprof_cycle_timer: completed without a crash, sigprofs=");
    write_dec(2, SIGPROF_COUNT.load(Ordering::Relaxed));
    write_all(2, b"\n");
}
