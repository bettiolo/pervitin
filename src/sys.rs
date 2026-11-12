//! The operating system: terminal modes, signals, clocks, and the handful of
//! commands pervitin runs. macOS only, and no crates: the few libc calls
//! needed are declared here.
use std::ffi::c_void;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub static STOP: AtomicI32 = AtomicI32::new(0);
pub static RESIZED: AtomicBool = AtomicBool::new(false);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    c_iflag: u64,
    c_oflag: u64,
    c_cflag: u64,
    c_lflag: u64,
    c_cc: [u8; 20],
    c_ispeed: u64,
    c_ospeed: u64,
}
#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}
#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}
#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const i8,
}
extern "C" {
    fn tcgetattr(fd: i32, t: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, act: i32, t: *const Termios) -> i32;
    fn ioctl(fd: i32, req: u64, ...) -> i32;
    fn poll(fds: *mut PollFd, n: u32, timeout: i32) -> i32;
    fn read(fd: i32, buf: *mut c_void, n: usize) -> isize;
    fn signal(sig: i32, h: extern "C" fn(i32)) -> usize;
    fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm;
    fn isatty(fd: i32) -> i32;
    fn __error() -> *mut i32;
}
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayIsAsleep(display: u32) -> u32;
    fn CGDisplayIsBuiltin(display: u32) -> u32;
}
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const u8) -> *const c_void;
    fn IOServiceGetMatchingService(port: u32, matching: *const c_void) -> u32;
    fn IORegistryEntryCreateCFProperty(entry: u32, key: *const c_void, alloc: *const c_void, opts: u32)
        -> *const c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFStringCreateWithCString(alloc: *const c_void, text: *const u8, encoding: u32) -> *const c_void;
    fn CFGetTypeID(value: *const c_void) -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFBooleanGetValue(value: *const c_void) -> u8;
    fn CFNumberGetTypeID() -> usize;
    fn CFNumberGetValue(number: *const c_void, kind: isize, value: *mut c_void) -> u8;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
    fn CFRelease(value: *const c_void);
}

const ECHO: u64 = 0x8;
const ICANON: u64 = 0x100;
const VMIN: usize = 16;
const VTIME: usize = 17;
const TIOCGWINSZ: u64 = 0x4008_7468;

extern "C" fn on_signal(sig: i32) {
    match sig {
        2 => STOP.store(130, Ordering::SeqCst),
        15 => STOP.store(143, Ordering::SeqCst),
        1 => STOP.store(129, Ordering::SeqCst),
        28 => RESIZED.store(true, Ordering::SeqCst),
        _ => {}
    }
}

/// Signals only set a flag; the main loop does the work.
pub fn trap_signals() {
    unsafe {
        for sig in [1, 2, 15, 28] {
            signal(sig, on_signal);
        }
    }
}

pub fn stdout_is_tty() -> bool {
    unsafe { isatty(1) == 1 }
}

/// No echo, and no line mode either: a terminal left in line mode with echo
/// off looks like a password prompt, and some terminals (Ghostty) then show
/// a lock over the window. Returns the modes to restore.
pub fn raw_mode() -> Option<Termios> {
    unsafe {
        let mut t: Termios = std::mem::zeroed();
        if tcgetattr(0, &mut t) != 0 {
            return None;
        }
        let saved = t;
        *SAVED.lock().unwrap() = Some(saved);
        t.c_lflag &= !(ECHO | ICANON);
        t.c_cc[VMIN] = 1;
        t.c_cc[VTIME] = 0;
        tcsetattr(0, 0, &t);
        Some(saved)
    }
}

/// Write to the terminal, shrugging if it has gone away: a closed window
/// must not stop sleep from being given back.
pub fn out(text: &str) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(text.as_bytes());
    let _ = stdout.flush();
}

static SAVED: std::sync::Mutex<Option<Termios>> = std::sync::Mutex::new(None);

/// Give the terminal back in a hurry: for the panic hook, which has nothing
/// but statics to go on.
pub fn emergency_restore() {
    out("\x1b[?2026l\x1b[?1004l\x1b[?1003l\x1b[?1006l\x1b[?25h\x1b[?7h\x1b[0m\x1b[?1049l");
    if let Ok(saved) = SAVED.try_lock() {
        restore_mode(&saved);
    }
}

pub fn restore_mode(saved: &Option<Termios>) {
    if let Some(t) = saved {
        unsafe {
            tcsetattr(0, 0, t);
        }
    }
}

