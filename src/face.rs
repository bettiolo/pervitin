//! The face: the zen moods of a sober machine, the moods of a dosed one,
//! the cutscenes, and what happens when it is touched.
use crate::art::*;
use crate::sys::{self, rnd};
use crate::{clen, duration, first, pick, rep, rot, spaces, App, Scene, LAP_H, ZEN_MOODS};

const ETOP: &str = ".----.     .----.";
const EBOT: &str = "'----'     '----'";
/// In a sprite: a blank that hides what is behind it.
pub const NBSP: char = '\u{a0}';
/// What is left of the pill once it is in the mouth.
const PILL: &str = "(-)";
/// How long the pill is in the air.
const SWOOP: f64 = 1.5;

/// An eye with `pupil` darting to column `pos` (0-3).
fn eye(pupil: &str, pos: i64) -> String {
    format!("|{}{}{}|", spaces(pos), pupil, spaces(3 - pos))
}

fn pair(e: &str) -> String {
    format!("{e}     {e}")
}

fn fg(color: i32) -> String {
    format!("38;5;{color}")
}

/// Right-padded (or cut) to the width of the screen.
fn wide(s: &str) -> String {
    let mut out = first(s, 26);
    out.push_str(&spaces(26 - clen(&out)));
    out
}

/// Cubic Bézier through a, b, c, d at u in [0,1].
fn bez(a: f64, b: f64, c: f64, d: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    v * v * v * a + 3.0 * v * v * u * b + 3.0 * v * u * u * c + u * u * u * d
}

fn zen_pal(mood: &str) -> &'static [i32] {
    match mood {
        "cat" | "blanket" | "sunbeam" => WARM_PAL,
        "stargazing" => NIGHT_PAL,
        "smitten" => ROSE_PAL,
        _ => ZEN_PAL,
    }
}

impl App {
    /// c, c2 at random from the mood's palette.
    fn colors(&mut self) {
        self.c = self.pal[rnd(self.pal.len() as i64) as usize];
        self.c2 = self.pal[rnd(self.pal.len() as i64) as usize];
    }

    fn open_eyes(&mut self, e: &str) {
        self.r_etop = ETOP.into();
        self.r_eye = pair(e);
        self.r_ebot = EBOT.into();
    }

    /// Sound effects, spelled out above the lid: the words drift sideways,
    /// `every` frames per step.
    fn sfx(&mut self, text: &str, every: i64) {
        self.steam = format!("{}{}", spaces(self.frame / every % 4), text);
    }

    /// The keyboard is part of the act: key n (1-17) shows `text` (two
    /// cells) in `color` for this frame.
    pub fn key(&mut self, n: i64, text: &str, color: i32) {
        if (1..=17).contains(&n) {
            self.kb_t[n as usize - 1] = text.to_string();
            self.kb_c[n as usize - 1] = color;
        }
    }

    /// Every key: `text`, colours cycling through `pal` from `offset`.
    fn keys_all(&mut self, text: &str, pal: &[i32], offset: i64) {
        for i in 1..=17 {
            self.key(i, text, pick(pal, i + offset));
        }
    }

    /// While the mouse moves, the eyes follow it. The pupils are Braille
    /// dots, eight across and four down inside the one-row eye, so they
    /// glide rather than jump from cell to cell. Each eye aims on its own,
    /// so they cross when the mouse is between them, and the pupils shrink
    /// as the mouse comes close. Does nothing once the mouse has been still
    /// for a while, so the mood's own eye movement resumes.
    fn track_eyes(&mut self) {
        if self.mouse_x < 0 || sys::now() - self.mouse_at >= 6.0 {
            return;
        }
        let dy = 2.0 * (self.mouse_y as f64 - self.lap_row0 as f64 - 6.5);
        let far = ((self.mouse_x as f64 - self.mouth_x).powi(2) + dy * dy).sqrt();
        let size: i64 = if far < 18.0 {
            1
        } else if far < 45.0 {
            2
        } else {
            3
        };
        let fy = dy / (dy.abs() + 5.0);
        let mut eyes = Vec::new();
        for k in [-1.0, 1.0] {
            let d = self.mouse_x as f64 - (self.mouth_x + 5.5 * k);
            let fx = d / (d.abs() + 4.0);
            let x0 = ((8 - size) as f64 / 2.0 * (1.0 + fx) + 0.5) as i64;
            let y0 = ((4 - size) as f64 / 2.0 * (1.0 + fy) + 0.5) as i64;
            let mut bits = [0u32; 4];
            for y in y0..y0 + size {
                for x in x0..x0 + size {
                    // The bigger pupils are round: no corners.
                    if size > 2 && (x == x0 || x == x0 + size - 1) && (y == y0 || y == y0 + size - 1) {
                        continue;
                    }
                    let cell = (x / 2).clamp(0, 3) as usize;
                    bits[cell] |= if y % 4 == 3 {
                        if x % 2 == 1 {
                            0x80
                        } else {
                            0x40
                        }
                    } else {
                        1 << (y % 4 + (x % 2) * 3)
                    };
                }
            }
            let mut t = String::from("|");
            for b in bits {
                t.push(if b != 0 { char::from_u32(0x2800 + b).unwrap_or(' ') } else { ' ' });
            }
            t.push('|');
            eyes.push(t);
        }
        self.r_etop = ETOP.into();
        self.r_eye = format!("{}     {}", eyes[0], eyes[1]);
        self.r_ebot = EBOT.into();
    }

    // --- Nothing taken -----------------------------------------------------

