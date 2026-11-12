//! pervitin: keep the Mac awake on two stimulants, until Ctrl+C.
//!
//! PERVITIN is `sudo pmset -a disablesleep 1` (no sleep at all, lid closed
//! included); CAFFEINE is `caffeinate -dims` (no idle sleep, screen stays
//! on). Everything else is an animated ASCII laptop that shows how it feels
//! about that, and a medical chart of the machine's vitals.
mod art;
mod draw;
mod face;
mod hud;
mod sensors;
mod sys;

use std::collections::VecDeque;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use art::*;
use sys::rnd;

pub const W: i64 = 26; // screen interior width
pub const LAP_W: i64 = 52;
pub const LAP_H: i64 = 16;
pub const HUD_H: i64 = 18;
pub const HUD_MAX: i64 = 82;
pub const EVENTS_SHOWN: usize = 6;

pub const ZEN_MOODS: [&str; 8] = ["meditate", "cat", "tea", "blanket", "sunbeam", "stargazing", "smitten", "humming"];
pub const BOTH_MOODS: [&str; 15] = [
    "wired", "bloodshot", "paranoid", "grinding", "invincible", "itchy", "tripping", "glitch", "matrix", "bsod", "hal",
    "vaporwave", "arcade", "disco", "transcendent",
];
pub const PERV_MOODS: [&str; 10] =
    ["wired", "bloodshot", "paranoid", "grinding", "invincible", "itchy", "tripping", "hal", "vaporwave", "arcade"];
