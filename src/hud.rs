//! The frame itself: titles, the laptop, and the medical chart beside (or
//! under) it, painted row by row at absolute addresses and clipped to the
//! window, so a short terminal never scrolls the art out from under the pill.
use std::fmt::Write as _;

use crate::art::*;
use crate::sys;
use crate::{clen, duration, first, pick, rep, spaces, App, Scene, EVENTS_SHOWN, LAP_H, LAP_W, W};

const LAB: &str = "\x1b[38;5;245m";
const DIM: &str = "\x1b[38;5;240m";
const END: &str = "\x1b[0m";
const EOL: &str = "\x1b[K";
const EEG_NAMES: [&str; 3] = ["δ", "β", "γ"];

/// Bytes as e.g. "6.6 GB".
fn gib(bytes: i64) -> String {
    format!("{:.1} GB", bytes as f64 / 1073741824.0)
}

/// File `lines` under rows `row`.. (0-based) of a column.
fn put(column: &mut [String], row: i64, lines: Vec<String>) {
    for (i, line) in lines.into_iter().enumerate() {
        let r = row + i as i64;
        if r >= 0 && (r as usize) < column.len() {
            column[r as usize] = line;
        }
    }
}

impl App {
    /// A laptop-screen row: text colour (SGR parameters) and centred text.
    fn screen_row(&self, pad: &str, color: &str, text: &str) -> String {
        let len = clen(text);
        let left = (W - len) / 2;
        let c = self.c;
        format!(
            "{pad}\x1b[38;5;{c}m   |  |\x1b[1;{color}m{}{text}{}\x1b[0;38;5;{c}m|  |{END}{EOL}",
            spaces(left),
            spaces(W - len - left)
        )
    }

    /// "<label> NN°C <caption>", coloured and captioned by how cooked it is.
    fn thermo(&self, label: &str, t: i64) -> String {
        let (color, mood) = if t < 60 && self.stage == "sober" {
            (self.c, "still water")
        } else if t < 60 {
            (46, "chill (suspicious)")
        } else if t < 75 {
            (226, "WARM")
        } else if t < 85 {
            (208, "SPICY")
        } else if t < 95 {
            (196, "MELTING")
        } else {
            (201, "CALL THE FIRE BRIGADE")
        };
        format!("{label} \x1b[1m{t}°C{END} \x1b[38;5;{color}m{mood}{END}")
    }