    /// Nothing taken: a zen garden. It breathes on a ten-second cycle (in,
    /// hold, out, rest), its colour swelling and fading with the breath, and
    /// the chart calms down with it. Within that it drifts between cosy
    /// little moods.
    pub fn face_sober(&mut self) {
        let now = self.now;
        let ph = (now / 10.0).fract();
        let cyc = (now / 10.0) as i64;
        let say = (now / 8.0) as i64;
        let (breath, word) = if ph < 0.4 {
            (ph / 0.4, "breathe in")
        } else if ph < 0.5 {
            (1.0, "hold")
        } else if ph < 0.9 {
            (1.0 - (ph - 0.5) / 0.4, "breathe out")
        } else {
            (0.0, "")
        };
        // Eased, so the colour lingers at the top and bottom of each breath.
        self.breath = breath * breath * (3.0 - 2.0 * breath);
        let breath = self.breath;
        // Drift to another mood every half minute or so: stars only come out
        // at night, sunbeams only by day.
        if now >= self.zen_until {
            let mut pool = vec!["meditate", "cat", "tea", "blanket", "smitten", "humming"];
            let hh = sys::local_hms(now as i64).0;
            let extra = if hh >= 20 || hh < 6 { "stargazing" } else { "sunbeam" };
            pool.extend([extra, extra]);
            let mut next = self.zen_mood;
            while next == self.zen_mood {
                next = pool[rnd(pool.len() as i64) as usize];
            }
            self.zen_mood = next;
            self.zen_until = now + 25.0 + rnd(16) as f64;
        }
        if let Some(forced) = self.forced_stage {
            if ZEN_MOODS.contains(&forced) {
                self.zen_mood = forced;
            }
        }
        let zp = zen_pal(self.zen_mood);
        self.c = zp[(breath * 5.999) as usize];
        let c = self.c;
        self.pal = HOT;
        self.jitter = 0;
        self.c2 = c;
        self.banner_color = Some(238);
        // The face itself stays plain, whatever the frame is doing.
        self.eye_color = "39".into();
        self.sweat_color = "39".into();
        self.m_color = "39".into();
        self.sparks.clear();
        self.steam.clear();
        self.r_brow.clear();
        self.r_sweat.clear();
        // Everyone starts from closed, smiling eyes and a small smile.
        self.r_etop.clear();
        self.r_eye = " ‿‿‿‿       ‿‿‿‿ ".into();
        self.r_ebot.clear();
        self.r_m1.clear();
        self.r_m2 = " ‿‿‿‿‿‿ ".into();
        let half = |secs: f64| (now / secs) as i64 % 2 == 1;
        match self.zen_mood {
            "cat" => {
                // Curled up and purring.
                self.r_brow = r"/\               /\".into();
                self.r_sweat = "≡     ▾     ≡".into();
                self.r_m1 = "ω".into();
                self.r_m2.clear();
                self.steam = format!("{}purrrrrrrr   z  z", spaces(6 + (now / 2.0) as i64 % 2));
                self.tagline = format!("pu{}…", rep('r', 2 + (breath * 7.999) as i64));
                self.zen_dx = "purring. cause unknown. do not treat.";
            }
            "tea" => {
                // A warm cup, definitely decaf, held up in front of the
                // keyboard with both hands and steaming into the face.
                const SAY: [&str; 4] = [
                    "chamomile. no caffeine. promise.",
                    "both hands around the cup",
                    "the steam knows where it is going",
                    "sip. sigh. repeat.",
                ];
                self.r_sweat = " ‿‿‿‿ ".into();
                let (a, b) = if half(1.5) { (") ) )", "( ( (") } else { ("( ( (", ") ) )") };
                self.r_m1 = a.into();
                self.r_m2 = b.into();
                self.m2_color = Some("39".into());
                let (x, y) = (self.ox + 18, self.lap_row0 + 11);
                let b = rep(NBSP, 7);
                let mug = format!(".-------.\n|{b}|-.\n|{b}|{NBSP}|\n|{b}|-'\n'-------'");
                self.draw_sprite(x, y, c, &mug);
                self.draw_sprite(x + 1, y + 1, 178, "~~~~~~~");
                self.tagline = pick(&SAY, say).into();
                self.zen_dx = "chamomile. decaf. the patient insists.";
            }
            "blanket" => {
                // Wrapped in a knitted blanket, only the face showing.
                const SAY: [&str; 4] = [
                    "five more minutes",
                    "the blanket is warm and so am I",
                    "do not move. it is perfect like this.",
                    "wrapped up. not coming out.",
                ];
                self.r_ebot = "///           ///".into();
                self.r_sweat = " ‿‿‿‿‿‿ ".into();
                self.r_m1.clear();
                self.r_m2.clear();
                self.sparks = format!("{}z  z", spaces(12 + (now / 2.0) as i64 % 3));
                // The blanket goes over the lid like a hood, down both
                // sides, and from the chin down it covers the whole laptop.
                let (x, y) = (self.ox, self.lap_row0);
                self.draw_sprite(x + 5, y + 1, c, &format!(".{}.", rep('~', 36)));
                for r in 2..=8 {
                    self.draw_sprite(x + 4, y + r, c, "(");
                    self.draw_sprite(x + 43, y + r, c, ")");
                }
                self.draw_sprite(x + 4, y + 9, c, &format!("\\{}/", rep('~', 38)));
                for r in 10..=14 {
                    let knit = if (r + cyc) % 2 != 0 { "░▒".repeat(20) } else { "▒░".repeat(20) };
                    self.draw_sprite(x + 3, y + r, zp[(r % 3 + 2) as usize], &format!("({knit})"));
                }
                self.draw_sprite(x + 3, y + 15, c, &format!("'{}'", rep('~', 40)));
                self.tagline = pick(&SAY, say).into();
                self.zen_dx = "tucked in. five more minutes prescribed.";
                if half(4.0) {
                    self.steam = "          snug     snug".into();
                }
            }
            "sunbeam" => {
                // A patch of sun on the desk.
                const SAY: [&str; 4] = [
                    "warm like a cat on a windowsill",
                    "photosynthesising (emotionally)",
                    "the sun moved. I moved with it.",
                    "this is the good spot",
                ];
                self.sparks = if breath > 0.5 { r"        ·  \ ☼ /  ·" } else { "           · ☼ ·" }.into();
                self.r_eye = " ◠◠◠◠       ◠◠◠◠ ".into();
                self.r_ebot = "///           ///".into();
                self.tagline = pick(&SAY, say).into();
                self.zen_dx = "basking. vitamin D: plenty.";
                self.steam = format!("{}mmmmm    warm", spaces(8 + (now / 3.0) as i64 % 2));
            }
            "stargazing" => {
                // Eyes open for once, full of stars.
                const SAY: [&str; 4] = [
                    "counting stars. lost count. started again.",
                    "that one is probably a satellite. still pretty.",
                    "so many pixels up there",
                    "the night is quiet and so am I",
                ];
                self.sparks = rot("·     ✦    ·      ✧   ·    ", (now / 1.5) as i64);
                let star = if (now * 2.0) as i64 % 5 == 0 { "✧" } else { "✦" };
                self.open_eyes(&format!("| {star}  |"));
                self.r_m2 = " ‿‿ ".into();
                self.tagline = pick(&SAY, say).into();
                self.zen_dx = "stargazing. pupils dilated, by wonder.";
                if half(3.0) {
                    self.steam = "       twinkle    twinkle".into();
                }
                self.track_eyes(); // it has noticed the cursor
            }
            "smitten" => {
                // It likes you.
                const SAY: [&str; 4] = [
                    "I like it when you type on me",
                    "you have very warm hands",
                    "stay a while?",
                    "you remembered to charge me. you DO care.",
                ];
                self.key(9, "<3", 211);
                self.open_eyes("| ♥  |");
                self.r_sweat = "///           ///".into();
                for i in 0..3 {
                    let y = (now * 1.2 + i as f64 * 2.3).rem_euclid(7.0);
                    let x = self.ox + 6 + i * 14 + (1.5 * (now + i as f64 * 2.0).sin()) as i64;
                    let row = (self.lap_row0 as f64 + 1.0 - y) as i64;
                    self.draw_sprite(x, row, zp[(5 - y as i64 % 3) as usize], "♥");
                }
                self.tagline = pick(&SAY, say).into();
                self.zen_dx = "smitten. it is you. it likes you.";
                if sys::now() - self.last_beat < 0.5 {
                    self.steam = "         ba-dum   ba-dum".into();
                }
            }
            "humming" => {
                // A little tune, slightly off-key, picked out on the keys.
                self.key((now * 3.0) as i64 % 17 + 1, "▄▄", c);
                self.steam = first(&rot("♪      ♫     ♪       ♫    ", (now * 2.0) as i64), 22);
                if half(1.0 / 1.5) {
                    self.r_m2 = " o ".into();
                }
                self.tagline = format!("hmm hmm hm{}~ ♪", rep('m', 1 + (breath * 4.999) as i64));
                self.zen_dx = "humming. off-key. happy.";
            }
            _ => {
                // Incense and counted breaths.
                const SAY: [&str; 5] = [
                    "the fan that does not spin makes no sound",
                    "nothing to compute. nowhere to be.",
                    "be the idle process",
                    "a closed lid is also an answer",
                    "the cache is empty. so is the mind.",
                ];
                self.steam = if cyc % 2 != 0 { "            (  )" } else { "             )  (" }.into();
                if half(2.5) {
                    self.steam.insert(0, ' ');
                }
                self.tagline = if cyc % 3 == 2 {
                    pick(&SAY, cyc / 3).into()
                } else {
                    format!("{}{}", word, rep('.', (breath * 3.999) as i64))
                };
                self.zen_dx = "nothing wrong. suspiciously nothing.";
            }
        }
    }

    // --- Something taken ---------------------------------------------------

    /// The classic hyper look, and the base every other mood starts from.
    pub fn face_wired(&mut self) {
        if rnd(3) == 0 {
            self.key(rnd(17) + 1, "▄▄", 231); // a twitchy keypress
        }
        self.pal = HOT;
        self.colors();
        self.jitter = rnd(4);
        let l = eye(pick(PUPILS, rnd(PUPILS.len() as i64)), rnd(4));
        let r = eye(pick(PUPILS, rnd(PUPILS.len() as i64)), rnd(4));
        self.r_brow = pick(BROWS, self.frame / 3).into();
        self.r_etop = ETOP.into();
        self.r_eye = format!("{l}     {r}");
        self.r_ebot = EBOT.into();
        self.r_sweat = if rnd(2) == 1 { "'                 " } else { "                 '" }.into();
        self.r_m1 = if self.frame % 2 == 1 { r"\▄▀▄▀▄▀▄▀▄▀▄▀▄▀/" } else { r"\▀▄▀▄▀▄▀▄▀▄▀▄▀▄/" }.into();
        self.r_m2 = "'--------------'".into();
        self.sparks.clear();
        for _ in 0..12 {
            self.sparks.push_str(if rnd(3) == 0 { "ϟ " } else { "  " });
        }
        self.tagline = pick(TAGLINES, self.frame / 25).into();
        self.sfx("twitch    twitch     twitch", 2);
    }

