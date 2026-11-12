//! The body's readings, asked of the machine directly: temperatures, watts
//! and fans from the SMC, load and memory from the kernel, GPU load from
//! the graphics driver's registry entry. No sudo, and no helper program.
use crate::sys;
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct KeyInfo {
    size: u32,
    kind: u32,
    attributes: u8,
}
/// The one structure the SMC speaks, 80 bytes in and 80 out.
#[repr(C)]
#[derive(Clone, Copy)]
struct KeyData {
    key: u32,
    vers: [u16; 3],
    p_limit: [u32; 4],
    info: KeyInfo,
    result: u8,
    status: u8,
    command: u8,
    index: u32,
    bytes: [u8; 32],
}
const _: () = assert!(std::mem::size_of::<KeyData>() == 80);

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const u8) -> *const c_void;
    fn IOServiceGetMatchingServices(port: u32, matching: *const c_void, iter: *mut u32) -> i32;
    fn IOIteratorNext(iter: u32) -> u32;
    fn IORegistryEntryGetName(entry: u32, name: *mut u8) -> i32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOServiceOpen(service: u32, task: u32, kind: u32, conn: *mut u32) -> i32;
    fn IOConnectCallStructMethod(
        conn: u32,
        selector: u32,
        input: *const c_void,
        input_size: usize,
        output: *mut c_void,
        output_size: *mut usize,
    ) -> i32;
}
extern "C" {
    static mach_task_self_: u32;
    fn mach_host_self() -> u32;
    fn host_statistics(host: u32, flavor: i32, info: *mut u32, count: *mut u32) -> i32;
    fn host_statistics64(host: u32, flavor: i32, info: *mut u32, count: *mut u32) -> i32;
    fn sysctlbyname(name: *const u8, old: *mut c_void, old_len: *mut usize, new: *const c_void, new_len: usize)
        -> i32;
}

const READ_BYTES: u8 = 5;
const READ_INDEX: u8 = 8;
const READ_INFO: u8 = 9;
const FLOAT: u32 = u32::from_be_bytes(*b"flt ");

const fn code(name: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*name)
}

/// One sample. Anything the machine would not say stays None.
#[derive(Default)]
pub struct Reading {
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub watts: Option<f64>,
    /// Share of all cores that were busy since the last sample, 0 to 1.
    pub cpu_load: Option<f64>,
    /// Percent, as the graphics driver reports it.
    pub gpu_load: Option<i64>,
    pub ram_total: Option<i64>,
    pub ram_used: Option<i64>,
    pub pressure: Option<i64>,
    pub fans: bool,
}

pub struct Sensors {
    smc: u32,
    cpu_keys: Vec<(u32, KeyInfo)>,
    gpu_keys: Vec<(u32, KeyInfo)>,
    watts_key: Option<(u32, KeyInfo)>,
    fans: bool,
    gpu: u32,
    host: u32,
    page: i64,
    ram_total: Option<i64>,
    ticks: Option<[u32; 4]>,
    round: u64,
    temps: (Option<f64>, Option<f64>),
}

fn sysctl_i64(name: &[u8]) -> Option<i64> {
    let mut value = [0u8; 8];
    let mut len = 8usize;
    let rc = unsafe { sysctlbyname(name.as_ptr(), value.as_mut_ptr() as *mut c_void, &mut len, std::ptr::null(), 0) };
    match (rc, len) {
        (0, 8) => Some(i64::from_ne_bytes(value)),
        (0, 4) => Some(i32::from_ne_bytes([value[0], value[1], value[2], value[3]]) as i64),
        _ => None,
    }
}