    /// Both titles side by side (PERV and CAF where the window is narrow).
    /// Each is lit while its substance is working and dark while it is not;
    /// the selected one carries the marker underneath.
    fn titles(&self, pw: usize, cw: usize) -> Vec<String> {
        let f = self.frame;
        let bp = self.pal;
        let scene_sub = match self.scene {
            Some(Scene::Pill) | Some(Scene::CrashPerv) => 1,
            Some(Scene::Coffee) | Some(Scene::CrashCaf) => 2,
            _ => 0,
        };
        // CAFFEINE is full of liquid: coffee, cream on top, in the plain
        // moods; otherwise it takes the mood's colours, layer by layer.
        let mut liq = [223, 215, 172, 130, 94];
        if bp != COFFEE && self.stage != "wired" {
            for (lv, layer) in liq.iter_mut().enumerate() {
                *layer = pick(bp, lv as i64 + 1 + f / 4);
            }
        }
        let mut lines = Vec::with_capacity(7);
        for i in 1..=6i64 {
            let mut out = if self.wobble {
                spaces(2 + (1.5 + 1.5 * (f as f64 * 0.4 + i as f64 * 0.9).sin()) as i64)
            } else {
                "  ".to_string()
            };
            for k in 1..=2 {
                let (row, lit, width) = if k == 1 {
                    (BANNER_PERV[i as usize - 1], self.vis_perv, pw)
                } else {
                    (BANNER_CAF[i as usize - 1], self.vis_caf, cw)
                };
                let flat = match self.banner_color {
                    Some(color) if scene_sub == 0 || scene_sub == k => Some(color),
                    _ if !lit => Some(238),
                    _ if k == 1 => Some(pick(bp, i + f)),
                    _ => None,
                };
                if let Some(color) = flat {
                    let _ = write!(out, "\x1b[1;38;5;{color}m{}{END}    ", first(row, width as i64));
                    continue;
                }
                // A surface that sloshes along the letters in slow waves.
                let mut last = -1;
                for (x, ch) in row.chars().take(width).enumerate() {
                    let x = (x + 1) as f64;
                    let d = i as f64
                        - (2.2 + (x * 0.16 + self.now * 2.2).sin() + 0.6 * (x * 0.07 - self.now * 1.3).sin());
                    let color = if d < -0.6 {
                        239
                    } else if d < 0.4 {
                        liq[0]
                    } else if d < 1.4 {
                        liq[1]
                    } else if d < 2.4 {
                        liq[2]
                    } else if d < 3.4 {
                        liq[3]
                    } else {
                        liq[4]
                    };
                    if color != last {
                        let _ = write!(out, "\x1b[1;38;5;{color}m");
                        last = color;
                    }
                    out.push(ch);
                }
                out.push_str("\x1b[0m    ");
            }
            out.push_str(EOL);
            lines.push(out);
        }
        let mut out = String::from("  ");
        for k in 1..=2 {
            let (on, lit, width) = if k == 1 { (self.perv, 226, pw) } else { (self.caf, 172, cw) };
            let color = if on { lit } else { 245 };
            let label = match (k == self.sel, on) {
                (true, true) => "▸ ON · space stops it",
                (true, false) => "▸ OFF · space takes it",
                (false, true) => "ON",
                (false, false) => "OFF",
            };
            let style = if k == self.sel { "1;7;" } else { "" };
            let _ = write!(out, "\x1b[{style}38;5;{color}m {label} {END}{}", spaces(width as i64 + 2 - clen(label)));
        }
        out.push_str(EOL);
        lines.push(out);
        lines
    }

    /// The laptop: sparks and sound effects above the lid, the face on the
    /// screen, and a keyboard of seventeen keys, any of which a mood may
    /// have pressed, lit or replaced.
    fn laptop(&self, pad: &str) -> Vec<String> {
        let (c, c2) = (self.c, self.c2);
        let shell = |art: &str| format!("{pad}\x1b[38;5;{c}m{art}{END}{EOL}");
        let mut keys = String::new();
        for (text, color) in self.kb_t.iter().zip(self.kb_c) {
            if color != 0 {
                let _ = write!(keys, "\x1b[38;5;{color}m{text}\x1b[38;5;{c}m");
            } else {
                keys.push_str(text);
            }
        }
        vec![
            format!("{pad}\x1b[38;5;{c2}m      {}{END}{EOL}", self.sparks),
            format!("{pad}\x1b[38;5;250m        {}{END}{EOL}", self.steam),
            shell("   .--------------------------------."),
            shell("   |  .--------------------------.  |"),
            self.screen_row(pad, &self.brow_color, &self.r_brow),
            self.screen_row(pad, &self.eye_color, &self.r_etop),
            self.screen_row(pad, &self.eye_color, &self.r_eye),
            self.screen_row(pad, &self.eye_color, &self.r_ebot),
            self.screen_row(pad, &self.sweat_color, &self.r_sweat),
            self.screen_row(pad, &self.m_color, &self.r_m1),
            self.screen_row(pad, self.m2_color.as_ref().unwrap_or(&self.m_color), &self.r_m2),
            shell("   |  '--------------------------'  |"),
            shell("   |                                |"),
            shell(" .-'--------------------------------'-."),
            shell(&format!("/  {keys}  \\")),
            shell("'--------------------------------------'"),
        ]
    }