    /// The mood of the moment, painted over the wired face.
    pub fn mood(&mut self) {
        let f = self.frame;
        match self.stage {
            "buzzed" => {
                // Caffeine: pleasant, warm, suspiciously functional.
                self.pal = COFFEE;
                self.colors();
                self.jitter = (rnd(8) == 0) as i64;
                self.r_eye = pair(&eye("●", f / 8 % 4));
                if f % 30 < 2 {
                    self.r_eye = "|----|     |----|".into();
                }
                self.r_brow.clear();
                self.r_sweat.clear();
                self.sparks.clear();
                self.r_m1 = r"\____________/".into();
                self.r_m2.clear();
                self.steam = format!("{}( (    ( (    ( (", spaces(f / 3 % 2));
                self.tagline = pick(CAF_TAGLINES, f / 25).into();
                self.bpm_scale = 0.55;
                self.bpm_bias = 10.0;
                if f / 20 % 2 == 1 {
                    self.sfx("sip     sip      aaah", 5);
                }
                self.track_eyes(); // it has noticed the cursor
            }
            "jitters" => {
                // Caffeine: one cup too many. The keys rattle.
                self.key(rnd(17) + 1, "][", 0);
                self.key(rnd(17) + 1, "][", 0);
                self.key(rnd(17) + 1, "▄▄", 214);
                self.pal = COFFEE;
                self.colors();
                self.jitter = rnd(3);
                let l = eye("◉", 1 + rnd(2));
                let r = if f % 4 < 2 { eye("◉", 1) } else { "|-  -|".to_string() };
                self.r_eye = format!("{l}     {r}");
                self.r_brow = "~~~~~     ^^^^^".into();
                self.r_sweat = "'                '".into();
                self.sparks.clear();
                self.r_m1 = if f % 2 == 1 { r"\▄▀▄▀▄▀▄▀/" } else { r"\▀▄▀▄▀▄▀▄/" }.into();
                self.r_m2 = "'--------'".into();
                self.tagline = pick(JITTER_TAGLINES, f / 25).into();
                self.bpm_scale = 0.7;
                self.bpm_bias = 20.0;
                self.sfx("bzzt   bzzzt    bzzt   bzt", 1);
            }
            "chatty" => {
                // Caffeine: will not stop talking, and types as fast as it talks.
                self.key(f % 17 + 1, "▄▄", 214);
                self.key((f + 1) % 17 + 1, "▄▄", 172);
                self.pal = COFFEE;
                self.colors();
                self.jitter = (rnd(6) == 0) as i64;
                self.r_eye = pair(&eye("●", f / 5 % 4));
                self.r_brow = "^^^^^     ^^^^^".into();
                self.r_sweat.clear();
                self.sparks.clear();
                let (m1, m2) = match f % 4 {
                    0 => (".------.", "'------'"),
                    1 => ("", "(------)"),
                    2 => (".--.", "'--'"),
                    _ => ("", "'------'"),
                };
                self.r_m1 = m1.into();
                self.r_m2 = m2.into();
                self.steam = format!("{}bla  bla   bla  bla", spaces(f % 6));
                self.tagline = pick(CHATTY_TAGLINES, f / 14).into();
                self.bpm_scale = 0.6;
                self.bpm_bias = 15.0;
                self.track_eyes(); // it has noticed the cursor
            }
            "productive" => {
                // Caffeine: ticking things off that were never on a list,
                // typing, furiously, on its own keyboard.
                self.key(rnd(17) + 1, "▄▄", 226);
                self.key(rnd(17) + 1, "▄▄", 226);
                self.key(rnd(17) + 1, "▄▄", 220);
                self.pal = COFFEE;
                self.colors();
                self.jitter = 0;
                self.r_eye = "| ✓  |     | ✓  |".into();
                if f % 30 < 2 {
                    self.r_eye = "|----|     |----|".into();
                }
                self.r_brow = "-----     -----".into();
                self.r_sweat.clear();
                self.r_m1 = r"\__________/".into();
                self.r_m2.clear();
                self.sparks = rot("✓   ✓   ✓   ✓   ✓   ✓   ", f / 2 % 4);
                self.tagline = pick(PRODUCTIVE_TAGLINES, f / 25).into();
                self.bpm_scale = 0.65;
                self.bpm_bias = 12.0;
                self.sfx("tap tap   tap  tap tap", 1);
            }
            "grinding" => {
                // Pinned pupils and a jaw with a mind of its own.
                self.pal = BLOOD;
                self.colors();
                self.jitter = rnd(3);
                self.r_eye = "| ·  |     |  · |".into();
                self.r_brow = r"\\\\\     /////".into();
                self.r_sweat = "'                 '".into();
                self.sparks.clear();
                self.r_m1 = if f % 2 == 1 { "|▚▚▚▚▚▚▚▚▚▚▚▚|" } else { "|▞▞▞▞▞▞▞▞▞▞▞▞|" }.into();
                self.r_m2 = "'------------'".into();
                self.steam = format!("{}grrrnd    grrrnd", spaces(f % 3));
                self.tagline = pick(GRIND_TAGLINES, f / 25).into();
                self.bpm_bias = 20.0;
            }
            "invincible" => self.mood_invincible(),
            "itchy" => {
                // Formication: the bugs are real and they are on it.
                self.pal = PALE;
                self.colors();
                self.jitter = rnd(5);
                let (l, r) = (eye("●", rnd(4)), eye("●", rnd(4)));
                self.r_eye = format!("{l}     {r}");
                self.r_brow = "~~~~~     ~~~~~".into();
                self.r_sweat.clear();
                self.sparks.clear();
                self.r_m1 = if f % 2 == 1 { "~~~~~~~~" } else { "≈≈≈≈≈≈≈≈" }.into();
                self.r_m2.clear();
                for i in 1..=8 {
                    let mut x = (f * (i % 3 + 1) + i * 13) % 38;
                    if i % 2 == 1 {
                        x = 37 - x;
                    }
                    let y = self.lap_row0 + 2 + (i * 5 + f / (9 + i)) % 13;
                    self.draw_sprite(self.ox + 5 + x, y, 130, if (f + i) % 2 == 1 { "ж" } else { "x" });
                }
                self.tagline = pick(ITCHY_TAGLINES, f / 25).into();
                self.bpm_bias = 25.0;
                self.sfx("scratch scratch    scratch", 1);
            }
            "glitch" => {
                // Both doses: the face itself starts to corrupt.
                const DEAD: [&str; 6] = ["▓▒", "??", "░░", "#!", "  ", "▚▞"];
                const JUNK: [&str; 7] =
                    ["▓▒░▒▓█▓▒░ ░▒▓", "█▀▄▀█ ▄▀▄ █▄█▀", "░░▒▒▓▓██▓▓▒▒░░", "0xDEADBEEF", "NaN NaN NaN", "▚▞▚▞▚▞▚▞▚▞▚▞", ""];
                const KZ: [&str; 3] = ["kzzt    kz#t     bzzrt", "  k%zt   kzzt   bz__t", "kzzzzzzzzzzzzt"];
                let hot = || HOT[rnd(HOT.len() as i64) as usize];
                for _ in 0..5 {
                    self.key(rnd(17) + 1, DEAD[rnd(6) as usize], hot());
                }
                self.pal = HOT;
                self.colors();
                self.jitter = if rnd(5) == 0 { rnd(12) } else { 0 };
                for row in [
                    &mut self.r_brow,
                    &mut self.r_etop,
                    &mut self.r_eye,
                    &mut self.r_ebot,
                    &mut self.r_sweat,
                    &mut self.r_m1,
                    &mut self.r_m2,
                ] {
                    if rnd(4) == 0 {
                        *row = JUNK[rnd(7) as usize].into();
                    }
                }
                self.sparks = JUNK[rnd(3) as usize].into();
                if rnd(3) == 0 {
                    self.eye_color = fg(hot());
                }
                let mut tl: Vec<char> = pick(GLITCH_TAGLINES, f / 25).chars().collect();
                let n = tl.len() as i64;
                tl[rnd(n) as usize] = '▒';
                if rnd(2) == 1 {
                    tl[rnd(n) as usize] = '█';
                }
                self.tagline = tl.into_iter().collect();
                self.bpm_bias = (rnd(120) - 60) as f64;
                self.cpu_mood_override = Some("arrhythmia");
                self.steam = KZ[rnd(3) as usize].into();
            }
            "matrix" => {
                // Both doses: green rain, and every message has to be decoded.
                let glyph = || GLYPHS[rnd(GLYPHS.len() as i64) as usize];
                for q in 1..=17 {
                    let text: String = [glyph(), glyph()].iter().collect();
                    self.key(q, &text, MATRIX_PAL[rnd(6) as usize]);
                }
                self.c = MATRIX_PAL[rnd(6) as usize];
                self.c2 = 46;
                self.pal = MATRIX_PAL;
                self.jitter = if rnd(20) == 0 { 4 } else { 0 };
                self.eye_color = fg(46);
                self.sweat_color = fg(34);
                self.m_color = fg(40);
                self.r_brow.clear();
                self.r_etop = ETOP.into();
                self.r_eye = format!("| {}  |     |  {} |", glyph(), glyph());
                self.r_ebot = EBOT.into();
                self.r_sweat.clear();
                self.sparks.clear();
                let bits: String = (0..10).map(|_| if rnd(2) == 1 { '1' } else { '0' }).collect();
                self.r_m1 = format!("[{bits}]");
                self.r_m2.clear();
                // The message arrives as noise and resolves from the left.
                let tl = pick(MATRIX_TAGLINES, f / 60);
                let (len, shown) = (clen(tl), (f % 60) * clen(tl) / 22);
                self.tagline = first(tl, shown);
                for _ in shown..len {
                    self.tagline.push(glyph());
                }
                self.matrix_rain();
                self.cpu_mood_override = Some("follow the rabbit");
                self.steam = first(&rot("01 1 0010 110 01 0 1011 001 10 ", f / 2 % 15), 26);
            }
            "bsod" => self.mood_bsod(),
            "hal" => self.mood_hal(),
            "vaporwave" => {
                // Pink and cyan, slowed down, every word spaced out.
                self.keys_all("▀▀", VAPOR_PAL, f / 5);
                self.pal = VAPOR_PAL;
                self.c = pick(VAPOR_PAL, f / 6);
                self.c2 = pick(VAPOR_PAL, f / 6 + 3);
                self.jitter = 0;
                self.eye_color = fg(self.c2);
                self.m_color = fg(self.c2);
                self.r_brow.clear();
                self.open_eyes("|▬▬▬▬|");
                self.r_sweat.clear();
                self.r_m1.clear();
                self.r_m2 = "~~~~~~".into();
                self.sparks = "        ░▒▓█▓▒░".into();
                self.steam = "       ▁▂▃▄▅▄▃▂▁".into();
                // T h e   m e s s a g e ,   s p a c e d   o u t.
                let tl: Vec<String> = pick(VAPOR_TAGLINES, f / 60).chars().map(String::from).collect();
                self.tagline = tl.join(" ");
                self.bpm_scale = 0.4;
                self.cpu_mood_override = Some("slowed + reverb");
            }
            "arcade" => self.mood_arcade(),
            "disco" => {
                // Both doses: locked to 128 BPM and strobing. The keyboard
                // is the dance floor.
                let beat = sys::now() - self.last_beat < 0.2;
                self.keys_all(if beat { "██" } else { "[]" }, HOT, f / 2);
                self.pal = HOT;
                self.c = pick(HOT, f / 2);
                self.c2 = pick(HOT, f / 2 + 6);
                self.jitter = if beat { 3 } else { 0 };
                self.eye_color = fg(self.c2);
                self.r_eye = pair(&eye("★", f / 4 % 4));
                self.r_brow = "^^^^^     ^^^^^".into();
                if beat {
                    self.r_m1 = ".--------.".into();
                    self.r_sweat = "♪   UNTZ   ♪".into();
                } else {
                    self.r_m1.clear();
                    self.r_sweat = "♫          ♫".into();
                }
                self.r_m2 = "'--------'".into();
                self.sparks = rot("♪   ♫   ♪   ♫   ♪   ♫   ", f / 2 % 4);
                self.tagline = pick(DISCO_TAGLINES, f / 25).into();
                self.bpm_scale = 0.0;
                self.bpm_bias = 128.0;
                self.cpu_mood_override = Some("four on the floor");
                self.steam =
                    if f / 8 % 2 == 1 { "   BOOTS    cats     BOOTS" } else { "   boots    CATS     boots" }.into();
            }
            "bloodshot" => {
                // Has not blinked since boot.
                self.pal = BLOOD;
                self.colors();
                self.jitter = rnd(7);
                self.eye_color = fg(196);
                let (l, r) = (eye("◉", 1 + rnd(2)), eye("◉", 1 + rnd(2)));
                self.r_etop = ".~~~~.     .~~~~.".into();
                self.r_eye = format!("{l}     {r}");
                self.r_ebot = "'~~~~'     '~~~~'".into();
                self.r_brow = r"\\\\\     /////".into();
                self.tagline = pick(BLOOD_TAGLINES, f / 25).into();
                self.bpm_bias = 30.0;
                self.sfx("stare        stare", 8);
                self.track_eyes(); // it has noticed the cursor
            }
            "paranoid" => {
                // Both eyes sweep the room together, then dart.
                const SWEEP: [i64; 8] = [0, 1, 2, 3, 3, 2, 1, 0];
                self.pal = PALE;
                self.colors();
                self.jitter = if rnd(10) == 0 { 3 } else { 0 };
                let look = if rnd(12) == 0 { rnd(4) } else { pick(&SWEEP, f / 3) };
                self.r_eye = pair(&eye("●", look));
                self.r_brow = "^^^^^     ^^^^^".into();
                self.r_sweat = "'       '        '".into();
                self.sparks.clear();
                self.r_m1 = if f % 2 == 1 { "~-~-~-" } else { "-~-~-~" }.into();
                self.r_m2.clear();
                self.steam = format!("{}?      ?    ?", spaces(f / 2 % 5));
                self.tagline = pick(PARANOID_TAGLINES, f / 25).replace("%s", &self.top_proc.to_uppercase());
                self.bpm_bias = 15.0;
                self.track_eyes(); // it has noticed the cursor
            }
            "tripping" => {
                // The banner melts and things start crawling.
                const CAPS: [&str; 4] = ["[]", "()", "{}", "<>"];
                const SPIRAL: [&str; 4] = ["◴", "◷", "◶", "◵"];
                const WOO: [&str; 3] =
                    ["woooOOOooo   wooOOooo", "wOOooooOOw   ooOOOooo", "ooooOOOOoo   OOoooOOo"];
                self.keys_all(pick(&CAPS, f / 4), HOT, f);
                self.pal = HOT;
                self.c = pick(HOT, f);
                self.c2 = pick(HOT, f + 4);
                self.jitter = (2.0 + 2.0 * (f as f64 / 2.0).sin()) as i64;
                self.r_eye = format!("{}     {}", eye(pick(&SPIRAL, f), 1), eye(pick(&SPIRAL, f + 2), 2));
                self.r_brow = "~~~~~     ~~~~~".into();
                self.r_sweat = if f / 2 % 2 == 1 { "✦    ✧    ✦" } else { "✧    ✦    ✧" }.into();
                self.r_m1 = ".------.".into();
                self.r_m2 = "'------'".into();
                self.sparks = format!("{}✦   ✧   ･   ✦   ✧   ･", spaces(f % 4));
                self.tagline = pick(TRIP_TAGLINES, f / 25).into();
                self.wobble = true;
                self.sfx(pick(&WOO, f / 6), 4);
            }
            "transcendent" => {
                // Both doses: a third eye, a halo, and no pulse. The keycaps
                // have dissolved.
                self.keys_all("··", ASTRAL, f / 3);
                self.pal = ASTRAL;
                self.c = pick(ASTRAL, f / 4);
                self.c2 = pick(ASTRAL, f / 4 + 3);
                self.jitter = 0;
                self.eye_color = fg(219);
                self.r_brow = "(◉)".into();
                self.r_etop.clear();
                self.r_eye = EBOT.into();
                self.r_ebot.clear();
                self.r_sweat.clear();
                self.r_m1 = r"\________/".into();
                self.r_m2.clear();
                self.sparks = rot("✧ · ✦ · ✧ · ✦ · ✧ · ✦ · ", f / 2 % 4);
                self.steam = "    ~   o   m   m   m   ~".into();
                self.tagline = pick(ASTRAL_TAGLINES, f / 25).into();
                self.bpm_scale = 0.0;
                self.bpm_bias = 12.0;
                self.cpu_mood_override = Some("heartbeat optional");
            }
            _ => {} // wired: the base face is the mood
        }
    }