impl Sensors {
    /// Opens the SMC and finds its thermometers. Their names differ from
    /// chip to chip, so every key is asked once: the float ones named Tp…,
    /// Te… and Ts… sit on the CPU, Tg… on the GPU (the same choice macmon
    /// makes, so the numbers agree with it).
    pub fn new() -> Sensors {
        let mut s = Sensors {
            smc: open_smc(),
            cpu_keys: Vec::new(),
            gpu_keys: Vec::new(),
            watts_key: None,
            fans: false,
            gpu: sys::service(b"IOAccelerator\0"),
            host: unsafe { mach_host_self() },
            page: sysctl_i64(b"hw.pagesize\0").unwrap_or(16384),
            ram_total: sysctl_i64(b"hw.memsize\0"),
            ticks: None,
            round: 0,
            temps: (None, None),
        };
        if s.smc == 0 {
            return s;
        }
        let count = s
            .info(code(b"#KEY"))
            .and_then(|info| s.bytes(code(b"#KEY"), info))
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0);
        for index in 0..count.min(10_000) {
            let Some(key) = s.key_at(index) else { continue };
            let name = key.to_be_bytes();
            let cpu = [&b"Tp"[..], b"Te", b"Ts"].iter().any(|p| name.starts_with(p));
            let gpu = name.starts_with(b"Tg");
            if !cpu && !gpu {
                continue;
            }
            let Some(info) = s.info(key) else { continue };
            if info.kind != FLOAT || info.size != 4 || s.float(key, info).is_none() {
                continue;
            }
            if cpu { &mut s.cpu_keys } else { &mut s.gpu_keys }.push((key, info));
        }
        let watts = code(b"PSTR");
        s.watts_key = s.info(watts).filter(|i| i.kind == FLOAT && i.size == 4).map(|i| (watts, i));
        let fans = code(b"FNum");
        s.fans = s.info(fans).and_then(|i| s.bytes(fans, i)).map(|b| b[0] > 0).unwrap_or(false);
        s
    }

    /// Whether there is a thermometer at all.
    pub fn has_thermometer(&self) -> bool {
        !self.cpu_keys.is_empty()
    }

    fn call(&self, input: &KeyData) -> Option<KeyData> {
        let mut output: KeyData = unsafe { std::mem::zeroed() };
        let mut size = std::mem::size_of::<KeyData>();
        let rc = unsafe {
            IOConnectCallStructMethod(
                self.smc,
                2,
                input as *const KeyData as *const c_void,
                size,
                &mut output as *mut KeyData as *mut c_void,
                &mut size,
            )
        };
        (rc == 0 && output.result == 0).then_some(output)
    }

    fn ask(&self, command: u8, key: u32, info: KeyInfo, index: u32) -> Option<KeyData> {
        let mut input: KeyData = unsafe { std::mem::zeroed() };
        input.command = command;
        input.key = key;
        input.info = info;
        input.index = index;
        self.call(&input)
    }

    fn key_at(&self, index: u32) -> Option<u32> {
        self.ask(READ_INDEX, 0, KeyInfo::default(), index).map(|d| d.key)
    }

    fn info(&self, key: u32) -> Option<KeyInfo> {
        self.ask(READ_INFO, key, KeyInfo::default(), 0).map(|d| d.info)
    }

    fn bytes(&self, key: u32, info: KeyInfo) -> Option<[u8; 32]> {
        self.ask(READ_BYTES, key, info, 0).map(|d| d.bytes)
    }

    fn float(&self, key: u32, info: KeyInfo) -> Option<f64> {
        let b = self.bytes(key, info)?;
        let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64;
        v.is_finite().then_some(v)
    }

    /// The mean of the thermometers that answer. One that is switched off
    /// says zero and is left out.
    fn mean(&self, keys: &[(u32, KeyInfo)]) -> Option<f64> {
        let warm: Vec<f64> =
            keys.iter().filter_map(|(key, info)| self.float(*key, *info)).filter(|v| *v > 0.0 && *v < 150.0).collect();
        (!warm.is_empty()).then(|| warm.iter().sum::<f64>() / warm.len() as f64)
    }

    /// Busy ticks over all ticks since the last call.
    fn cpu_load(&mut self) -> Option<f64> {
        let mut now = [0u32; 4]; // user, system, idle, nice
        let mut count = 4u32;
        if unsafe { host_statistics(self.host, 3, now.as_mut_ptr(), &mut count) } != 0 {
            return None;
        }
        let before = self.ticks.replace(now)?;
        let d: Vec<f64> = (0..4).map(|i| now[i].wrapping_sub(before[i]) as f64).collect();
        let all: f64 = d.iter().sum();
        (all > 0.0).then(|| (all - d[2]) / all)
    }

    /// What Activity Monitor calls memory used: app, wired and compressed.
    fn ram_used(&self) -> Option<i64> {
        let mut vm = [0u32; 64];
        let mut count = 64u32;
        if unsafe { host_statistics64(self.host, 4, vm.as_mut_ptr(), &mut count) } != 0 || count < 36 {
            return None;
        }
        // Offsets, in 32-bit words, into vm_statistics64.
        let (active, inactive, wired) = (vm[1] as i64, vm[2] as i64, vm[3] as i64);
        let (purgeable, speculative) = (vm[22] as i64, vm[23] as i64);
        let (compressor, external) = (vm[32] as i64, vm[34] as i64);
        Some((active + inactive + wired + speculative + compressor - purgeable - external) * self.page)
    }

    fn gpu_load(&self) -> Option<i64> {
        if self.gpu == 0 {
            return None;
        }
        let stats = sys::property(self.gpu, b"PerformanceStatistics\0")?;
        stats.number(b"Device Utilization %\0")
    }

    /// One sample. Each thermometer is a separate question to the SMC and
    /// there are dozens, which makes them the dear part, so they are only
    /// asked every third time; heat moves slowly anyway.
    pub fn read(&mut self) -> Reading {
        if self.round % 3 == 0 {
            self.temps = (self.mean(&self.cpu_keys), self.mean(&self.gpu_keys));
        }
        self.round += 1;
        Reading {
            cpu_temp: self.temps.0,
            gpu_temp: self.temps.1,
            watts: self.watts_key.and_then(|(key, info)| self.float(key, info)),
            cpu_load: self.cpu_load(),
            gpu_load: self.gpu_load(),
            ram_total: self.ram_total,
            ram_used: self.ram_used(),
            pressure: sysctl_i64(b"kern.memorystatus_level\0").map(|free| 100 - free),
            fans: self.fans,
        }
    }
}

