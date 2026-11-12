//! Things drawn over the finished frame (sprites, creatures, rain, the
//! giant pill and cup), the heart and brain monitors, and the sound.
use std::fmt::Write as _;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use crate::art::*;
use crate::face::NBSP;
use crate::sys::{self, rnd};
use crate::{App, Creature};

const ECG_SPEEDS: [f64; 3] = [10.0, 20.0, 34.0]; // columns per second, by doses
const EEG_STEP: [f64; 3] = [0.35, 0.7, 1.4]; // radians per column, by doses
const ECG_BEAT: [f64; 7] = [2.0, 1.0, 8.0, 4.0, 1.0, 3.0, 3.0]; // one beat at full load

/// PERVETIN in the banner's own font, solid blocks only.
fn pill_mask() -> &'static Vec<Vec<bool>> {
    static MASK: OnceLock<Vec<Vec<bool>>> = OnceLock::new();
    MASK.get_or_init(|| BANNER_PERV.iter().map(|row| row.chars().map(|ch| ch == '█').collect()).collect())
}

/// One cell of two half-block pixels (-1 = nothing there).
fn half_blocks(line: &mut String, top: i32, bot: i32) {
    let _ = match (top >= 0, bot >= 0) {
        (true, true) => write!(line, "\x1b[38;5;{top};48;5;{bot}m▀"),
        (true, false) => write!(line, "\x1b[38;5;{top};49m▀"),
        (false, true) => write!(line, "\x1b[38;5;{bot};49m▄"),
        _ => write!(line, "\x1b[C"),
    };
}

fn cache_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
    });
    base.join("pervetin")
}

/// The heartbeat as an 8-bit mono WAV, three seconds long, speeding up or
/// slowing down. Each pulse is a low lub and a higher dub (decaying sines),
/// and every other pulse has a third, gallop beat, which is what makes it
/// sound wrong.
fn heartbeat(speeding_up: bool) -> Vec<u8> {
    let (rate, n) = (8000u32, 24000u32);
    let mut gaps = [0.60, 0.52, 0.44, 0.37, 0.31, 0.26, 0.22, 0.19];
    if !speeding_up {
        gaps.reverse();
    }
    let tau = std::f64::consts::TAU;
    let mut wav = Vec::with_capacity(44 + n as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + n).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&[1, 0, 1, 0]);
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&[1, 0, 8, 0]);
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&n.to_le_bytes());
    let (mut b, mut tb, mut gap) = (0usize, 0.04, gaps[0]);
    for i in 0..n {
        let t = i as f64 / rate as f64;
        if b + 1 < gaps.len() && t >= tb + gap {
            tb += gap;
            b += 1;
            gap = gaps[b];
        }
        let dt = t - tb;
        let mut s = 0.0;
        if dt >= 0.0 {
            if dt < 0.25 {
                s = ((tau * 62.0 * dt).sin() + 0.6 * (tau * 124.0 * dt).sin()) * (-dt * 26.0).exp();
            }
            let d = dt - gap * 0.36;
            if (0.0..0.2).contains(&d) {
                s += 0.7 * ((tau * 84.0 * d).sin() + 0.6 * (tau * 168.0 * d).sin()) * (-d * 34.0).exp();
            }
            let d = dt - gap * 0.68;
            if b % 2 == 1 && (0.0..0.15).contains(&d) {
                s += 0.5 * (tau * 110.0 * d).sin() * (-d * 40.0).exp();
            }
        }
        wav.push((128.0 + 75.0 * s).clamp(0.0, 255.0) as u8);
    }
    wav
}