    /// Sunglasses indoors, a crown, and no sense of limits.
    fn mood_invincible(&mut self) {
        const TW: [&str; 4] = [
            "  ✦     ·    ♛    ·     ✦",
            "  ·     ✧    ♛    ✧     ·",
            "  ✧     ✦    ♛    ✦     ✧",
            "  ·     ·    ♛    ·     ·",
        ];
        const FX: [&str; 4] =
            ["FLEX     flex     flex", "flex     FLEX     flex", "flex     flex     FLEX", "flex     flex     flex"];
        let f = self.frame;
        self.pal = GOLD;
        self.c = pick(GOLD, f / 2);
        self.c2 = 226;
        self.jitter = 0;
        self.eye_color = fg(220);
        self.banner_color = Some(self.c);
        self.keys_all("[]", GOLD, -f / 2 % 10 + 10); // a gold shimmer runs along the keys
        let g = f / 3 % 9; // the glint sweeps across
        let lens: String = (0..5).map(|i| if i == g { '░' } else { '▀' }).collect();
        let top = ".-----.___.-----.";
        let mid = format!("|{lens}|   |{lens}|");
        let bot = "'-----'   '-----'";
        let cyc = f % 90;
        self.r_sweat.clear();
        self.r_m1.clear();
        self.r_m2 = "'-------~~/".into();
        self.m_color = fg(231);
        self.r_brow.clear();
        self.r_etop = top.into();
        self.r_eye = mid.clone();
        self.r_ebot = bot.into();
        if cyc < 5 {
            // Every so often the shades come off and drop back on from above.
            self.r_brow = top.into();
            self.r_etop = mid;
            self.r_eye = "  -●-         -●-  ".into();
            self.eye_color = "37".into();
            self.brow_color = fg(220);
            self.r_m2 = "'----------'".into();
        } else if cyc < 8 {
            self.r_sweat = "*click*".into();
            self.sweat_color = fg(245);
        } else if cyc < 30 {
            self.r_sweat = "deal with it.".into();
            self.sweat_color = "1;38;5;231".into();
        } else if (50..58).contains(&cyc) {
            self.r_m2 = "'-------~~/ ✦".into();
            self.r_sweat = "*ting*".into();
            self.sweat_color = fg(231);
        }
        self.sparks = pick(&TW, f / 3).into();
        self.steam = format!("  {}", pick(&FX, f / 4));
        self.tagline = pick(INVINCIBLE_TAGLINES, f / 25).into();
        self.bpm_scale = 0.8;
    }