/// Rows and columns of the window, if it will say.
pub fn window_size() -> Option<(i64, i64)> {
    unsafe {
        let mut w: Winsize = std::mem::zeroed();
        for fd in [1, 0, 2] {
            if ioctl(fd, TIOCGWINSZ, &mut w as *mut Winsize) == 0 && w.ws_row > 0 && w.ws_col > 0 {
                return Some((w.ws_row as i64, w.ws_col as i64));
            }
        }
    }
    None
}

pub enum Input {
    Data(Vec<u8>),
    Nothing,
    Closed,
}

/// Wait up to `secs` for keys or mouse reports on stdin.
pub fn wait_input(secs: f64) -> Input {
    let ms = if secs <= 0.0 { 0 } else { (secs * 1000.0).ceil() as i32 };
    unsafe {
        let mut p = PollFd { fd: 0, events: 1, revents: 0 };
        let n = poll(&mut p, 1, ms);
        if n < 0 {
            return Input::Nothing; // interrupted by a signal
        }
        if n == 0 {
            return Input::Nothing;
        }
        if p.revents & 1 == 0 {
            return Input::Closed;
        }
        let mut buf = [0u8; 4096];
        let got = read(0, buf.as_mut_ptr().cast(), buf.len());
        if got > 0 {
            Input::Data(buf[..got as usize].to_vec())
        } else if got == 0 {
            Input::Closed
        } else if matches!(*__error(), 4 | 35) {
            Input::Nothing // interrupted, or nothing there after all
        } else {
            Input::Closed
        }
    }
}

pub fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Local (hour, minute, second) of an epoch time.
pub fn local_hms(epoch: i64) -> (i32, i32, i32) {
    unsafe {
        let mut tm: Tm = std::mem::zeroed();
        localtime_r(&epoch, &mut tm);
        (tm.tm_hour, tm.tm_min, tm.tm_sec)
    }
}