fn run(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

impl App {
    // --- Sound -------------------------------------------------------------
    // Closing the lid gets a spoken notice and three seconds of
    // accelerating, galloping heartbeat; opening it gets the reverse.

    /// Made once, in the background, and kept in the cache.
    pub fn make_heartbeats(&self) {
        std::thread::spawn(|| {
            let dir = cache_dir();
            for (name, up) in [("heartbeat-up.wav", true), ("heartbeat-down.wav", false)] {
                let file = dir.join(name);
                if std::fs::metadata(&file).map(|m| m.len() > 0).unwrap_or(false) {
                    continue;
                }
                let tmp = dir.join(format!("{name}.{}", std::process::id()));
                if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&tmp, heartbeat(up)).is_ok() {
                    let _ = std::fs::rename(&tmp, &file);
                }
            }
        });
    }

    /// `what` = closed | opened | dozed
    pub fn sound(&self, what: &'static str) {
        if self.muted {
            return;
        }
        std::thread::spawn(move || {
            let wav = |name: &str| cache_dir().join(name).to_string_lossy().into_owned();
            match what {
                "closed" => {
                    run("/usr/bin/say", &["Lid closed. I am staying awake."]);
                    run("/usr/bin/afplay", &[&wav("heartbeat-up.wav")]);
                }
                "opened" => {
                    run("/usr/bin/afplay", &[&wav("heartbeat-down.wav")]);
                    let line = "Welcome back. I saw everything.";
                    if !run("/usr/bin/say", &["-v", "Whisper", line]) {
                        run("/usr/bin/say", &[line]);
                    }
                }
                _ => {
                    run("/usr/bin/say", &["Sorry. I must have dozed off."]);
                }
            }
        });
    }

    // --- Monitors ----------------------------------------------------------

    /// CPU = heart: an ECG that scrolls in real time (independent of frame
    /// rate) and draws one P-QRS-T complex per beat. GPU = brain: an EEG
    /// wave. How fast both go is set by how many substances have kicked in
    /// (none: slow, one: quick, both: frantic), nudged by load and heat; how
    /// TALL they are is the real load.
    pub fn update_monitors(&mut self) {
        let doses = self.vis_perv as usize + self.vis_caf as usize;
        let load = self.cpu_load.unwrap_or(30) as f64;
        let hot = (self.cpu_t.unwrap_or(0) - 45).max(0) as f64;
        let target = match doses {
            0 => 46.0 + load * 0.25,
            1 => 92.0 + load * 0.7 + hot,
            _ => 150.0 + load * 1.5 + hot * 2.0,
        };
        // Each mood scales and shifts the pulse.
        let target = (target * self.bpm_scale + self.bpm_bias).clamp(8.0, 300.0);
        self.bpm += (target - self.bpm) * 0.08;

        let now = sys::now();
        self.ecg_speed += (ECG_SPEEDS[doses] - self.ecg_speed) * 0.08;
        self.ecg_due += (now - self.ecg_clock) * self.ecg_speed;
        self.ecg_clock = now;
        let n = (self.ecg_due as usize).min(self.ecg_w);
        self.ecg_due -= n as f64;
        // Square roots, so that an ordinary few percent of load is still visible.
        let amp = 1.0 + 6.0 * (self.gpu_load.unwrap_or(10) as f64 / 100.0).sqrt();
        for _ in 0..n {
            self.eeg_phase += EEG_STEP[doses] * (0.8 + rnd(40) as f64 / 100.0);
            let lvl = (1.0 + amp * (0.5 + 0.5 * self.eeg_phase.sin()) + rnd(doses as i64 + 1) as f64 * 0.6) as usize;
            self.eeg_trace.pop_front();
            self.eeg_trace.push_back(EEG_LEVELS[lvl.clamp(1, 8) - 1]);
            if self.ecg_until_beat <= 0.0 {
                let k = 0.3 + 0.7 * (load / 100.0).sqrt();
                self.ecg_pending =
                    ECG_BEAT.iter().map(|l| EEG_LEVELS[((1.0 + (l - 1.0) * k + 0.5) as usize).clamp(1, 8) - 1]).collect();
                // A little beat-to-beat variability so it never looks like a metronome.
                self.ecg_until_beat = 60.0 / self.bpm * self.ecg_speed * (0.9 + rnd(20) as f64 / 100.0);
                self.last_beat = now;
            }
            self.ecg_until_beat -= 1.0;
            self.ecg_trace.pop_front();
            self.ecg_trace.push_back(self.ecg_pending.pop_front().unwrap_or('▁'));
        }
    }

    // --- Overlays ----------------------------------------------------------

    /// The columns [from, to) of 0-based row `r` that hold text and stay
    /// clear of decorations: the chart, out to the window's edge since its
    /// lines run long, and the event log. The titles and the laptop are fair
    /// game.
    fn text_span(&self, r: i64) -> (i64, i64) {
        if r < self.lap_row0 || r >= self.box_r1 {
            (0, 0)
        } else if r >= self.text_r0 {
            (self.box_c0, if self.side { self.box_c1 } else { self.term_cols })
        } else if self.side {
            (self.text_c0, self.term_cols)
        } else {
            (0, 0)
        }
    }

    /// Draw a sprite (rows split by newlines) with its top-left at column
    /// x, row y (0-based). Plain spaces are transparent.
    pub fn draw_sprite(&mut self, x: i64, y: i64, color: i32, text: &str) {
        for (i, line) in text.split('\n').enumerate() {
            let r = y + i as i64;
            if r < 0 || r >= self.term_rows {
                continue;
            }
            let mut row: Vec<char> = line.chars().collect();
            let mut col = x;
            if x < 0 {
                row.drain(..((-x) as usize).min(row.len()));
                col = 0;
            }
            if col >= self.term_cols {
                continue;
            }
            // Keep out of the text: the stats (right of the laptop, or
            // everything below it when stacked) and the event log. What
            // would overlap is cut, so things pass behind the text rather
            // than over it.
            let (t0, t1) = self.text_span(r);
            if t0 < t1 {
                if col < t0 {
                    row.truncate((t0 - col) as usize);
                } else if col < t1 {
                    row.drain(..((t1 - col) as usize).min(row.len()));
                    col = t1;
                }
            }
            row.truncate((self.term_cols - col).max(0) as usize);
            if row.is_empty() {
                continue;
            }
            let _ = write!(self.overlay, "\x1b[{};{}H\x1b[38;5;{}m", r + 1, col + 1, color);
            for ch in row {
                match ch {
                    ' ' => self.overlay.push_str("\x1b[C"),
                    NBSP => self.overlay.push(' '),
                    _ => self.overlay.push(ch),
                }
            }
            self.overlay.push_str("\x1b[0m");
        }
    }

    /// Digital rain, around the laptop's stats but never over them: down
    /// the margins, and over the titles and the laptop.
    pub fn matrix_rain(&mut self) {
        const SHADE: [i32; 6] = [231, 46, 40, 34, 28, 22];
        let (rows, cols) = (self.term_rows, self.term_cols);
        while (self.rain.len() as i64) < cols / 3 {
            self.rain.push((rnd(cols), -(rnd(rows) as f64), 0.4 + rnd(10) as f64 / 10.0));
        }
        for i in 0..self.rain.len() {
            let (x, mut y, v) = self.rain[i];
            y += v;
            if y - 6.0 > rows as f64 {
                self.rain[i] = (rnd(cols), -(rnd(8) as f64), v);
                continue;
            }
            self.rain[i].1 = y;
            for (k, shade) in SHADE.iter().enumerate() {
                let r = y as i64 - k as i64;
                if r < 0 || r >= rows {
                    continue;
                }
                let (t0, t1) = self.text_span(r);
                if x >= t0 - 1 && x < t1 {
                    continue;
                }
                let glyph = GLYPHS[rnd(GLYPHS.len() as i64) as usize];
                let _ = write!(self.overlay, "\x1b[{};{}H\x1b[38;5;{}m{}", r + 1, x + 1, shade, glyph);
            }
        }
        self.overlay.push_str("\x1b[0m");
    }

    /// Hallucinations: fish swim across, ghosts rise, spiders abseil, eyes
    /// blink in the dark and, inevitably, pink elephants walk by.
    pub fn spawn_creature(&mut self) {
        const KINDS: [&str; 6] = ["fish", "fish", "eyeball", "ghost", "spider", "elephant"];
        let (rows, cols) = (self.term_rows, self.term_cols);
        let kind = KINDS[rnd(6) as usize];
        let (x, y, v) = match kind {
            "fish" => {
                let v = 0.7 + rnd(12) as f64 / 10.0;
                if rnd(2) == 1 {
                    (cols as f64, rnd(rows) as f64, -v)
                } else {
                    (-7.0, rnd(rows) as f64, v)
                }
            }
            "eyeball" => (rnd(cols) as f64, rnd(rows) as f64, 0.0),
            "ghost" => (rnd(cols) as f64, rows as f64, -0.35),
            "spider" => (rnd(cols) as f64, -1.0, 0.6),
            _ => (cols as f64, rnd(if rows > 4 { rows - 3 } else { 1 }) as f64, -0.5),
        };
        self.creatures.push(Creature { kind, x, y, v, age: 0 });
    }

    pub fn update_creatures(&mut self) {
        let (rows, cols) = (self.term_rows as f64, self.term_cols as f64);
        let mut all = std::mem::take(&mut self.creatures);
        // Newest first, so the oldest are drawn on top.
        for cr in all.iter_mut().rev() {
            cr.age += 1;
            let age = cr.age;
            match cr.kind {
                "fish" => {
                    cr.x += cr.v;
                    if cr.v > 0.0 {
                        self.draw_sprite(cr.x as i64, cr.y as i64, 45, "><(((°>");
                    } else {
                        self.draw_sprite(cr.x as i64, cr.y as i64, 214, "<°)))><");
                    }
                }
                "eyeball" => {
                    self.draw_sprite(cr.x as i64, cr.y as i64, 231, if age % 12 < 2 { "(-)" } else { "(◉)" });
                    if age > 45 {
                        cr.x = -99.0;
                    }
                }
                "ghost" => {
                    cr.y += cr.v;
                    cr.x += (age as f64 / 4.0).sin() * 0.6;
                    let skirt = if age / 3 % 2 == 1 { "'~^~'" } else { "'^~^'" };
                    self.draw_sprite(cr.x as i64, cr.y as i64, 255, &format!(" .-.\n(o o)\n| O |\n{skirt}"));
                }
                "spider" => {
                    // Abseils to about mid-screen, dangles, then climbs back up.
                    if cr.v > 0.0 && cr.y > rows * 0.55 {
                        cr.v = 0.0;
                    }
                    if cr.v == 0.0 && age % 30 == 0 {
                        cr.v = -0.6;
                    }
                    cr.y += cr.v;
                    for r in 0..cr.y as i64 {
                        self.draw_sprite(cr.x as i64 + 3, r, 240, "│");
                    }
                    let legs = if age % 2 == 1 { r"/\(oo)/\" } else { r"\/(oo)\/" };
                    self.draw_sprite(cr.x as i64, cr.y as i64, 196, legs);
                }
                _ => {
                    cr.x += cr.v;
                    self.draw_sprite(cr.x as i64, cr.y as i64, 213, "   .-\"\"-.__\n _/  o     )~\n(_,/||---||");
                }
            }
        }
        all.retain(|cr| !(cr.x < -14.0 || cr.x > cols + 2.0 || cr.y < -5.0 || cr.y > rows + 1.0));
        self.creatures = all;
    }

    /// Drips off the melting banner.
    pub fn update_drips(&mut self) {
        let mut all = std::mem::take(&mut self.drips);
        for drip in all.iter_mut() {
            drip.1 += 0.6;
        }
        all.retain(|drip| drip.1 <= self.term_rows as f64);
        for (i, (x, y)) in all.iter().enumerate().rev() {
            let color = HOT[(i as i64 + 1 + self.frame).rem_euclid(HOT.len() as i64) as usize];
            self.draw_sprite(*x, *y as i64 - 1, color, "░\n▓");
        }
        self.drips = all;
    }

    /// A stream of coffee: a quadratic Bézier from `a` past the control
    /// point `b` to `e` (cells), drawn between the fractions `from` and `to`
    /// of its length, `w` cells wide. The colours run along it.
    pub fn stream(&mut self, a: (f64, f64), b: (f64, f64), e: (f64, f64), from: f64, to: f64, w: i64) {
        let n = (2.0 * ((b.0 - a.0).abs() + (e.0 - b.0).abs() + (b.1 - a.1).abs() + (e.1 - b.1).abs())) as i64 + 2;
        let blank = " ".repeat(w as usize);
        let mut last = (-1, -1);
        for i in (from * n as f64) as i64..=(to * n as f64) as i64 {
            let p = i as f64 / n as f64;
            let q = 1.0 - p;
            let x = q * q * a.0 + 2.0 * q * p * b.0 + p * p * e.0;
            let y = q * q * a.1 + 2.0 * q * p * b.1 + p * p * e.1;
            let (r, c0) = (y.floor() as i64, (x - w as f64 / 2.0 + 0.5).floor() as i64);
            if (r, c0) == last {
                continue;
            }
            last = (r, c0);
            if r < 0 || r >= self.term_rows || c0 < 0 || c0 + w > self.term_cols {
                continue;
            }
            let color = BREW[(i - self.frame).rem_euclid(BREW.len() as i64) as usize];
            let _ = write!(self.fluid, "\x1b[{};{}H\x1b[48;5;{}m{}\x1b[0m", r + 1, c0 + 1, color, blank);
        }
    }

    // --- The giant pill ----------------------------------------------------

    /// An old-school two-tone capsule drawn with half blocks (▀/▄), so each
    /// terminal cell holds two square "pixels". It is lit from the top left
    /// and spins about its long axis with PERVETIN printed around it.
    /// Appended to `out` as cursor-addressed spans: centre column,row
    /// (cells), radius in pixels (= height in rows), spin in radians.
    pub fn render_pill(&self, out: &mut String) {
        let (cx, cyp, rot) = (self.pill_x, self.pill_y * 2.0, self.pill_rot);
        let rad = self.pill_r.max(1.0);
        let (lh, text_w, band, pi) = (rad * 1.5, rad * 4.5, 0.55, std::f64::consts::PI);
        let mask = pill_mask();
        let (mh, mw) = (mask.len() as i64, mask[0].len() as i64);
        let text = rad * 4.5 >= 24.0;
        let r0 = ((cyp - rad) / 2.0).floor() as i64;
        let r1 = ((cyp + rad) / 2.0).floor() as i64;
        for cr in r0..=r1 {
            if cr < 0 || cr >= self.term_rows {
                continue;
            }
            // Per pixel row (top, bottom): chord half-width, height on the
            // cylinder, and which text row is facing us after the spin.
            let mut xr = [-1.0f64; 2];
            let mut vs = [0.0f64; 2];
            let mut trs = [0i64; 2];
            for k in 0..2 {
                let dy = 2.0 * cr as f64 + k as f64 + 0.5 - cyp;
                if dy.abs() < rad {
                    let v = -dy / rad;
                    let mut u = v.asin() + rot;
                    u -= pi * (u / pi + 0.5).floor();
                    let mut tr = 0;
                    if text && u.abs() < band {
                        tr = (((band - u) / (2.0 * band) * mh as f64) as i64 + 1).min(mh);
                    }
                    xr[k] = lh + (rad * rad - dy * dy).sqrt();
                    vs[k] = v;
                    trs[k] = tr;
                }
            }
            let xmax = xr[0].max(xr[1]);
            if xmax < 0.0 {
                continue;
            }
            let c0 = ((cx - xmax).floor() as i64).max(0);
            let c1 = ((cx + xmax).ceil() as i64 - 1).min(self.term_cols - 1);
            if c0 > c1 {
                continue;
            }
            let _ = write!(out, "\x1b[{};{}H", cr + 1, c0 + 1);
            for c in c0..=c1 {
                let ax = c as f64 + 0.5 - cx;
                let mut px = [-1i32; 2];
                for k in 0..2 {
                    if ax.abs() > xr[k] {
                        continue;
                    }
                    let (v, tr) = (vs[k], trs[k]);
                    let d = ax.abs() - lh;
                    let nx = if d > 0.0 { d.copysign(ax) / rad } else { 0.0 };
                    let nz2 = (1.0 - nx * nx - v * v).max(0.0);
                    let dif = (-0.35 * nx + 0.6 * v + 0.72 * nz2.sqrt()).max(0.0);
                    let idx = (((0.2 + 0.8 * dif) * 5.999) as usize).min(5);
                    let mut on = false;
                    if tr > 0 {
                        let tc = ((ax / text_w + 0.5) * mw as f64) as i64 + 1;
                        on = tc >= 1 && tc <= mw && mask[tr as usize - 1][tc as usize - 1];
                    }
                    px[k] = if (ax < 0.0) != on { PILL_RED[idx] } else { PILL_WHITE[idx] };
                }
                half_blocks(out, px[0], px[1]);
            }
            out.push_str("\x1b[0m");
        }
    }

    // --- The giant coffee cup ----------------------------------------------

    /// Caffeine's answer to the pill, in the same half-block pixels: a
    /// white mug with a coffee-coloured band, a handle and a dark surface at
    /// the rim. Centre column,row (cells), scale in pixels (it is 1.5 scales
    /// tall), tilt in radians (negative tips the rim to the left).
    pub fn render_cup(&self, out: &mut String) {
        let (cx, cyp, rot) = (self.pill_x, self.pill_y * 2.0, self.pill_rot);
        let s = self.pill_r.max(2.0);
        let (cs, sn) = (rot.cos(), rot.sin());
        let r0 = ((cyp - 1.1 * s) / 2.0).floor() as i64;
        let r1 = ((cyp + 1.1 * s) / 2.0).floor() as i64;
        let c0 = ((cx - 1.1 * s).floor() as i64).max(0);
        let c1 = ((cx + 1.1 * s).ceil() as i64).min(self.term_cols - 1);
        if c0 > c1 {
            return;
        }
        for cr in r0..=r1 {
            if cr < 0 || cr >= self.term_rows {
                continue;
            }
            let _ = write!(out, "\x1b[{};{}H", cr + 1, c0 + 1);
            for c in c0..=c1 {
                let px = c as f64 + 0.5 - cx;
                let mut cell = [-1i32; 2];
                for k in 0..2 {
                    let py = 2.0 * cr as f64 + k as f64 + 0.5 - cyp;
                    let u = (px * cs + py * sn) / s;
                    let v = (py * cs - px * sn) / s;
                    if v.abs() >= 0.75 {
                        continue;
                    }
                    let hw = 0.62 - 0.055 * (v + 0.75); // tapers towards the base
                    cell[k] = if u.abs() < hw {
                        let n = u / hw;
                        // A cylinder lit from the top left, like the pill.
                        let dif = (-0.35 * n * cs - 0.6 * n * sn + 0.72 * (1.0 - n * n).sqrt()).max(0.0);
                        let idx = (((0.2 + 0.8 * dif) * 5.999) as usize).min(5);
                        if s >= 4.0 && (u / (hw - 0.1)).powi(2) + ((v + 0.6) / 0.1).powi(2) < 1.0 {
                            94 // the coffee
                        } else if v > 0.05 && v < 0.42 {
                            CUP_PAL[idx]
                        } else {
                            PILL_WHITE[idx]
                        }
                    } else {
                        // The handle: a ring on the right-hand side.
                        let hr = ((u - 0.6).powi(2) + v * v).sqrt();
                        if !(u > 0.0 && hr > 0.24 && hr < 0.44) {
                            continue;
                        }
                        250
                    };
                }
                half_blocks(out, cell[0], cell[1]);
            }
            out.push_str("\x1b[0m");
        }
    }
}