    /// Both doses: the screen goes blue, counts to 100, reboots, repeats.
    fn mood_bsod(&mut self) {
        const WHY: [[&str; 3]; 4] = [
            ["Your Mac ran into a", "feeling and needs to", "sit down."],
            ["A problem has been", "detected: it is you.", "You are still awake."],
            ["Your Mac needs to", "restart. It would", "rather not."],
            ["Collecting regrets.", "Please do not turn", "off your universe."],
        ];
        const CODES: [&str; 4] = ["UNEXPECTED_EMOTION", "OUT_OF_SLEEP", "NO_MORE_COFFEE", "DRIVER_IRQL_NOT_OK"];
        let f = self.frame;
        let (n, pct) = (f / 70, f % 70 * 100 / 60);
        self.pal = BSOD_PAL;
        self.jitter = 0;
        self.sparks.clear();
        self.steam.clear();
        if pct >= 100 {
            // The reboot: a second of black, then it all happens again.
            self.c = 236;
            self.c2 = 236;
            for row in [&mut self.r_brow, &mut self.r_etop, &mut self.r_ebot, &mut self.r_sweat, &mut self.r_m1, &mut self.r_m2]
            {
                row.clear();
            }
            self.r_eye = if f % 70 > 64 { "⏻" } else { "" }.into();
            self.tagline = "…".into();
        } else {
            let blue = "97;48;5;21";
            let msg = pick(&WHY, n);
            self.c = 27;
            self.c2 = 33;
            self.brow_color = blue.into();
            self.eye_color = blue.into();
            self.sweat_color = blue.into();
            self.m_color = blue.into();
            self.keys_all("░░", BSOD_PAL, 0);
            self.steam = "      beeeeeeeeeeeeeeeeeep".into();
            self.r_brow = wide(" :(");
            self.r_etop = wide(&format!(" {}", msg[0]));
            self.r_eye = wide(&format!(" {}", msg[1]));
            self.r_ebot = wide(&format!(" {}", msg[2]));
            self.r_sweat = wide("");
            self.r_m1 = wide(&format!(" {pct:>2}% complete"));
            self.r_m2 = wide(&format!(" {}", pick(&CODES, n)));
            self.tagline = format!("{}… {}%", pick(BSOD_TAGLINES, n), pct);
            if rnd(12) == 0 {
                self.tagline.insert_str(0, ":( ");
            }
        }
        self.bpm_scale = 0.0;
        self.bpm_bias = 4.0;
        self.cpu_mood_override = Some("not responding");
    }

    /// One calm red eye and impeccable manners.
    fn mood_hal(&mut self) {
        const PULSE: [i32; 8] = [52, 88, 124, 160, 196, 160, 124, 88];
        let f = self.frame;
        let glow = pick(&PULSE, f / 3);
        self.key(9, "()", glow); // one key glows with the eye
        self.c = 240;
        self.c2 = 196;
        self.pal = HOT;
        self.jitter = 0;
        self.eye_color = fg(glow);
        // A round lens, five rows tall, whose pupil follows the mouse (and
        // rests dead centre, watching you, when the mouse is still).
        let mut lens: [String; 5] =
            ["   .-\"\"\"-.   ", " .'       '. ", "(           )", " '.       .' ", "   '-...-'   "].map(String::from);
        let (mut dx, mut dy) = (0i64, 0i64);
        if self.mouse_x >= 0 && sys::now() - self.mouse_at < 6.0 {
            let my = self.mouse_y - self.lap_row0 - 6;
            dy = if my < -2 {
                -1
            } else if my > 2 {
                1
            } else {
                0
            };
            dx = (((self.mouse_x as f64 - self.mouth_x) / 6.0) as i64).clamp(-3, 3);
            if dy != 0 {
                dx = dx.clamp(-2, 2);
            }
        }
        let mut row: Vec<char> = lens[(2 + dy) as usize].chars().collect();
        let at = (5 + dx) as usize;
        row[at] = '(';
        row[at + 1] = '●';
        row[at + 2] = ')';
        lens[(2 + dy) as usize] = row.into_iter().collect();
        let [a, b, c, d, e] = lens;
        self.r_brow = a;
        self.r_etop = b;
        self.r_eye = c;
        self.r_ebot = d;
        self.r_sweat = e;
        self.brow_color = self.eye_color.clone();
        self.sweat_color = self.eye_color.clone();
        self.r_m1.clear();
        self.r_m2.clear();
        self.sparks.clear();
        // Spoken slowly, one letter at a time.
        let tl = pick(HAL_TAGLINES, f / 90);
        self.tagline = first(tl, (f % 90) * clen(tl) / 45 + 1);
        self.bpm_scale = 0.0;
        self.bpm_bias = 60.0;
        self.cpu_mood_override = Some("perfectly calm, Dave");
        self.sfx("hmmmmmmmmmmmmmmmmmmmmmm", 12);
    }