/// A connection to the SMC, or 0. Apple Silicon hangs the keys off a child
/// of AppleSMC; whichever entry lets itself be opened is the one.
fn open_smc() -> u32 {
    unsafe {
        let mut iter = 0u32;
        if IOServiceGetMatchingServices(0, IOServiceMatching(b"AppleSMC\0".as_ptr()), &mut iter) != 0 {
            return 0;
        }
        let (mut best, mut fallback) = (0u32, 0u32);
        loop {
            let entry = IOIteratorNext(iter);
            if entry == 0 {
                break;
            }
            let mut name = [0u8; 128];
            IORegistryEntryGetName(entry, name.as_mut_ptr());
            let mut conn = 0u32;
            if IOServiceOpen(entry, mach_task_self_, 0, &mut conn) == 0 && conn != 0 {
                if name.starts_with(b"AppleSMCKeysEndpoint\0") && best == 0 {
                    best = conn;
                } else if fallback == 0 {
                    fallback = conn;
                }
            }
            IOObjectRelease(entry);
        }
        IOObjectRelease(iter);
        if best != 0 {
            best
        } else {
            fallback
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn prints_a_reading() {
        let mut s = super::Sensors::new();
        s.read();
        std::thread::sleep(std::time::Duration::from_millis(1000));
        let began = std::time::Instant::now();
        let r = s.read();
        let took = began.elapsed();
        let names = |keys: &[(u32, super::KeyInfo)]| {
            keys.iter().map(|(k, _)| String::from_utf8_lossy(&k.to_be_bytes()).into_owned()).collect::<Vec<_>>().join(" ")
        };
        println!("cpu keys: {}", names(&s.cpu_keys));
        println!("gpu keys: {}", names(&s.gpu_keys));
        println!(
            "cpu {:?} gpu {:?} watts {:?} load {:?} gpu% {:?} ram {:?}/{:?} pressure {:?} fans {} took {:?}",
            r.cpu_temp, r.gpu_temp, r.watts, r.cpu_load, r.gpu_load, r.ram_used, r.ram_total, r.pressure, r.fans, took
        );
        assert!(s.has_thermometer());
        assert!(r.cpu_temp.is_some() && r.watts.is_some() && r.cpu_load.is_some() && r.ram_used.is_some());
    }
}