pub fn stamp(epoch: i64) -> String {
    let (h, m, s) = local_hms(epoch);
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// Standard output of a command, or nothing if it could not run.
pub fn output_of(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

pub fn sleep_disabled() -> bool {
    output_of("/usr/bin/pmset", &["-g"]).lines().any(|l| {
        let mut f = l.split_whitespace();
        f.next() == Some("SleepDisabled") && f.next() == Some("1")
    })
}

/// `sudo pmset -a disablesleep N`. Quiet: never prompt, never print (used
/// while the animation owns the terminal).
pub fn set_disablesleep(on: bool, quiet: bool) -> bool {
    let mut c = Command::new("/usr/bin/sudo");
    if quiet {
        c.arg("-n").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    }
    c.args(["/usr/bin/pmset", "-a", "disablesleep", if on { "1" } else { "0" }]);
    c.status().map(|s| s.success()).unwrap_or(false)
}

/// A registry service by class name (NUL-terminated), or 0.
pub fn service(class: &[u8]) -> u32 {
    unsafe { IOServiceGetMatchingService(0, IOServiceMatching(class.as_ptr())) }
}

/// A value read out of the I/O registry, released when dropped.
pub struct Value(*const c_void);

impl Drop for Value {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

fn cf_key(name: &[u8]) -> Option<Value> {
    let key = unsafe { CFStringCreateWithCString(std::ptr::null(), name.as_ptr(), 0x0800_0100) };
    (!key.is_null()).then_some(Value(key))
}

fn cf_bool(value: *const c_void) -> Option<bool> {
    unsafe { (CFGetTypeID(value) == CFBooleanGetTypeID()).then(|| CFBooleanGetValue(value) != 0) }
}

fn cf_int(value: *const c_void) -> Option<i64> {
    unsafe {
        if CFGetTypeID(value) != CFNumberGetTypeID() {
            return None;
        }
        let mut n = 0i64;
        (CFNumberGetValue(value, 4, &mut n as *mut i64 as *mut c_void) != 0).then_some(n)
    }
}

impl Value {
    pub fn flag(&self) -> Option<bool> {
        cf_bool(self.0)
    }

    pub fn int(&self) -> Option<i64> {
        cf_int(self.0)
    }

    /// A number inside a dictionary value.
    pub fn number(&self, name: &[u8]) -> Option<i64> {
        let key = cf_key(name)?;
        unsafe {
            if CFGetTypeID(self.0) != CFDictionaryGetTypeID() {
                return None;
            }
            let inner = CFDictionaryGetValue(self.0, key.0);
            if inner.is_null() {
                None
            } else {
                cf_int(inner)
            }
        }
    }
}

/// One property (NUL-terminated name) of a registry entry.
pub fn property(entry: u32, name: &[u8]) -> Option<Value> {
    let key = cf_key(name)?;
    let value = unsafe { IORegistryEntryCreateCFProperty(entry, key.0, std::ptr::null(), 0) };
    (!value.is_null()).then_some(Value(value))
}

/// A service looked up once and kept.
fn cached(slot: &AtomicU32, class: &[u8]) -> Option<u32> {
    let mut entry = slot.load(Ordering::Relaxed);
    if entry == 0 {
        entry = service(class);
        slot.store(entry, Ordering::Relaxed);
    }
    (entry != 0).then_some(entry)
}

/// Whether the lid is closed, straight from the power management root
/// domain, which is where `ioreg` reads it too. None if it will not say.
fn clamshell_closed() -> Option<bool> {
    static ROOT: AtomicU32 = AtomicU32::new(0);
    property(cached(&ROOT, b"IOPMrootDomain\0")?, b"AppleClamshellState\0")?.flag()
}

fn clamshell_closed_ioreg() -> bool {
    output_of("/usr/sbin/ioreg", &["-r", "-k", "AppleClamshellState", "-d", "4"])
        .contains("\"AppleClamshellState\" = Yes")
}

pub fn lid_open() -> bool {
    !clamshell_closed().unwrap_or_else(clamshell_closed_ioreg)
}

pub fn screen_on() -> bool {
    unsafe { CGDisplayIsAsleep(CGMainDisplayID()) == 0 }
}

/// Whether anyone could be looking: the screen is lit, and a shut lid only
/// leaves something to look at when the main display is an external one.
pub fn display_seen(lid_open: bool, screen_on: bool) -> bool {
    screen_on && (lid_open || unsafe { CGDisplayIsBuiltin(CGMainDisplayID()) == 0 })
}

/// "100% charged", from the battery's registry entry, in the words
/// `pmset -g batt` uses. Empty on a Mac without a battery.
pub fn battery() -> String {
    static BATTERY: AtomicU32 = AtomicU32::new(0);
    let Some(entry) = cached(&BATTERY, b"AppleSmartBattery\0") else { return String::new() };
    let int = |name: &[u8]| property(entry, name).and_then(|v| v.int());
    let flag = |name: &[u8]| property(entry, name).and_then(|v| v.flag()).unwrap_or(false);
    let (Some(now), Some(full)) = (int(b"CurrentCapacity\0"), int(b"MaxCapacity\0")) else { return String::new() };
    if full <= 0 {
        return String::new();
    }
    let state = if !flag(b"ExternalConnected\0") {
        "discharging"
    } else if flag(b"IsCharging\0") {
        "charging"
    } else if flag(b"FullyCharged\0") {
        "charged"
    } else {
        "AC attached"
    };
    format!("{}% {}", (now * 100 + full / 2) / full, state)
}

/// What `pmset -g batt` says, for checking the above.
#[cfg(test)]
fn battery_pmset() -> String {
    for line in output_of("/usr/bin/pmset", &["-g", "batt"]).lines() {
        if !line.contains('%') {
            continue;
        }
        let fields: Vec<&str> = line.split(|ch| ch == ';' || ch == '\t').map(|f| f.trim_start()).collect();
        if fields.len() >= 3 {
            return format!("{} {}", fields[1], fields[2]);
        }
    }
    String::new()
}

/// The busiest process: (whole percent of a core, name). This one still
/// asks `ps`: only a root program may read the CPU time of processes owned
/// by others (WindowServer, kernel_task), and `ps` is one.
pub fn top_process() -> Option<(i64, String)> {
    let out = output_of("/bin/ps", &["-Aceo", "pcpu,comm", "-r"]);
    let line = out.lines().nth(1)?.trim_start();
    let (num, name) = line.split_once(' ')?;
    let whole: String = num.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    Some((whole.parse().ok()?, name.trim_start().to_string()))
}

// A small generator standing in for the shell's $RANDOM.
static SEED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0x9E37_79B9_7F4A_7C15);

pub fn seed() {
    let s = (now() * 1e6) as u64 ^ (std::process::id() as u64) << 32;
    SEED.store(s | 1, Ordering::Relaxed);
}

/// A random integer in 0..n.
pub fn rnd(n: i64) -> i64 {
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    if n <= 0 {
        0
    } else {
        ((x >> 33) % n as u64) as i64
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lid_state_matches_ioreg() {
        assert_eq!(super::clamshell_closed(), Some(super::clamshell_closed_ioreg()));
    }

    #[test]
    fn battery_matches_pmset() {
        assert_eq!(super::battery(), super::battery_pmset());
    }
}