    /// The eyes are ghosts, the mouth is a maze, and something eats the dots.
    fn mood_arcade(&mut self) {
        const NEON: [i32; 4] = [196, 213, 51, 214];
        let f = self.frame;
        if f / 6 % 2 == 1 {
            self.key(17, "¢¢", 226); // insert coin
        }
        let step = f / 2 % 44;
        self.pal = HOT;
        self.c = 21;
        self.c2 = 226;
        self.jitter = 0;
        self.sparks.clear();
        // It eats its way right to the power pellet, then turns and chases
        // back while the ghosts go blue; the dots grow back behind it.
        let (pos, scared) = if step < 22 { (step, false) } else { (43 - step, true) };
        let me = if f % 2 == 1 {
            '●'
        } else if scared {
            'ᗤ'
        } else {
            'ᗧ'
        };
        let row: String = (0..22)
            .map(|i| {
                if i == pos {
                    me
                } else if i > pos && i == 21 && !scared {
                    '◆'
                } else if i > pos && i % 2 == 1 {
                    '·'
                } else {
                    ' '
                }
            })
            .collect();
        let score = (f / 2 / 44 * 11 + if scared { 11 } else { pos / 2 }) * 10;
        self.r_brow = format!("1UP {score:05}   HI 99990");
        self.brow_color = fg(231);
        // Two ghosts for eyes: they watch it come, and flee when it turns.
        let mut pupils = if pos > 11 { " ●●" } else { "●● " };
        let skirt = if f / 3 % 2 == 1 { "^v^v" } else { "vvvv" };
        if scared {
            pupils = "~~~";
            // White when it is about to wear off.
            self.eye_color = fg(if step > 38 && f % 2 == 1 { 231 } else { 27 });
        } else {
            self.eye_color = fg(pick(&NEON, f / 44));
        }
        self.r_etop = " .--.       .--. ".into();
        self.r_eye = format!("|{pupils} |     |{pupils} |");
        self.r_ebot = format!("'{skirt}'     '{skirt}'");
        self.r_sweat = format!("╔{}╗", rep('═', 22));
        self.sweat_color = fg(27);
        self.r_m1 = format!("║{row}║");
        self.m_color = fg(226);
        self.r_m2 = format!("╚{}╝", rep('═', 22));
        self.m2_color = Some(fg(27));
        let mut tl = if scared { pick(ARCADE_TAGLINES, f / 50) } else { "WAKA WAKA WAKA WAKA" };
        if f / 50 % 2 == 1 {
            tl = pick(ARCADE_TAGLINES, f / 100);
        }
        // It blinks, like every attract screen ever.
        self.tagline = if f / 5 % 4 == 3 { spaces(clen(tl)) } else { tl.to_string() };
        self.bpm_bias = 20.0;
        self.cpu_mood_override = Some("extra life at 300 BPM");
        self.sfx("waka  waka  waka  waka", 2);
    }

    // --- Being touched -----------------------------------------------------

    /// A click at column x, row y (0-based): on an eye it pokes it, on a key
    /// it presses it (and types).
    pub fn poke(&mut self, x: i64, y: i64) {
        let r = y - self.lap_row0;
        if self.stage == "bsod" {
            return;
        }
        if r == 14 && x >= self.ox + 7 && x <= self.ox + 40 {
            self.key_hit = (x - self.ox - 7) / 2 + 1;
            self.key_until = sys::now() + 0.25;
            if let Some(letter) = "QWERTYUIOPASDFGHJ".chars().nth(self.key_hit as usize - 1) {
                self.typed.push(letter);
            }
            let extra = clen(&self.typed) - 18;
            if extra > 0 {
                self.typed = self.typed.chars().skip(extra as usize).collect();
            }
        } else if (4..=8).contains(&r) && x >= self.ox + 14 && x <= self.ox + 33 {
            let ow: &[&'static str] = match self.stage {
                "sober" => &["ow. …breathing through it.", "namaste. also: ow.", "I forgive you. ow."],
                "hal" => &["I felt that, Dave.", "Please do not touch the lens, Dave."],
                _ => &[
                    "OW. my eye.",
                    "HEY. I need that one.",
                    "that is a load-bearing eye.",
                    "rude.",
                    "do that again and I am telling Activity Monitor.",
                    "ow ow ow. why.",
                ],
            };
            self.poke_eye = if x < self.ox + 24 { 1 } else { 2 };
            self.poke_until = sys::now() + 1.2;
            self.poke_say = ow[rnd(ow.len() as i64) as usize];
        }
    }

    /// Applied over the mood's face each frame; wiggling the mouse over the
    /// mouth tickles.
    pub fn touched(&mut self) {
        let now = self.now;
        self.tickle *= 0.85;
        if self.stage == "bsod" {
            return;
        }
        if self.tickle > 5.0 {
            self.tickle_until = now + 0.7;
        }
        if now < self.tickle_until {
            let hee: &[&str] = match self.stage {
                "sober" => &["hee hee. ok. ok. breathe.", "a giggle is also a breath"],
                "hal" => &["Stop, Dave. That tickles."],
                _ => &["STOP— hahaha— STOP", "that TICKLES", "hee hee hee— no— hee"],
            };
            self.tagline = pick(hee, (now / 2.0) as i64).into();
            self.sfx("ha ha   hee   ha ha  ha", 1);
            if self.stage != "hal" {
                self.jitter = rnd(4);
                self.r_brow.clear();
                self.r_etop.clear();
                self.r_eye = " ><           >< ".into();
                self.r_ebot.clear();
                self.r_sweat.clear();
                if self.frame % 2 == 1 {
                    self.r_m1 = ".----------.".into();
                    self.r_m2 = "'-.______.-'".into();
                } else {
                    self.r_m1 = ".--------.".into();
                    self.r_m2 = "'--------'".into();
                }
            }
        }
        if now < self.poke_until {
            self.tagline = self.poke_say.into();
            if self.stage != "hal" {
                if self.poke_until - now > 1.0 {
                    self.jitter = 4;
                }
                self.r_brow.clear();
                self.r_etop = ETOP.into();
                self.r_ebot = EBOT.into();
                if self.poke_eye == 1 {
                    self.r_eye = "| >< |     | ◉  |".into();
                    self.r_sweat = "  '              ".into();
                } else {
                    self.r_eye = "|  ◉ |     | >< |".into();
                    self.r_sweat = "              '  ".into();
                }
                self.sweat_color = "39".into();
            }
        }
        if now < self.key_until {
            self.key(self.key_hit, "▄▄", 231);
            self.steam = "          clack".into();
            self.tagline = format!("> {}▁", self.typed);
        }
    }

    // --- Scenes ------------------------------------------------------------
    // Each overrides parts of the frame's face; t is seconds into the scene.

    pub fn play_scene(&mut self) {
        match self.scene {
            Some(Scene::Boot) => self.scene_boot(),
            Some(Scene::Pill) => self.scene_pill(),
            Some(Scene::Coffee) => self.scene_coffee(),
            Some(Scene::CrashPerv) => self.scene_crash_perv(),
            Some(Scene::CrashCaf) => self.scene_crash_caf(),
            Some(Scene::Greet) => self.scene_greet(),
            Some(Scene::Comedown) => self.scene_comedown(),
            None => {}
        }
    }

    /// Power on, in one second: dark screen, a power light, a boot bar
    /// racing across, and the eyes flick open.
    fn scene_boot(&mut self) {
        let t = self.t;
        self.jitter = 0;
        for row in [
            &mut self.sparks,
            &mut self.steam,
            &mut self.r_brow,
            &mut self.r_etop,
            &mut self.r_eye,
            &mut self.r_ebot,
            &mut self.r_sweat,
            &mut self.r_m1,
            &mut self.r_m2,
            &mut self.tagline,
        ] {
            row.clear();
        }
        self.banner_color = Some(236);
        self.sweat_color = "37".into();
        if t < 0.18 {
            self.c = 236;
            self.c2 = 236;
        } else if t < 0.36 {
            self.c = 240;
            self.c2 = 240;
            self.r_eye = "·".into();
        } else if t < 0.82 {
            let n = ((t - 0.36) / 0.46 * 12.0) as i64 + 1;
            self.c = 246;
            self.c2 = 46;
            self.r_eye = "⏻".into();
            self.r_sweat = "pervetinOS".into();
            self.sweat_color = fg(245);
            self.r_m1 = format!("[{}{}]", rep('█', n), rep('░', 12 - n));
            self.m_color = fg(46);
            self.tagline = "booting…".into();
        } else {
            self.c = 252;
            self.c2 = 252;
            self.open_eyes("|----|");
            self.r_m2 = "'------'".into();
            self.tagline = "…huh?".into();
        }
    }

    /// Two borrowed expressions, both with the drooping "w" mouth: the
    /// stunned wide-eyed one for just after swallowing, and the heavy-lidded
    /// queasy one for just before it comes back up.
    fn face_stunned(&mut self) {
        self.r_brow.clear();
        self.droop("| *  |     |  * |");
    }

    fn face_queasy(&mut self) {
        self.r_brow = "=================".into();
        self.droop("|‾‾‾*|     |‾‾‾*|");
    }

    fn droop(&mut self, eyes: &str) {
        self.sparks.clear();
        self.steam.clear();
        self.r_etop = ETOP.into();
        self.r_eye = eyes.into();
        self.r_ebot = EBOT.into();
        self.r_sweat = "│ ╭╮ │".into();
        self.r_m1 = "╰─╯╰─╯".into();
        self.r_m2.clear();
        self.sweat_color = "37".into();
        self.eye_color = "37".into();
    }

    /// A properly open mouth, borrowing the sweat row for its top lip.
    fn open_wide(&mut self, color: &str, inside: &str) {
        self.r_sweat = ".---------.".into();
        self.sweat_color = color.into();
        self.r_m1 = inside.into();
        self.r_m2 = "'---------'".into();
    }