    /// The medical chart: one labelled line per organ, then what was
    /// prescribed (Rx) and the diagnosis (Dx).
    fn chart(&self, stage_name: &str) -> Vec<String> {
        let (c, c2) = (self.c, self.c2);
        let wall = sys::now();
        let epoch = wall as i64;
        let beating = wall - self.last_beat < 0.15;
        let gpu = self.gpu_load.unwrap_or(0);
        let load = self.cpu_load.unwrap_or(0);
        // The cardiology of CPU load.
        let mut cpu_mood = match load {
            _ if load < 5 => "resting (on stimulants)",
            _ if load < 15 => "palpitations",
            _ if load < 30 => "TACHYCARDIC",
            _ if load < 50 => "HUMMINGBIRD MODE",
            _ if load < 70 => "CHEST PAINS",
            _ if load < 85 => "HEART ATTACK",
            _ if load < 95 => "CODE BLUE",
            _ => "CLEAR! *BZZT*",
        };
        if let Some(mood) = self.cpu_mood_override {
            cpu_mood = mood;
        }
        let mut gpu_mood = match gpu {
            _ if gpu >= 90 => "EGO DEATH",
            _ if gpu >= 70 => "FULL HALLUCINATION",
            _ if gpu >= 35 => "SEEING COLOURS",
            _ if gpu >= 10 => "thinking in pixels",
            _ => "daydreaming",
        };
        let mut heart = if beating { "\x1b[1;38;5;196m♥".to_string() } else { "\x1b[38;5;52m♥".to_string() };
        let (mut ecg_c, mut hot_c, mut brain_c) = (46, "1;38;5;196".to_string(), "1;38;5;201".to_string());
        // Sober, the whole chart is as calm as the face: no alarm colours,
        // no shouting.
        if !self.hyper {
            cpu_mood = match load {
                _ if load < 15 => "at peace",
                _ if load < 40 => "mildly stirred",
                _ if load < 75 => "meditating very hard",
                _ => "inner turmoil",
            };
            if gpu < 35 {
                gpu_mood = "empty mind";
            }
            ecg_c = c;
            hot_c = format!("38;5;{c}");
            brain_c = hot_c.clone();
            heart = if beating { format!("\x1b[38;5;{c}m♥") } else { "\x1b[38;5;238m♥".to_string() };
        }

        let room = if self.side { self.term_cols - self.ox - LAP_W } else { self.term_cols - self.ox };
        let rule = format!("  {DIM}{}{END}{EOL}", rep('─', (room - 4).clamp(10, 68)));
        let mut hud: Vec<String> = Vec::with_capacity(20);

        let elapsed = (wall - self.started) as i64 - self.slept_total;
        let up = epoch - self.boot_epoch;
        let note = if up < 86400 {
            "still fresh"
        } else if up < 604800 {
            "getting ripe"
        } else {
            "refuses to reboot"
        };
        hud.push(format!(
            "  {LAB}PATIENT {END} \x1b[1m{}{END} {LAB}· admitted{END} \x1b[1;38;5;226m{}{END} {LAB}ago · up{END} {}{LAB}, {note}{END}{EOL}",
            self.model,
            duration(elapsed),
            duration(up)
        ));
        hud.push(rule.clone());

        let ecg: String = self.ecg_trace.iter().collect();
        let mut line = format!(
            "  {LAB}PULSE   {END} {heart}{END} \x1b[38;5;{ecg_c}m{ecg}{END} \x1b[{hot_c}m{:>3} BPM{END}",
            self.bpm as i64
        );
        if let Some(load) = self.cpu_load {
            let _ = write!(line, " {LAB}{load:>3}%{END} \x1b[{hot_c}m{cpu_mood}{END}");
        }
        hud.push(line + EOL);

        let eeg: String = self.eeg_trace.iter().collect();
        let waves = EEG_NAMES[self.vis_perv as usize + self.vis_caf as usize];
        let mut line = format!(
            "  {LAB}BRAIN   {END} \x1b[1;38;5;{c2}m✺{END} \x1b[38;5;{c2}m{eeg}{END} \x1b[{brain_c}m {waves} waves{END}"
        );
        if let Some(gpu) = self.gpu_load {
            let _ = write!(line, " {LAB}{gpu:>3}%{END} \x1b[{brain_c}m{gpu_mood}{END}");
        }
        hud.push(line + EOL);

        let mut line = format!("  {LAB}TEMP    {END} ");
        if let Some(t) = self.cpu_t {
            line += &self.thermo("core", t);
            // An idle, power-gated GPU reports a bogus ~5°C; only show real readings.
            if let Some(g) = self.gpu_t.filter(|g| *g >= 15) {
                let _ = write!(line, " {LAB}·{END} gfx \x1b[1m{g}°C{END}");
            }
            let fans = if self.fans {
                ("fans on", "hyperventilating")
            } else if self.hyper {
                ("no fans", "sweating passively")
            } else {
                ("no fans", "perfectly still")
            };
            let _ = write!(line, " {LAB}·{END} {}{LAB}, {}{END}", fans.0, fans.1);
        } else if !self.has_sensor {
            let _ = write!(line, "no thermometer{LAB} on this machine{END}");
        } else {
            let _ = write!(line, "{LAB}shaking the thermometer…{END}");
        }
        hud.push(line + EOL);

        // Battery and power: what goes in, what gets burnt. It has feelings
        // about the cable, and they are not entirely wholesome.
        let batt = self.battery.as_str();
        let draining = batt.contains("discharging");
        let fluids: [&str; 4] = if draining {
            ["unplugged and unsatisfied", "living off its reserves", "misses the cable already", "going it alone tonight"]
        } else if batt.contains("charged") || batt.contains("AC") {
            [
                "full, and still will not pull out",
                "satisfied. keeps it in anyway.",
                "topped up, clinging to the cable",
                "full, and still asking for more",
            ]
        } else if batt.contains("charg") {
            [
                "plugged in and loving every watt",
                "taking it straight from the wall",
                "being filled up, do not disturb",
                "humming happily at 60 Hz",
            ]
        } else {
            [""; 4]
        };
        let mut note = pick(&fluids, epoch / 20);
        let pct: String = batt.chars().take_while(|ch| ch.is_ascii_digit()).collect();
        if draining && batt[pct.len()..].starts_with('%') && pct.parse::<i64>().map(|p| p < 20).unwrap_or(false) {
            note = "running on fumes, begging for it";
        }
        let level = batt.split(' ').next().filter(|w| !w.is_empty()).unwrap_or("n/a");
        let mut line = format!("  {LAB}FLUIDS  {END} battery \x1b[1m{level}{END}");
        if !note.is_empty() {
            let _ = write!(line, "{LAB}, {note}{END}");
        }
        if let Some(watts) = &self.sys_w {
            let w: f64 = watts.parse().unwrap_or(0.0);
            let note = if w < 8.0 {
                "sipping"
            } else if w < 20.0 {
                "drinking steadily"
            } else if w < 40.0 {
                "guzzling"
            } else {
                "chugging straight from the wall"
            };
            let _ = write!(line, " {LAB}·{END} {watts} W{LAB}, {note}{END}");
        }
        hud.push(line + EOL);

        if let (Some(total), Some(used)) = (self.ram_total, self.ram_used) {
            let mut line = format!("  {LAB}MEMORY  {END} \x1b[1m{}{END} free of {}", gib(total - used), gib(total));
            let pressure = self.mem_pressure.unwrap_or(0);
            let note = if !self.hyper {
                "mind like water"
            } else if pressure < 40 {
                "remembers everything, unfortunately"
            } else if pressure < 70 {
                "starting to forget names"
            } else {
                "who are you people"
            };
            if let Some(p) = self.mem_pressure {
                let _ = write!(line, " {LAB}·{END} pressure {p}%");
            }
            let _ = write!(line, "{LAB}, {note}{END}{EOL}");
            hud.push(line);
        } else {
            hud.push(format!("  {LAB}MEMORY   cannot recall{END}{EOL}"));
        }

        let mut line = format!("  {LAB}EYES    {END} ");
        if let (Some(lid), Some(screen)) = (self.lid_state, self.screen_state) {
            let state = |on: bool, yes: &str, no: &str| {
                if on {
                    format!("\x1b[1;38;5;46m{yes}{END}")
                } else {
                    format!("\x1b[1;38;5;208m{no}{END}")
                }
            };
            let _ = write!(
                line,
                "lid {} {LAB}{}{END} {LAB}·{END} screen {} {LAB}{}{END}",
                state(lid, "OPEN", "CLOSED"),
                duration(epoch - self.lid_since),
                state(screen, "ON", "OFF"),
                duration(epoch - self.screen_since)
            );
            if !lid && self.perv {
                let _ = write!(line, "{LAB}, wide awake in the dark{END}");
            } else if !screen {
                let _ = write!(line, "{LAB}, only pretending to sleep{END}");
            }
        } else {
            let _ = write!(line, "{LAB}checking the lid…{END}");
        }
        hud.push(line + EOL);
        hud.push(rule);

        // Rx: what each prescription really does to sleep. Where the window
        // is too small for the titles, these lines also carry the selection.
        for i in 1..=2 {
            let (name, on, color, mut effect) = if i == 1 {
                ("PERVETIN", self.perv, 226, "ϟ awake even with the lid closed".to_string())
            } else {
                ("CAFFEINE", self.caf, 172, "screen stays on, no idle sleep".to_string())
            };
            if i == 1 && !self.caf {
                let _ = write!(effect, "{LAB} · the screen may still turn off");
            }
            if i == 2 && !self.perv {
                let _ = write!(effect, "{LAB} · closing the lid still sleeps");
            }
            let mut line = if i == 1 { format!("  {LAB}Rx      {END} ") } else { spaces(11) };
            let (mut chip, dot) =
                if on { (format!("\x1b[1;38;5;{color}m"), '◉') } else { ("\x1b[38;5;242m".to_string(), '○') };
            if !on {
                effect = format!("{DIM}not taken");
            }
            if !self.show_banner && i == self.sel {
                chip.push_str("\x1b[7m");
            }
            let _ = write!(line, "{chip}{dot} {name}{END}  \x1b[38;5;{color}m{effect}{END}{EOL}");
            hud.push(line);
        }

        // Dx: the mood, and what the doctor makes of it.
        let watching;
        let dx = match self.stage {
            "sober" => self.zen_dx,
            "buzzed" => "mild caffeination, unusually chatty",
            "jitters" => "caffeine tremor, cannot hold a cursor still",
            "wired" => "acute wakefulness, prognosis: awake",
            "bloodshot" => "has not blinked since admission",
            "paranoid" => {
                watching = format!("convinced {} is watching it", self.top_proc);
                watching.as_str()
            }
            "chatty" => "logorrhoea. do not mention Vim.",
            "productive" => "delusions of productivity",
            "grinding" => "bruxism, jaw clocked at 4.2 GHz",
            "invincible" => "delusions of grandeur, thinks it is a Mac Pro",
            "itchy" => "formication. the bugs are, regrettably, real.",
            "glitch" => "kernel panic of the soul",
            "matrix" => "took the red pill. and the other pill.",
            "bsod" => "a Windows illness. on a Mac. baffling.",
            "hal" => "fully operational, which is the worrying part",
            "vaporwave" => "nostalgia for a decade it never ran in",
            "arcade" => "thinks this is a game. the ghosts disagree.",
            "disco" => "believes it is in a club. it is on a desk.",
            "tripping" => "seeing things that are not in the framebuffer",
            "transcendent" => "has left the chassis. vitals optional.",
            _ => "unclear. keep under observation.",
        };
        let mut line = format!("  {LAB}Dx      {END} \x1b[1m{stage_name}{END} {LAB}·{END} {dx}");
        if self.hyper && self.top_pcpu >= 10 && self.stage != "paranoid" {
            let _ = write!(line, "{LAB} · {} is eating {}% of a core{END}", self.top_proc, self.top_pcpu);
        }
        hud.push(line + EOL);
        if self.dry_run {
            hud.push(format!("  \x1b[38;5;208m(dry run: pmset and caffeinate untouched){END}{EOL}"));
        }
        hud.push(EOL.to_string());
        hud.push(format!("  \x1b[1;38;5;{c2}m» {} «{END}{EOL}", self.tagline));
        hud.push(EOL.to_string());
        hud
    }