pub const CAF_MOODS: [&str; 4] = ["buzzed", "jitters", "chatty", "productive"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scene {
    Boot,
    Pill,
    Coffee,
    CrashPerv,
    CrashCaf,
    Greet,
    Comedown,
}

impl Scene {
    fn len(self) -> f64 {
        match self {
            Scene::Boot => 1.0,
            Scene::Pill | Scene::Coffee => 3.1,
            Scene::CrashPerv | Scene::Comedown => 3.8,
            Scene::CrashCaf => 3.6,
            Scene::Greet => 7.0,
        }
    }
}

/// What the background probes last saw. They run off the render loop
/// because each one starts a process.
#[derive(Default)]
pub struct Probe {
    pub lid_open: Option<bool>,
    pub screen_on: Option<bool>,
    pub unseen: bool,
    pub battery: Option<String>,
    pub top: Option<(i64, String)>,
    pub reading: Option<sensors::Reading>,
}

pub struct Creature {
    pub kind: &'static str,
    pub x: f64,
    pub y: f64,
    pub v: f64,
    pub age: i64,
}

pub struct App {
    // What was asked for.
    pub dry_run: bool,
    pub no_anim: bool,
    pub forced_stage: Option<&'static str>,
    pub muted: bool,
    // Doses: perv/caf say what is really keeping the machine awake;
    // vis_perv/vis_caf say which has visibly kicked in, which trails behind
    // while the swallowing animation plays.
    pub perv: bool,
    pub caf: bool,
    pub vis_perv: bool,
    pub vis_caf: bool,
    pub sel: i32,
    pub caf_child: Option<Child>,
    pub has_sensor: bool,
    pub restored: Option<bool>,
    // Clocks.
    pub frame: i64,
    pub now: f64,
    pub started: f64,
    pub slept_total: i64,
    pub last_tick: f64,
    pub dozed_at: f64,
    pub stage_until: f64,
    pub flash: String,
    pub flash_until: f64,
    pub coming_down: bool,
    // Mood.
    pub stage: &'static str,
    pub last_pool: (bool, bool, bool),
    pub hyper: bool,
    pub zen_mood: &'static str,
    pub zen_until: f64,
    pub zen_dx: &'static str,
    pub breath: f64,
    // Cutscenes.
    pub scene: Option<Scene>,
    pub scene_queue: VecDeque<Scene>,
    pub scene_start: f64,
    pub t: f64,
    pub greet_dozed: bool,
    pub greet_gone: i64,
    // The face, row by row, and how it is coloured (SGR parameters).
    pub r_brow: String,
    pub r_etop: String,
    pub r_eye: String,
    pub r_ebot: String,
    pub r_sweat: String,
    pub r_m1: String,
    pub r_m2: String,
    pub brow_color: String,
    pub eye_color: String,
    pub sweat_color: String,
    pub m_color: String,
    pub m2_color: Option<String>,
    pub sparks: String,
    pub steam: String,
    pub tagline: String,
    pub c: i32,
    pub c2: i32,
    pub pal: &'static [i32],
    pub banner_color: Option<i32>,
    pub jitter: i64,
    pub wobble: bool,
    pub bpm_scale: f64,
    pub bpm_bias: f64,
    pub cpu_mood_override: Option<&'static str>,
    pub kb_t: Vec<String>,
    pub kb_c: [i32; 17],
    // Things drawn over the finished frame.
    pub overlay: String,
    pub fluid: String,
    pub pill_on: bool,
    pub cup_on: bool,
    pub pill_x: f64,
    pub pill_y: f64,
    pub pill_r: f64,
    pub pill_rot: f64,
    pub creatures: Vec<Creature>,
    pub drips: Vec<(i64, f64)>,
    pub rain: Vec<(i64, f64, f64)>,
    // Layout, recomputed every frame from the window size.
    pub term_rows: i64,
    pub term_cols: i64,
    pub side: bool,
    pub compact: bool,
    pub show_banner: bool,
    pub title_w: Option<(usize, usize)>,
    pub ox: i64,
    pub oy: i64,
    pub lap_row0: i64,
    pub box_c0: i64,
    pub box_c1: i64,
    pub box_r1: i64,
    pub text_c0: i64,
    pub text_r0: i64,
    pub pill_big_r: f64,
    pub pill_cy: f64,
    pub mouth_x: f64,
    pub mouth_y: f64,
    pub left: Vec<String>,
    pub right: Vec<String>,
    // CPU = heart, GPU = brain.
    pub ecg_w: usize,
    pub ecg_trace: VecDeque<char>,
    pub eeg_trace: VecDeque<char>,
    pub ecg_pending: VecDeque<char>,
    pub ecg_speed: f64,
    pub eeg_phase: f64,
    pub ecg_clock: f64,
    pub ecg_due: f64,
    pub ecg_until_beat: f64,
    pub last_beat: f64,
    pub bpm: f64,
    // Vitals.
    pub probe: Arc<Mutex<Probe>>,
    pub cpu_t: Option<i64>,
    pub gpu_t: Option<i64>,
    pub sys_w: Option<String>,
    pub cpu_load: Option<i64>,
    pub gpu_load: Option<i64>,
    pub fans: bool,
    pub ram_total: Option<i64>,
    pub ram_used: Option<i64>,
    pub mem_pressure: Option<i64>,
    pub cpu_peak: i64,
    pub battery: String,
    pub top_proc: String,
    pub top_pcpu: i64,
    pub lid_state: Option<bool>,
    pub screen_state: Option<bool>,
    pub lid_since: i64,
    pub screen_since: i64,
    pub events: Vec<String>,
    pub model: String,
    pub boot_epoch: i64,
    // The mouse, as the terminal reports it (0-based cell), and being touched.
    pub mouse_x: i64,
    pub mouse_y: i64,
    pub mouse_at: f64,
    /// The terminal says whether its window has the keyboard.
    pub focused: bool,
    /// Lid shut or screen dark: nobody can be looking.
    pub unseen: bool,
    pub poke_eye: i32,
    pub poke_until: f64,
    pub poke_say: &'static str,
    pub key_hit: i64,
    pub key_until: f64,
    pub typed: String,
    pub tickle: f64,
    pub tickle_until: f64,
    pub title: String,
    // What is on screen, row by row, so that only changes are sent.
    pub painted: Vec<String>,
    pub painted_over: bool,
}

pub fn spaces(n: i64) -> String {
    " ".repeat(n.max(0) as usize)
}

pub fn rep(ch: char, n: i64) -> String {
    std::iter::repeat(ch).take(n.max(0) as usize).collect()
}

pub fn clen(s: &str) -> i64 {
    s.chars().count() as i64
}

/// The first `n` characters of `s`.
pub fn first(s: &str, n: i64) -> String {
    s.chars().take(n.max(0) as usize).collect()
}

/// `s` rotated left by `k` characters.
pub fn rot(s: &str, k: i64) -> String {
    let ch: Vec<char> = s.chars().collect();
    let k = (k.rem_euclid(ch.len().max(1) as i64)) as usize;
    ch[k..].iter().chain(ch[..k].iter()).collect()
}

/// One of a list, cycling.
pub fn pick<T: Copy>(list: &[T], i: i64) -> T {
    list[i.rem_euclid(list.len() as i64) as usize]
}

/// 3725 -> 01:02:05, 90061 -> 1d 01:01:01
pub fn duration(s: i64) -> String {
    let s = s.max(0);
    let hms = format!("{:02}:{:02}:{:02}", s % 86400 / 3600, s % 3600 / 60, s % 60);
    if s >= 86400 {
        format!("{}d {}", s / 86400, hms)
    } else {
        hms
    }
}

fn stage_name(name: &str) -> Option<&'static str> {
    BOTH_MOODS.iter().chain(CAF_MOODS.iter()).chain(ZEN_MOODS.iter()).copied().find(|m| *m == name)
}