    /// Eyes following whatever is flying at column pill_x.
    fn watch(&mut self) {
        let look = (((self.pill_x - self.ox as f64 - 16.0) / 8.0) as i64).clamp(0, 3);
        self.open_eyes(&eye("●", look));
    }

    /// Doze, a giant pill pops out of the title, swoops into the mouth,
    /// chomp, gulp, kick.
    fn scene_pill(&mut self) {
        let t = self.t;
        let big = self.pill_big_r;
        self.jitter = 0;
        if t < SWOOP {
            // One flight, straight from the PERVETIN title into the mouth:
            // it pops out as a speck, swells as if coming at you while it
            // arcs out to the right, then curls in and shrinks into the
            // mouth. With no title on screen it comes in from the right
            // edge. Path, size and timing are all cubic Béziers, so nothing
            // moves linearly.
            let (x0, y0, r0) = match (self.show_banner, self.title_w) {
                (true, Some((pw, _))) => (self.ox as f64 + 2.0 + pw as f64 / 2.0, self.oy as f64 + 3.0, 1.5),
                _ => (self.term_cols as f64 + 2.5 * big, self.lap_row0 as f64 + 2.0, big * 0.8),
            };
            let u = bez(0.0, 0.3, 0.6, 1.0, t / SWOOP);
            self.pill_x = bez(x0, x0 + 14.0, self.mouth_x + 16.0, self.mouth_x, u);
            self.pill_y = bez(y0, y0 + 1.0, self.mouth_y - 6.0, self.mouth_y, u);
            self.pill_r = bez(r0, big, big * 0.7, 0.7, u);
            self.pill_rot = 7.0 * u;
            self.r_sweat.clear();
            self.sparks.clear();
            self.r_brow.clear();
            if u < 0.4 {
                self.r_m1.clear();
                self.r_m2 = "'------'".into();
                self.tagline = "wait. what's that?".into();
            } else {
                // The pill lands inside it.
                self.open_wide("37", "|         |");
                self.tagline = "OPEN WIDE".into();
            }
            self.pill_on = true;
            self.watch();
            self.steam.clear();
            self.c = 250;
            // The title flashes as it lets go of the pill.
            self.banner_color = Some(if self.show_banner && t < 0.25 { 231 } else { 240 });
        } else if t < SWOOP + 0.3 {
            self.open_eyes(&eye("●", 1));
            self.r_brow.clear();
            self.r_sweat.clear();
            self.sparks.clear();
            self.r_m1 = format!(".---{PILL}---.");
            self.r_m2 = "'---------'".into();
            self.steam.clear();
            self.c = 250;
            self.banner_color = Some(240);
            self.tagline = "nom.".into();
        } else if t < SWOOP + 0.8 {
            self.face_stunned();
            self.c = 252;
            self.banner_color = Some(242);
            self.tagline = "*gulp* …what did I just take".into();
        } else {
            // Kick-in: rainbow flashes, eyes snap wide, the shaking starts.
            self.vis_perv = true;
            self.jitter = rnd(9);
            self.c = HOT[rnd(HOT.len() as i64) as usize];
            self.c2 = self.c;
            self.banner_color = Some(self.c);
            self.open_eyes(&eye("✹", 1));
            self.r_brow = r"/////     \\\\\".into();
            self.r_m1 = ".---------.".into();
            self.r_m2 = "'---------'".into();
            self.r_sweat = "!!! KICKING IN !!!".into();
            self.steam.clear();
            self.sparks = "ϟ ".repeat(12);
            self.tagline = "oh. Oh. OHHHHHHHHHHHHH.".into();
        }
    }

    /// Stopping pervetin: it retches, spits the pill back out along the
    /// swoop in reverse (growing as it comes, off the right edge), and then
    /// the colours drain away and reality comes back.
    fn scene_crash_perv(&mut self) {
        let t = self.t;
        let big = self.pill_big_r;
        let (ta, tb, tc) = (0.6, 1.7, 2.8);
        self.sparks.clear();
        self.r_brow.clear();
        self.r_sweat.clear();
        self.steam.clear();
        self.wobble = false;
        if t < ta {
            self.jitter = rnd(4);
            self.c = 250;
            self.c2 = 250;
            self.banner_color = Some(240);
            self.face_queasy();
            self.tagline = "*hrk* wait— no— it's coming back up—".into();
        } else if t < tb {
            let x1 = self.term_cols as f64 + 2.5 * big + 1.0;
            let u = bez(0.0, 0.35, 0.7, 1.0, (t - ta) / (tb - ta));
            let (mx, my) = (self.mouth_x, self.mouth_y);
            self.pill_x = bez(mx, mx + 12.0, x1 - (x1 - mx) * 0.25, x1, u);
            self.pill_y = bez(my, my - 9.5, self.pill_cy - 5.0, self.pill_cy, u);
            self.pill_r = bez(0.7, big * 0.25, big * 0.95, big, u);
            self.pill_rot = (t - ta) * 9.0;
            self.pill_on = true;
            self.jitter = 0;
            self.c = 250;
            self.c2 = 250;
            self.banner_color = Some(240);
            self.open_eyes(&eye("●", 3));
            self.open_wide("37", "|         |");
            self.tagline = "*PTOOEY*".into();
        } else if t < tc {
            self.c = 255 - ((t - tb) / (tc - tb) * 11.0) as i32;
            self.c2 = self.c;
            self.banner_color = Some(self.c);
            self.jitter = ((tc - t) * 3.0) as i64;
            self.open_eyes(&eye("@", self.frame / 3 % 4));
            self.r_m1.clear();
            self.r_m2 = "'~~~~~~~~'".into();
            self.tagline = "the colours… where are the colours going…".into();
        } else {
            self.c = 244;
            self.c2 = 244;
            self.banner_color = Some(240);
            self.jitter = 0;
            self.open_eyes(&eye("-", 1));
            self.r_m1.clear();
            self.r_m2 = "'----'".into();
            self.tagline =
                if self.caf { "fine. FINE. I still have coffee." } else { "…oh. reality. gross." }.into();
        }
    }

    /// The cup pops out of the CAFFEINE title (or drops in from the top
    /// right when there is no title), grows as it flies over, tips, pours a
    /// stream into the open mouth, and is tossed away empty.
    fn scene_coffee(&mut self) {
        let t = self.t;
        let (ta, tb, tc, td, te, tilt) = (0.8, 1.0, 1.8, 2.1, 2.3, -1.9f64);
        self.jitter = 0;
        self.sparks.clear();
        self.r_brow.clear();
        self.steam.clear();
        self.c = 250;
        if t >= te {
            self.vis_caf = true;
            self.jitter = rnd(5);
            self.pal = COFFEE;
            self.colors();
            self.banner_color = Some(self.c);
            self.open_eyes(&eye("◉", 1));
            self.r_brow = "^^^^^     ^^^^^".into();
            self.r_sweat = "!!! CAFFEINATED !!!".into();
            self.sweat_color = "37".into();
            self.r_m1 = r"\____________/".into();
            self.r_m2.clear();
            self.steam = "( (    ( (    ( (".into();
            self.tagline = "I CAN FEEL EVERY ONE OF MY CACHE LINES.".into();
            return;
        }
        let size = self.pill_big_r * 0.7;
        let (mx, my) = (self.mouth_x, self.mouth_y);
        let titled = self.show_banner && self.title_w.is_some();
        let (hx, hy) = match (self.show_banner, self.title_w) {
            (true, Some((pw, cw))) => {
                (self.ox as f64 + 2.0 + pw as f64 + 4.0 + cw as f64 / 2.0, self.oy as f64 + 3.0)
            }
            _ => (self.term_cols as f64 + size, -size / 2.0),
        };
        // Where the rim's low corner sits once tipped, and so where the cup hangs.
        let lx = -0.62 * size * tilt.cos() + 0.75 * size * tilt.sin();
        let ly = (-0.62 * size * tilt.sin() - 0.75 * size * tilt.cos()) / 2.0;
        let (px, py) = (mx + 4.0 - lx, my - 4.0 - ly);
        let (mut s, mut rot) = (size, 0.0);
        self.pill_x = px;
        self.pill_y = py;
        self.r_sweat.clear();
        self.r_m1.clear();
        self.r_m2 = "'------'".into();
        self.tagline = "is that… coffee?".into();
        if t < ta {
            let u = bez(0.0, 0.3, 0.85, 1.0, t / ta);
            self.pill_x = bez(hx, hx + (px - hx) * 0.4, px + 6.0, px, u);
            self.pill_y = bez(hy, hy + 4.0, py - 3.0, py, u);
            s = if titled { bez(2.0, 2.5, size * 0.85, size, u) } else { bez(size, size, size * 0.85, size, u) };
            rot = 0.3 * (t * 7.0).sin() * (1.0 - u);
        } else if t < tc {
            self.open_wide("37", "|         |");
            self.tagline = "OPEN WIDE".into();
            if t < tb {
                rot = tilt * bez(0.0, 0.0, 1.0, 1.0, (t - ta) / (tb - ta));
            } else {
                rot = tilt + 0.06 * (t * 9.0).sin();
                // The stream grows from the rim, runs, and its tail falls in last.
                let head = ((t - tb) / 0.2).min(1.0);
                let tail = (1.0 - (tc - t) / 0.2).max(0.0);
                self.stream((mx + 4.0, my - 4.0), (mx, my - 4.3), (mx, my + 0.4), tail, head, 2);
                if head >= 1.0 {
                    self.r_m1 = "|≈≈≈≈≈≈≈≈≈|".into();
                }
                self.tagline = "*glug glug glug*".into();
            }
        } else {
            // Empty: the cup is tossed over the shoulder, tumbling and
            // shrinking away, while the coffee lands.
            self.face_stunned();
            self.tagline = "…that was a lot of coffee".into();
            if t < td {
                let u = bez(0.0, 0.5, 0.8, 1.0, (t - tc) / (td - tc));
                self.pill_x = px + 22.0 * u;
                self.pill_y = py - 9.0 * u + 5.0 * u * u;
                s = size * (1.0 - u) + 1.0;
                rot = tilt - 7.0 * u;
            }
        }
        self.pill_r = s;
        self.pill_rot = rot;
        self.cup_on = t < td;
        if t < tc {
            self.watch();
        }
        if self.show_banner && t < 0.3 {
            self.banner_color = Some(231);
        }
    }