    /// The whole frame, ready to write.
    pub fn compose(&mut self) -> String {
        let rows = self.term_rows.max(1) as usize;
        let pad = spaces(4 + self.jitter);
        let stage_name = if self.hyper { self.stage } else { self.zen_mood }.to_uppercase();

        let mut left = std::mem::take(&mut self.left);
        let mut right = std::mem::take(&mut self.right);
        for column in [&mut left, &mut right] {
            column.clear();
            column.resize(rows, String::new());
        }
        if let (true, Some((pw, cw))) = (self.show_banner, self.title_w) {
            put(&mut left, self.oy, self.titles(pw, cw));
        }
        put(&mut left, self.lap_row0, self.laptop(&pad));

        // The chart goes to the right of the laptop when there is room, else
        // below.
        let mut hud = self.chart(&stage_name);
        let mut evlog = Vec::new();
        if !self.events.is_empty() {
            evlog.push(format!("  \x1b[38;5;245mevents{END}{EOL}"));
            let from = self.events.len().saturating_sub(EVENTS_SHOWN);
            evlog.extend(self.events[from..].iter().map(|ev| format!("{ev}{EOL}")));
            evlog.push(EOL.to_string());
        }
        let mute_hint = if self.muted { "m unmute" } else { "m mute" };
        let mut foot = vec![format!(
            "  {DIM}←/→ switch · ↑/↓ mood · space start/stop · {mute_hint} · Ctrl+C to come down gently{END}{EOL}"
        )];
        if self.perv {
            foot.push(format!("  {DIM}keep it out of a closed bag, it gets hot{END}{EOL}"));
        }
        if self.side {
            // Side by side: stats in the right column, event log under the laptop.
            hud.extend(foot);
            put(&mut right, self.lap_row0, hud);
            put(&mut left, self.lap_row0 + LAP_H + 1, evlog);
        } else {
            let mut at = self.lap_row0 + LAP_H + 1;
            if self.compact {
                // Squeeze out the blank lines to fit more stats.
                hud.retain(|line| line != EOL);
                at -= 1;
            }
            hud.extend(evlog);
            hud.extend(foot);
            put(&mut left, at, hud);
        }

        // Every row of the window is accounted for, so nothing stale
        // survives a reflow, but only the rows that changed are sent: unless
        // something was drawn over them, in which case all of them are.
        let margin = spaces(self.ox);
        let over = !self.overlay.is_empty() || !self.fluid.is_empty() || self.pill_on || self.cup_on;
        let all = over || self.painted_over || self.painted.len() != rows;
        let mut out = String::with_capacity(16384);
        let mut painted = std::mem::take(&mut self.painted);
        painted.resize(rows, String::new());
        let mut row = String::new();
        for r in 0..rows {
            row.clear();
            let _ = write!(row, "\x1b[{};1H{margin}{}{EOL}", r + 1, left[r]);
            if !right[r].is_empty() {
                let _ = write!(row, "\x1b[{};{}H{}", r + 1, self.ox + LAP_W + 1, right[r]);
            }
            if all || painted[r] != row {
                out.push_str(&row);
                std::mem::swap(&mut painted[r], &mut row);
            }
        }
        self.painted = painted;
        self.painted_over = over;
        self.left = left;
        self.right = right;

        out.push_str(&self.overlay);
        out.push_str(&self.fluid);
        if self.pill_on {
            self.render_pill(&mut out);
        }
        if self.cup_on {
            self.render_cup(&mut out);
        }
        // The heartbeat also twitches in the window title.
        let title = format!("♥ {} BPM · {}", self.bpm as i64, stage_name);
        if title != self.title {
            let _ = write!(out, "\x1b]0;{title}\x07");
            self.title = title;
        }
        out
    }
}