impl App {
    fn new(dry_run: bool) -> App {
        let now = sys::now();
        let env = |k: &str| std::env::var(k).map(|v| v == "1").unwrap_or(false);
        let mut model = sys::output_of("/usr/sbin/scutil", &["--get", "ComputerName"]).trim().to_string();
        if model.is_empty() {
            model = sys::output_of("/usr/sbin/sysctl", &["-n", "hw.model"]).trim().to_string();
        }
        if model.is_empty() {
            model = "this Mac".into();
        }
        let model = first(&model, 28);
        let boot = sys::output_of("/usr/sbin/sysctl", &["-n", "kern.boottime"]);
        let boot_epoch = boot
            .split("sec = ")
            .nth(1)
            .and_then(|r| r.split(|ch: char| !ch.is_ascii_digit()).next())
            .and_then(|d| d.parse().ok())
            .unwrap_or(now as i64);
        App {
            dry_run,
            no_anim: env("PERVITIN_NO_INTRO"),
            forced_stage: std::env::var("PERVITIN_STAGE").ok().and_then(|s| stage_name(&s)),
            muted: env("PERVITIN_QUIET"),
            perv: true,
            caf: true,
            vis_perv: false,
            vis_caf: false,
            sel: 1,
            caf_child: None,
            has_sensor: false,
            restored: None,
            frame: 0,
            now,
            started: now,
            slept_total: 0,
            last_tick: 0.0,
            dozed_at: 0.0,
            stage_until: 0.0,
            flash: String::new(),
            flash_until: 0.0,
            coming_down: false,
            stage: "sober",
            last_pool: (false, false, false),
            hyper: false,
            zen_mood: "meditate",
            zen_until: 0.0,
            zen_dx: "",
            breath: 0.0,
            scene: None,
            scene_queue: VecDeque::new(),
            scene_start: 0.0,
            t: 0.0,
            greet_dozed: false,
            greet_gone: 0,
            r_brow: String::new(),
            r_etop: String::new(),
            r_eye: String::new(),
            r_ebot: String::new(),
            r_sweat: String::new(),
            r_m1: String::new(),
            r_m2: String::new(),
            brow_color: "37".into(),
            eye_color: "37".into(),
            sweat_color: "36".into(),
            m_color: "37".into(),
            m2_color: None,
            sparks: String::new(),
            steam: String::new(),
            tagline: String::new(),
            c: 250,
            c2: 250,
            pal: HOT,
            banner_color: None,
            jitter: 0,
            wobble: false,
            bpm_scale: 1.0,
            bpm_bias: 0.0,
            cpu_mood_override: None,
            kb_t: vec!["[]".to_string(); 17],
            kb_c: [0; 17],
            overlay: String::new(),
            fluid: String::new(),
            pill_on: false,
            cup_on: false,
            pill_x: 0.0,
            pill_y: 0.0,
            pill_r: 0.0,
            pill_rot: 0.0,
            creatures: Vec::new(),
            drips: Vec::new(),
            rain: Vec::new(),
            term_rows: 24,
            term_cols: 80,
            side: false,
            compact: false,
            show_banner: false,
            title_w: None,
            ox: 0,
            oy: 0,
            lap_row0: 0,
            box_c0: 0,
            box_c1: 80,
            box_r1: 24,
            text_c0: 0,
            text_r0: 16,
            pill_big_r: 14.0,
            pill_cy: 16.0,
            mouth_x: 23.5,
            mouth_y: 16.5,
            left: Vec::new(),
            right: Vec::new(),
            ecg_w: 36,
            ecg_trace: std::iter::repeat('▁').take(36).collect(),
            eeg_trace: std::iter::repeat('▁').take(36).collect(),
            ecg_pending: VecDeque::new(),
            ecg_speed: 10.0,
            eeg_phase: 0.0,
            ecg_clock: now,
            ecg_due: 0.0,
            ecg_until_beat: 0.0,
            last_beat: 0.0,
            bpm: 58.0,
            probe: Arc::new(Mutex::new(Probe::default())),
            cpu_t: None,
            gpu_t: None,
            sys_w: None,
            cpu_load: None,
            gpu_load: None,
            fans: false,
            ram_total: None,
            ram_used: None,
            mem_pressure: None,
            cpu_peak: 0,
            battery: sys::battery(),
            top_proc: "kernel_task".into(),
            top_pcpu: 0,
            lid_state: None,
            screen_state: None,
            lid_since: now as i64,
            screen_since: now as i64,
            events: Vec::new(),
            model,
            boot_epoch,
            mouse_x: -1,
            mouse_y: -1,
            mouse_at: 0.0,
            focused: true,
            unseen: false,
            poke_eye: 0,
            poke_until: 0.0,
            poke_say: "",
            key_hit: 0,
            key_until: 0.0,
            typed: String::new(),
            tickle: 0.0,
            tickle_until: 0.0,
            title: String::new(),
            painted: Vec::new(),
            painted_over: false,
        }
    }