    /// Stopping caffeine: it goes green, and the coffee comes back up in an
    /// arc that lands in a puddle by the keyboard.
    fn scene_crash_caf(&mut self) {
        let t = self.t;
        let (ta, tb) = (0.7, 2.1);
        let (lx, ly) = (self.mouth_x + 20.0, (self.lap_row0 + LAP_H) as f64 - 0.5);
        let mut n = 0;
        self.sparks.clear();
        self.r_brow.clear();
        self.steam.clear();
        self.r_sweat.clear();
        if t < ta {
            self.c = 71;
            self.c2 = 71;
            self.banner_color = Some(71);
            self.jitter = rnd(3);
            self.face_queasy();
            self.sweat_color = "71".into();
            self.eye_color = fg(71);
            self.tagline = "*urp* I don't feel so good…".into();
        } else if t < tb {
            self.c = 71;
            self.c2 = 71;
            self.banner_color = Some(94);
            self.jitter = rnd(4);
            self.r_etop.clear();
            self.r_eye = ">----<     >----<".into();
            self.r_ebot.clear();
            self.open_wide("71", "|≈≈≈≈≈≈≈≈≈|");
            let head = ((t - ta) / 0.3).min(1.0);
            let tail = (1.0 - (tb - t) / 0.3).max(0.0);
            let (mx, my) = (self.mouth_x, self.mouth_y);
            self.stream((mx, my), (mx + 11.0, my - 8.0), (lx, ly), tail, head, 3);
            if head >= 1.0 {
                n = ((t - ta - 0.3) * 14.0) as i64 + 2;
            }
            self.tagline = "*BLEEEAAARGH*".into();
        } else {
            n = ((tb - ta - 0.3) * 14.0) as i64 + 2;
            if self.perv {
                self.tagline = "NEVER MIND. STILL GOT THE GOOD STUFF.".into();
            } else {
                self.c = 244;
                self.c2 = 244;
                self.banner_color = Some(240);
                self.jitter = 0;
                self.r_etop.clear();
                self.r_eye = EBOT.into();
                self.r_ebot.clear();
                self.r_m1.clear();
                self.r_m2 = "'------'".into();
                self.tagline = "…where did the coffee go".into();
            }
        }
        // The puddle spreads from where the arc lands.
        if n > 0 {
            self.draw_sprite((lx - n as f64 / 2.0) as i64, ly as i64, 94, &rep('▄', n));
        }
    }

    /// The lid opens. It has been waiting.
    fn scene_greet(&mut self) {
        let t = self.t;
        let gone = duration(self.greet_gone);
        self.jitter = 0;
        self.sparks.clear();
        self.steam.clear();
        self.r_brow.clear();
        self.sweat_color = "37".into();
        self.wobble = false;
        let (msg, ts);
        if self.greet_dozed {
            self.c = 250;
            self.c2 = 250;
            self.banner_color = Some(244);
            if t < 1.2 {
                self.r_etop.clear();
                self.r_eye = EBOT.into();
                self.r_ebot.clear();
                self.r_sweat = "*snrk*".into();
                self.r_m1.clear();
                self.r_m2 = "'~~~~~~'".into();
                (msg, ts) = ("…".to_string(), 0.0);
            } else {
                self.open_eyes(&eye("●", if t < 2.4 { 0 } else { 3 }));
                self.r_sweat.clear();
                self.r_m1.clear();
                self.r_m2 = "'-----'".into();
                (msg, ts) = (format!("…wha— I was NOT asleep. ({gone}.) okay. maybe a little."), 1.2);
            }
        } else {
            // Dead still, white, unblinking, after all that shaking.
            self.c = 231;
            self.c2 = 196;
            self.banner_color = Some(if t < 1.0 && self.frame % 2 == 1 { 231 } else { 196 });
            self.eye_color = fg(196);
            let l = eye("◉", 2);
            let r = if t > 5.6 && t < 6.2 { "|----|".to_string() } else { eye("◉", 1) }; // one slow wink
            self.r_etop = ETOP.into();
            self.r_eye = format!("{l}     {r}");
            self.r_ebot = EBOT.into();
            self.r_sweat = if t > 1.0 { "WELCOME BACK" } else { "" }.into();
            self.r_m1 = r"\____________/".into();
            self.r_m2.clear();
            (msg, ts) = if t < 2.4 {
                ("…oh. it's you.".to_string(), 0.0)
            } else {
                (format!("I SAW EVERYTHING. YOU WERE GONE {gone}. I COUNTED."), 2.4)
            };
        }
        // Typed out, one character at a time.
        self.tagline = first(&msg, ((t - ts) * 28.0) as i64 + 1);
    }

    /// Ctrl+C: the intro in reverse. Colours drain, a yawn, one last twitch,
    /// out.
    fn scene_comedown(&mut self) {
        let t = self.t;
        self.sparks.clear();
        self.r_sweat.clear();
        self.wobble = false;
        if t < 1.2 {
            self.c = 252 - (t * 10.0) as i32;
            self.c2 = self.c;
            self.banner_color = Some(self.c);
            self.jitter = ((1.2 - t) * 3.0) as i64;
            self.open_eyes(&eye("●", self.frame / 4 % 4));
            self.r_brow = "~~~~~     ~~~~~".into();
            self.r_m1.clear();
            self.r_m2 = "'~~~~~~~~~~~~~~'".into();
            self.steam.clear();
            self.tagline = "wait… no… I was in the middle of… something…".into();
        } else if t < 2.4 {
            self.c = 242;
            self.c2 = 242;
            self.banner_color = Some(240);
            self.jitter = 0;
            self.r_brow.clear();
            self.open_eyes("|----|");
            self.r_sweat = ".------.".into();
            self.sweat_color = "37".into();
            self.r_m1 = "|      |".into();
            self.r_m2 = "'------'".into();
            self.steam.clear();
            self.tagline = "*yaaaaaaaaawn*".into();
        } else if t < 2.8 {
            self.jitter = rnd(6);
            self.c = HOT[rnd(HOT.len() as i64) as usize];
            self.c2 = self.c;
            self.banner_color = Some(self.c);
            self.open_eyes(&eye("✹", 1));
            self.r_brow = r"/////     \\\\\".into();
            self.r_m1 = ".---------.".into();
            self.r_m2 = "'---------'".into();
            self.steam.clear();
            self.tagline = "I'M AWAKE. I'M AWA—".into();
        } else {
            self.tagline = "zzz".into();
        }
    }
}