    /// Caffeine: -w makes caffeinate quit by itself when pervitin dies, even
    /// on SIGKILL.
    fn set_caffeine(&mut self, on: bool) {
        if on {
            if self.dry_run || self.caf_child.is_some() {
                return;
            }
            let pid = std::process::id().to_string();
            self.caf_child = Command::new("/usr/bin/caffeinate")
                .args(["-dims", "-w", &pid])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok();
        } else if let Some(mut child) = self.caf_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn set_disablesleep(&self, on: bool, quiet: bool) -> bool {
        self.dry_run || sys::set_disablesleep(on, quiet)
    }

    /// Give sleep back. Safe to call twice; quiet = never prompt for a
    /// password (a failure is then retried, with a prompt, on the way out).
    fn restore_sleep(&mut self, quiet: bool) {
        if self.restored == Some(true) {
            return;
        }
        self.set_caffeine(false);
        let ok = self.set_disablesleep(false, quiet) && (self.dry_run || !sys::sleep_disabled());
        self.restored = Some(ok);
    }

    /// The probes: sensors, lid and screen once a second, battery and the
    /// busiest process every five.
    fn start_probes(&mut self) {
        let mut sensors = sensors::Sensors::new();
        self.has_sensor = sensors.has_thermometer();
        let shared = self.probe.clone();
        std::thread::spawn(move || {
            let mut tick = 0u64;
            loop {
                let (lid, screen) = (sys::lid_open(), sys::screen_on());
                let reading = sensors.read();
                {
                    let mut p = shared.lock().unwrap();
                    p.reading = Some(reading);
                    p.lid_open = Some(lid);
                    p.screen_on = Some(screen);
                    p.unseen = !sys::display_seen(lid, screen);
                }
                if tick % 5 == 0 {
                    let (battery, top) = (sys::battery(), sys::top_process());
                    let mut p = shared.lock().unwrap();
                    p.battery = Some(battery);
                    if top.is_some() {
                        p.top = top;
                    }
                }
                tick += 1;
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
        if !self.dry_run {
            // Keep the sudo timestamp fresh so the comedown never needs a
            // password. Dies with pervitin, so sudo is not kept warm forever.
            std::thread::spawn(|| loop {
                let _ = Command::new("/usr/bin/sudo")
                    .args(["-n", "-v"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                std::thread::sleep(std::time::Duration::from_secs(50));
            });
        }
    }

    /// An event: colour and text, time-stamped.
    pub fn log_note(&mut self, color: i32, text: &str) {
        let stamp = sys::stamp(sys::now() as i64);
        self.events.push(format!("  \x1b[38;5;240m{}\x1b[0m  \x1b[1;38;5;{}m{}\x1b[0m", stamp, color, text));
    }

    fn log_event(&mut self, word: &str, was: &str, color: i32, at: i64, lasted: i64) {
        self.events.push(format!(
            "  \x1b[38;5;240m{}\x1b[0m  \x1b[1;38;5;{}m{}\x1b[0m \x1b[38;5;245mwas {} for\x1b[0m {}",
            sys::stamp(at),
            color,
            word,
            was,
            duration(lasted)
        ));
    }

    fn refresh_probe(&mut self) {
        let (lid, screen, battery, top, reading) = {
            let mut p = self.probe.lock().unwrap();
            self.unseen = p.unseen;
            (p.lid_open, p.screen_on, p.battery.take(), p.top.take(), p.reading.take())
        };
        if let Some(b) = battery {
            self.battery = b;
        }
        if let Some((pcpu, name)) = top {
            self.top_pcpu = pcpu;
            self.top_proc = name;
        }
        if let Some(reading) = reading {
            self.read_sensors(&reading);
        }
        let (Some(lid), Some(screen)) = (lid, screen) else { return };
        let now = sys::now() as i64;
        if let Some(was) = self.lid_state {
            if was != lid {
                let lasted = now - self.lid_since;
                if lid {
                    self.log_event("lid opened   ", "closed", 46, now, lasted);
                } else {
                    self.log_event("lid closed   ", "open", 208, now, lasted);
                }
                self.on_lid(lid, lasted);
                self.lid_since = now;
            }
        }
        if let Some(was) = self.screen_state {
            if was != screen {
                let lasted = now - self.screen_since;
                if screen {
                    self.log_event("screen on    ", "off", 46, now, lasted);
                } else {
                    self.log_event("screen off   ", "on", 208, now, lasted);
                }
                self.screen_since = now;
            }
        }
        self.lid_state = Some(lid);
        self.screen_state = Some(screen);
    }

    /// The few numbers on the chart. A reading the machine withheld keeps
    /// its last value.
    fn read_sensors(&mut self, r: &sensors::Reading) {
        if let Some(v) = r.cpu_temp {
            self.cpu_t = Some(v as i64);
        }
        if let Some(v) = r.gpu_temp {
            self.gpu_t = Some(v as i64);
        }
        if let Some(v) = r.watts {
            // One decimal at most, cut rather than rounded.
            let whole = v.trunc() as i64;
            let tenth = ((v - v.trunc()) * 10.0) as i64;
            self.sys_w = Some(if v.fract() == 0.0 { format!("{}", whole) } else { format!("{}.{}", whole, tenth) });
        }
        if let Some(v) = r.cpu_load {
            self.cpu_load = Some((v * 100.0).round() as i64);
        }
        self.gpu_load = r.gpu_load.or(self.gpu_load);
        self.ram_total = r.ram_total.or(self.ram_total);
        self.ram_used = r.ram_used.or(self.ram_used);
        self.mem_pressure = r.pressure.or(self.mem_pressure);
        self.fans = r.fans;
        if let Some(t) = self.cpu_t {
            self.cpu_peak = self.cpu_peak.max(t);
        }
    }

    /// The lid moved: `open`, and how long it spent in the previous state.
    fn on_lid(&mut self, open: bool, lasted: i64) {
        if self.coming_down {
            return;
        }
        if !open {
            // Only pervitin survives a closed lid, so only then is there news.
            if self.perv {
                self.sound("closed");
            }
        } else if sys::now() - self.dozed_at > 15.0 {
            self.greet_dozed = false;
            self.greet_gone = lasted;
            self.creatures.clear();
            self.sound("opened");
            self.scene_queue.clear();
            self.start_scene(Scene::Greet);
        }
    }

    pub fn start_scene(&mut self, scene: Scene) {
        self.scene = Some(scene);
        self.scene_start = sys::now();
    }

    /// Moods rotate at random; which ones are on offer depends on the doses.
    /// Caffeine alone is merely jittery, pervitin alone goes properly wrong,
    /// and together they unlock the deep end.
    fn pick_stage(&mut self) {
        const BOTH: [&str; 17] = [
            "wired", "bloodshot", "paranoid", "grinding", "invincible", "itchy", "tripping", "glitch", "glitch",
            "matrix", "matrix", "bsod", "hal", "vaporwave", "arcade", "disco", "transcendent",
        ];
        let pool: &[&'static str] = match (self.vis_perv, self.vis_caf) {
            (true, true) => &BOTH,
            (true, false) => &PERV_MOODS,
            (false, true) => &CAF_MOODS,
            _ => {
                self.stage = "sober";
                return;
            }
        };
        let mut next = self.stage;
        while next == self.stage {
            next = pool[rnd(pool.len() as i64) as usize];
        }
        self.stage = next;
        self.stage_until = sys::now() + 12.0 + rnd(14) as f64;
    }

    /// Up/down step through the moods on offer by hand. The pick stays for a
    /// good while before the rotation resumes; a pinned mood moves.
    fn swap_mood(&mut self, dir: i64) {
        let dosed = self.vis_perv || self.vis_caf;
        let pool: &[&'static str] = match (self.vis_perv, self.vis_caf) {
            (true, true) => &BOTH_MOODS,
            (true, false) => &PERV_MOODS,
            (false, true) => &CAF_MOODS,
            _ => &ZEN_MOODS,
        };
        let cur = if dosed { self.stage } else { self.zen_mood };
        let at = pool.iter().position(|m| *m == cur).unwrap_or(0) as i64;
        let next = pick(pool, at + dir);
        let pinned_zen = self.forced_stage.map(|f| ZEN_MOODS.contains(&f));
        if dosed {
            self.stage = next;
            self.stage_until = sys::now() + 60.0;
            if pinned_zen == Some(false) {
                self.forced_stage = Some(next);
            }
        } else {
            self.zen_mood = next;
            self.zen_until = sys::now() + 90.0;
            if pinned_zen == Some(true) {
                self.forced_stage = Some(next);
            }
        }
    }

    /// Start or stop the selected substance, for real, and play its scene.
    fn toggle_dose(&mut self) {
        if self.sel == 1 {
            if !self.set_disablesleep(!self.perv, true) {
                self.flash = "sudo timed out: cannot change pmset".into();
                self.flash_until = sys::now() + 4.0;
                return;
            }
            self.scene_queue.clear();
            if self.perv {
                self.perv = false;
                self.vis_perv = false;
                self.log_note(208, "pervitin stopped   closing the lid sleeps again");
                self.start_scene(Scene::CrashPerv);
            } else {
                self.perv = true;
                self.log_note(46, "pervitin taken     awake with the lid closed");
                self.start_scene(Scene::Pill);
            }
        } else {
            self.scene_queue.clear();
            if self.caf {
                self.set_caffeine(false);
                self.caf = false;
                self.vis_caf = false;
                self.log_note(208, "caffeine stopped   screen and idle sleep allowed again");
                self.start_scene(Scene::CrashCaf);
            } else {
                self.set_caffeine(true);
                self.caf = true;
                self.log_note(46, "caffeine taken     screen stays on, no idle sleep");
                self.start_scene(Scene::Coffee);
            }
        }
    }

    /// Everything is placed from the terminal size, every frame, so resizing
    /// reflows live: the HUD sits beside the laptop when there is width for
    /// it, the banner is dropped when rows are scarce, the whole composition
    /// is centred, the ECG grows with the space, and the pill scales down to
    /// fit.
    fn layout(&mut self) {
        let (rows, cols) = (self.term_rows, self.term_cols);
        let ev_h = if self.events.is_empty() { 0 } else { self.events.len().min(EVENTS_SHOWN) as i64 + 2 };
        self.side = cols >= 110;
        let (mut content_h, total_w) =
            if self.side { (HUD_H.max(LAP_H), LAP_W + HUD_MAX) } else { (LAP_H + 1 + HUD_H, HUD_MAX) };
        // Stacked in a short window: squeeze out the blank lines to fit more.
        self.compact = !self.side && rows < content_h;
        // Titles: both in full when they fit side by side, else PERV and
        // CAF, else none (the HUD still shows both states).
        let (pw, cw) = (clen(BANNER_PERV[0]), clen(BANNER_CAF[0]));
        self.title_w = if cols >= 2 + pw + 4 + cw {
            Some((pw as usize, cw as usize))
        } else if cols >= 2 + 33 + 4 + 24 {
            Some((33, 24))
        } else {
            None
        };
        // Six rows of title, the ON/OFF labels, and a blank row to breathe.
        self.show_banner = self.title_w.is_some() && rows >= content_h + 8;
        if self.show_banner {
            content_h += 8;
        }
        self.ox = if cols > total_w { (cols - total_w) / 2 } else { 0 };
        // Centred vertically, nudged up only as far as the event log needs.
        let avail = rows - content_h;
        self.oy = if avail > 0 { avail / 2 } else { 0 };
        if avail - self.oy < ev_h {
            self.oy = if avail > ev_h { avail - ev_h } else { 0 };
        }
        self.lap_row0 = self.oy + if self.show_banner { 8 } else { 0 };

        let room = if self.side { cols - self.ox - LAP_W } else { cols - self.ox };
        let w = (room - 51).clamp(10, 60) as usize;
        if w != self.ecg_w {
            // Pad on the left and, when shrinking, keep the newest columns.
            for trace in [&mut self.ecg_trace, &mut self.eeg_trace] {
                while trace.len() > w {
                    trace.pop_front();
                }
                while trace.len() < w {
                    trace.push_front('▁');
                }
            }
            self.ecg_w = w;
        }

        // The box everything is drawn in, and within it where the text
        // (stats and event log) starts: decorations keep out of the text.
        self.box_c0 = self.ox;
        self.box_c1 = (self.ox + total_w).min(cols);
        if self.side {
            self.box_r1 = self.lap_row0 + (HUD_H + 2).max(LAP_H + 1 + ev_h);
            self.text_c0 = self.ox + LAP_W;
            self.text_r0 = self.lap_row0 + LAP_H + 1;
        } else {
            self.box_r1 = self.lap_row0 + LAP_H + 1 + HUD_H + ev_h + 2;
            self.text_c0 = self.ox;
            self.text_r0 = self.lap_row0 + LAP_H;
        }
        self.box_r1 = self.box_r1.min(rows);

        // As tall as the laptop (14 rows) unless the window is too narrow
        // for a capsule five radii long.
        self.pill_big_r = if cols < 72 { (cols - 2) as f64 / 5.0 } else { 14.0 };
        self.pill_big_r = self.pill_big_r.max(4.0);
        self.pill_cy = (self.lap_row0 + LAP_H) as f64 - self.pill_big_r / 2.0;
        // Centre of the open mouth: the row of r_m1, the middle of the screen.
        self.mouth_x = self.ox as f64 + 11.0 + W as f64 / 2.0 - 0.5;
        self.mouth_y = self.lap_row0 as f64 + 9.5;
    }

    fn reset_keys(&mut self) {
        for k in self.kb_t.iter_mut() {
            k.clear();
            k.push_str("[]");
        }
        self.kb_c = [0; 17];
    }

    /// How long each frame stays up: the one place the frame rate is
    /// decided. Smooth while something flies, slower for a cutscene, slower
    /// still for the chart alone, and a crawl when nobody is looking. Keep
    /// every value well under the five seconds that count as having slept.
    fn frame_period(&self) -> f64 {
        if self.pill_on || self.cup_on || self.scene == Some(Scene::Boot) {
            1.0 / 30.0
        } else if self.scene.is_some() {
            0.075
        } else if self.unseen {
            1.0
        } else if !self.focused {
            0.5
        } else {
            0.135
        }
    }

    /// One frame: the mood, any cutscene over it, then the whole window.
    /// Returns false once the comedown has finished.
    fn frame(&mut self) -> bool {
        self.frame += 1;
        self.now = sys::now();
        let now = self.now;
        // A jump in the clock means the machine slept (caffeine cannot hold
        // a closed lid, and neither can nothing).
        if self.last_tick > 0.0 && now - self.last_tick > 5.0 && !self.coming_down {
            self.greet_gone = (now - self.last_tick) as i64;
            self.slept_total += self.greet_gone;
            let note = format!("woke up            was asleep for {}", duration(self.greet_gone));
            self.log_note(117, &note);
            self.dozed_at = now;
            self.greet_dozed = true;
            self.sound("dozed");
            self.scene_queue.clear();
            self.start_scene(Scene::Greet);
        }
        self.last_tick = now;
        self.refresh_probe();
        if sys::RESIZED.swap(false, Ordering::SeqCst) {
            if let Some((rows, cols)) = sys::window_size() {
                self.term_rows = rows;
                self.term_cols = cols;
            }
            self.painted.clear();
        }
        // Now and then everything is sent again, in case something else
        // wrote to the terminal.
        if self.frame % 64 == 0 {
            self.painted.clear();
        }
        self.layout();

        // The mood, from whatever has kicked in.
        let pool = (true, self.vis_perv, self.vis_caf);
        if pool != self.last_pool || now >= self.stage_until {
            self.pick_stage();
            self.last_pool = pool;
        }
        if let Some(forced) = self.forced_stage {
            if !ZEN_MOODS.contains(&forced) && self.stage != "sober" {
                self.stage = forced;
            }
        }
        self.pill_on = false;
        self.cup_on = false;
        self.sweat_color = "36".into();
        self.eye_color = "37".into();
        self.m_color = "37".into();
        self.brow_color = "37".into();
        self.m2_color = None;
        self.overlay.clear();
        self.fluid.clear();
        self.banner_color = None;
        self.wobble = false;
        self.reset_keys();
        self.bpm_scale = 1.0;
        self.bpm_bias = 0.0;
        self.cpu_mood_override = None;
        if self.stage == "sober" {
            self.hyper = false;
            self.face_sober();
        } else {
            self.hyper = true;
            self.face_wired();
            self.mood();
            // Both at once: everything shakes twice as hard.
            if self.vis_perv && self.vis_caf && self.stage != "transcendent" {
                self.jitter *= 2;
            }
        }

        // The cutscene, if one is playing, has the last word on the face.
        if let Some(scene) = self.scene {
            self.t = now - self.scene_start;
            if self.t >= scene.len() {
                if scene == Scene::Comedown {
                    return false;
                }
                self.scene = None;
                if let Some(next) = self.scene_queue.pop_front() {
                    self.start_scene(next);
                    self.t = 0.0;
                }
            }
            // A cutscene starts from plain colours, whatever the mood painted.
            if self.scene.is_some() {
                self.eye_color = "37".into();
                self.sweat_color = "36".into();
                self.m_color = "37".into();
                self.brow_color = "37".into();
                self.m2_color = None;
                self.overlay.clear();
                self.reset_keys();
                self.play_scene();
            }
        }
        if self.scene.is_none() {
            self.touched();
        }
        self.jitter = self.jitter.min(8);
        if now < self.flash_until {
            self.tagline = self.flash.clone();
        }

        // Hallucinations: in the tripping mood, or whenever the GPU is flat out.
        if self.scene.is_none() {
            if self.stage == "tripping" || (self.vis_perv && self.gpu_load.unwrap_or(0) >= 70) {
                if self.creatures.len() < 7 && rnd(6) == 0 {
                    self.spawn_creature();
                }
                if let (true, Some((pw, cw))) = (self.show_banner, self.title_w) {
                    if self.drips.len() < 10 && rnd(3) == 0 && self.stage == "tripping" {
                        self.drips.push((self.ox + 2 + rnd(pw as i64 + 4 + cw as i64), (self.oy + 5) as f64));
                    }
                }
            } else {
                // The trip is over: whatever was walking about vanishes.
                self.creatures.clear();
                self.drips.clear();
            }
        }
        if self.stage != "matrix" {
            self.rain.clear();
        }
        self.update_creatures();
        self.update_drips();
        self.update_monitors();

        // A frame is far bigger than the terminal reads in one go, and rows
        // are cleared before what lies over them is drawn again. Bracketed as
        // a synchronized update, the terminal shows it only once it is whole.
        let out = format!("\x1b[?2026h{}\x1b[?2026l", self.compose());
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();
        true
    }

    /// Keys and mouse reports. Returns true if a key (not just the mouse)
    /// was handled, so the next frame can show it at once.
    fn handle_input(&mut self, buf: &mut Vec<u8>) -> bool {
        let mut keyed = false;
        let mut i = 0;
        while i < buf.len() {
            let b = buf[i];
            if b != 0x1b {
                i += 1;
                keyed = true;
                if self.coming_down {
                    continue;
                }
                match b {
                    b' ' => self.toggle_dose(),
                    b'k' if self.scene.is_none() => self.swap_mood(-1),
                    b'j' if self.scene.is_none() => self.swap_mood(1),
                    b'h' | b'p' => self.sel = 1,
                    b'l' | b'c' => self.sel = 2,
                    b'\t' => self.sel = 3 - self.sel,
                    b'm' => self.muted = !self.muted,
                    _ => {}
                }
                continue;
            }
            // Arrow keys arrive as ESC [ C / ESC [ D (or ESC O C / ESC O D),
            // the mouse as ESC [ < button ; column ; row M.
            if i + 2 >= buf.len() {
                break; // the rest has not arrived yet
            }
            if buf[i + 1] != b'[' && buf[i + 1] != b'O' {
                i += 1;
                continue;
            }
            if buf[i + 1] == b'[' && (buf[i + 2] == b'I' || buf[i + 2] == b'O') {
                // The window gained or lost the keyboard.
                self.focused = buf[i + 2] == b'I';
                keyed = true;
                i += 3;
                continue;
            }
            if buf[i + 2] != b'<' {
                keyed = true;
                if !self.coming_down {
                    match buf[i + 2] {
                        b'C' => self.sel = 2,
                        b'D' => self.sel = 1,
                        b'A' if self.scene.is_none() => self.swap_mood(-1),
                        b'B' if self.scene.is_none() => self.swap_mood(1),
                        _ => {}
                    }
                }
                i += 3;
                continue;
            }
            let Some(end) = buf[i + 3..].iter().position(|ch| *ch == b'M' || *ch == b'm') else {
                if buf.len() - i > 24 {
                    i += 3; // not a report after all
                    continue;
                }
                break;
            };
            let body = String::from_utf8_lossy(&buf[i + 3..i + 3 + end]).into_owned();
            let press = buf[i + 3 + end] == b'M';
            i += 4 + end;
            let nums: Vec<i64> = body.split(';').filter_map(|n| n.parse().ok()).collect();
            if nums.len() != 3 {
                continue;
            }
            let (x, y) = (nums[1] - 1, nums[2] - 1);
            if x != self.mouse_x || y != self.mouse_y {
                self.mouse_x = x;
                self.mouse_y = y;
                self.mouse_at = sys::now();
                // Wiggling over the mouth tickles.
                if y >= self.lap_row0 + 8 && y <= self.lap_row0 + 11 && x >= self.ox + 14 && x <= self.ox + 34 {
                    self.tickle += 1.0;
                }
            }
            // Button 0 going down is a click: on a title it takes or stops
            // that substance, anywhere else it pokes the laptop.
            if nums[0] == 0 && press && !self.coming_down {
                if let Some(title) = self.title_at(x, y) {
                    self.sel = title;
                    self.toggle_dose();
                } else if self.scene.is_none() {
                    self.poke(x, y);
                }
            }
        }
        buf.drain(..i);
        keyed
    }

    /// Which title, 1 for PERVITIN or 2 for CAFFEINE, is under this cell:
    /// its letters or the ON/OFF label beneath them.
    fn title_at(&self, x: i64, y: i64) -> Option<i32> {
        let (pw, cw) = self.title_w.filter(|_| self.show_banner)?;
        let (pw, cw) = (pw as i64, cw as i64);
        if y < self.oy || y > self.oy + 6 {
            return None;
        }
        // Two columns of margin, then each title and half the gap after it.
        match x - self.ox - 2 {
            c if c < 0 => None,
            c if c < pw + 2 => Some(1),
            c if c < pw + 4 + cw + 2 => Some(2),
            _ => None,
        }
    }

    /// Leave the alternate screen, give sleep back, and say what happened.
    fn comedown(&mut self, saved: &Option<sys::Termios>) {
        sys::out("\x1b[?2026l\x1b[?1004l\x1b[?1003l\x1b[?1006l\x1b]0;\x07\x1b[23;0t\x1b[?25h\x1b[?7h\x1b[0m\x1b[?1049l");
        // Swallow mouse reports still in flight so they do not land in the shell.
        while let sys::Input::Data(_) = sys::wait_input(0.05) {}
        sys::restore_mode(saved);

        let elapsed = (sys::now() - self.started) as i64;
        self.restore_sleep(false);
        let status = if self.restored == Some(true) {
            "\x1b[1;32mSleep re-enabled.\x1b[0m"
        } else {
            "\x1b[1;31mFAILED to re-enable sleep — run: pervitin off\x1b[0m"
        };
        let mut text =
            format!("\n  \x1b[38;5;117m(-_-) zzZ\x1b[0m  coming down after {} of pure wakefulness", duration(elapsed));
        if self.cpu_peak > 0 {
            text += &format!(" (CPU peaked at {}°C)", self.cpu_peak);
        }
        text += &format!(".\n  {status} Your MacBook may now nap like a normal machine.\n\n");
        if !self.events.is_empty() {
            text += "  \x1b[38;5;245mwhat happened while you were away:\x1b[0m\n";
            for ev in &self.events {
                text += ev;
                text.push('\n');
            }
            text.push('\n');
        }
        sys::out(&text);
    }
}

fn main() {
    let dry_run = std::env::var("PERVITIN_DRY_RUN").map(|v| v == "1").unwrap_or(false);
    match std::env::args().nth(1).as_deref() {
        Some("off") => {
            if dry_run || sys::set_disablesleep(false, false) {
                println!("Sleep re-enabled.");
                return;
            }
            std::process::exit(1);
        }
        Some("status") => {
            if sys::sleep_disabled() {
                println!("Sleep is DISABLED (lid close will not sleep).");
            } else {
                println!("Sleep is enabled.");
            }
            return;
        }
        Some("-h") | Some("--help") | Some("help") => {
            print!("{}", HELP);
            return;
        }
        None => {}
        Some(other) => {
            eprintln!("pervitin: unknown argument '{}' (try: pervitin help)", other);
            std::process::exit(2);
        }
    }
    if !sys::stdout_is_tty() {
        eprintln!("pervitin: needs an interactive terminal");
        std::process::exit(1);
    }

    if !dry_run {
        if sys::sleep_disabled() {
            println!("pervitin: sleep was already disabled; it will be re-enabled on exit.");
        }
        let primed = Command::new("/usr/bin/sudo").arg("-v").status().map(|s| s.success()).unwrap_or(false);
        if !primed || !sys::set_disablesleep(true, false) {
            std::process::exit(1);
        }
    }

    // Whatever goes wrong, the terminal and sleep are given back.
    std::panic::set_hook(Box::new(move |info| {
        sys::emergency_restore();
        if !dry_run {
            sys::set_disablesleep(false, true);
        }
        let _ = writeln!(std::io::stderr(), "pervitin: {info}\npervitin: if sleep is still disabled, run: pervitin off");
    }));
    sys::seed();
    sys::trap_signals();
    let mut app = App::new(dry_run);
    app.set_caffeine(true);
    app.start_probes();
    app.make_heartbeats();
    if let Some((rows, cols)) = sys::window_size() {
        app.term_rows = rows;
        app.term_cols = cols;
    }

    sys::out("\x1b[22;0t\x1b[?1049h\x1b[?25l\x1b[?7l\x1b[2J\x1b[?1003h\x1b[?1006h\x1b[?1004h");
    let saved = sys::raw_mode();

    if app.no_anim {
        app.vis_perv = true;
        app.vis_caf = true;
    } else {
        app.start_scene(Scene::Boot);
        app.scene_queue = VecDeque::from([Scene::Pill, Scene::Coffee]);
    }

    let mut pending: Vec<u8> = Vec::new();
    let mut input_open = true;
    let mut code;
    loop {
        let stop = sys::STOP.swap(0, Ordering::SeqCst);
        if stop != 0 {
            code = stop;
            if stop == 130 && !app.coming_down && !app.no_anim {
                // Ctrl+C: give sleep back first, then play the comedown. A
                // second Ctrl+C skips it.
                app.coming_down = true;
                app.restore_sleep(true);
                app.perv = false;
                app.caf = false;
                app.vis_perv = false;
                app.vis_caf = false;
                app.scene_queue.clear();
                app.creatures.clear();
                app.drips.clear();
                app.start_scene(Scene::Comedown);
            } else {
                break;
            }
        }
        let began = sys::now();
        if !app.frame() {
            code = 130;
            break;
        }
        // The wait doubles as the keyboard and mouse poll.
        let period = app.frame_period();
        let mut fresh = false;
        loop {
            let left = began + period - sys::now();
            if left <= 0.0 || sys::STOP.load(Ordering::SeqCst) != 0 {
                break;
            }
            if !input_open {
                std::thread::sleep(std::time::Duration::from_secs_f64(left));
                break;
            }
            match sys::wait_input(left) {
                sys::Input::Data(bytes) => {
                    fresh = true;
                    pending.extend_from_slice(&bytes);
                    if app.handle_input(&mut pending) && sys::now() - began > 0.02 {
                        break;
                    }
                }
                sys::Input::Closed => input_open = false,
                sys::Input::Nothing => {}
            }
        }
        // Half an escape sequence that never completed is not worth keeping.
        if !fresh {
            pending.clear();
        }
    }
    app.comedown(&saved);
    std::process::exit(code);
}
