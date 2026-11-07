//! Cut files for a low raft of panpipes, E3 to E4 in E natural minor: eight
//! closed pipes of rectangular bore in 3mm ply, each tuned by a press-fit
//! plug.
//!
//!     target/release/panpipes --no-write                # the checks, write nothing
//!     target/release/panpipes --out=DIR                 # the default build into DIR
//!     target/release/panpipes --edge=notch --out=DIR    # another blowing edge
//!     target/release/panpipes --notes=G3,A3,B3,D4 ...   # other pipes, low to high
//!     target/release/panpipes --end=0.7 ...             # the open end's correction
//!     target/release/panpipes --kerf=0.18 ...           # for a laser with that kerf
//!     target/release/panpipes --page-to=turn.html       # the 3D view, every edge
//!
//! THE RAFT is two plates, the front and the back, with a wall between every
//! pair of pipes and one at each end. An inner wall has FINGER wide tabs,
//! TAB_PITCH apart, through slots in both plates; the two end walls have the
//! square flute's finger joint, a finger every 2 x FINGER, in notches in the
//! plates' long edges, flush with the outside. The tops are flush, the rim
//! the player blows across; the bottoms step down with the pipes, so the
//! plates come out as a staircase.
//!
//! EACH PIPE is a closed pipe, a quarter wave long. From the rim:
//!
//!   air      L = C / 4f - END x a, where a is the radius of the circle with
//!            the bore's area: the column the plug's face stops
//!   plug     PLUG_LAYERS rectangles of ply, press-fit into the bore from the
//!            open bottom, up to the blue line on the front plate
//!   spare    SPARE of empty pipe below the plug, so it can come down and
//!            flatten the note as well as go up and sharpen it
//!
//! The bore is DEPTH from front to back in every pipe, so the plates stay
//! plain, and narrows from W_LOW to W_HIGH across the raft, so the high
//! pipes are not fat for their length.
//!
//! THE BLOWING EDGE is the back plate's top edge, the one the breath strikes
//! after crossing the bore. --edge= picks how it is made:
//!
//!   chamfer  the default. The plate as cut, with a blue line engraved across
//!            its outside CHAMFER_DOWN below the rim: sand a 45 degree bevel
//!            down to it, leaving CHAMFER_LEFT of edge on the inside of the bore
//!   plain    the plate as cut, a 3mm square edge. It plays, breathier
//!   notch    a U notch cut into the rim over every pipe, half the pipe wide,
//!            as a quena's, with a blue line engraved round each one
//!            CHAMFER_DOWN out from it on the outside: file a 45 degree bevel
//!            down to the line, leaving CHAMFER_LEFT of edge
//!   strip    the back plate stops STRIP_H short of the rim, and a strip of
//!            STRIP_T stock, cut from its own sheet, is glued along it with
//!            its inner face flush with the bore, so the edge is STRIP_T thick
//!
//! The front plate's top, where the lower lip rests, is rounded with
//! sandpaper in every build; nothing here draws that.
//!
//! The kerf, the finger joint, the sheet packing and the inks are the square
//! flute's (square-flute/square_flute.rs), the mitred offset the four-hole
//! ocarinas' and the path writer swept-curve's: each checkout is expected
//! beside this one, with lasermade-tools for the 3D page.
#![allow(dead_code)]
use four_hole_ocarinas::offset;
use lasermade_tools::die;
use lasermade_tools::py::{self, g};
use lasermade_tools::turn3d::{self, chain, placed_raw, Mat, Num, Pt, F, I, J};
use ribbon_bore::path;
use square_flute::{self as sf, fingers_x, kerf, pack, Cut, Part, Sheet, Z};

pub const T: f64 = sf::T;
pub const FINGER: f64 = sf::FINGER;
pub const FINGER_END: f64 = sf::FINGER_END;
pub const MIN_WEB: f64 = sf::MIN_WEB;
pub const A4: f64 = 440.0;
pub fn c() -> f64 {
    sf::c() * 1000.0 // mm/s, air warm from the breath, as the flutes'
}

pub const NOTES: [&str; 8] = ["E3", "F#3", "G3", "A3", "B3", "C4", "D4", "E4"];
pub const DEPTH: f64 = 20.0; // bore, front to back, every pipe
pub const W_LOW: f64 = 22.0; // bore across the raft, lowest pipe
pub const W_HIGH: f64 = 16.0; // and highest
pub const END: f64 = 0.61; // open-end correction, x the equivalent radius
pub const PLUG_LAYERS: usize = 3;
pub const INTERF: f64 = sf::INTERF_ACROSS; // the plug's press-fit, each way across
pub const SPARE: f64 = 12.0; // empty pipe below the plug, to flatten into
pub const EDGES: [&str; 4] = ["chamfer", "plain", "notch", "strip"];
pub const CHAMFER_LEFT: f64 = 1.0; // edge left on the inside of the bore
pub const CHAMFER_DOWN: f64 = T - CHAMFER_LEFT; // how far down the outside a 45 degree bevel runs
pub const NOTCH_OF: f64 = 0.5; // notch width, of the pipe's width
pub const NOTCH_PAST: f64 = 1.0; // notch depth past its half-round bottom
pub const STRIP_H: f64 = 5.0;
pub const STRIP_T: f64 = 1.5;
pub const STEPS: usize = 16; // points on a half circle
// The inner walls' tabs, through slots in both plates. The end walls keep
// the square flute's fingers, a finger and a space each 2T, because those
// show; at that pitch an inner wall would carry 22 tabs a side, and 44 tabs
// pressed home at once is a wall that will not go in straight.
pub const TAB_PITCH: f64 = 40.0;
pub const OVER_CLEAR: f64 = 4.0; // wall either side of a tab below a shorter pipe
// The 3D page's credit line: the program that wrote it.
pub const BY: &str = "panpipes.rs";

/// 'F#3' -> Hz, equal tempered from A4.
pub fn freq(note: &str) -> f64 {
    let bad = || -> ! { die(format!("error: {note} is not a note, e.g. E3 or F#3.")) };
    let mut ch = note.chars();
    let first = ch.next().unwrap_or_else(|| bad());
    let mut k: i64 = match first.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => bad(),
    };
    let mut rest = ch.as_str();
    while let Some(c) = rest.chars().next().filter(|c| *c == '#' || *c == 'b') {
        k += if c == '#' { 1 } else { -1 };
        rest = &rest[1..];
    }
    let oct: i64 = rest.trim().parse().unwrap_or_else(|_| bad());
    let midi = 12 * (oct + 1) + k;
    A4 * py::powf(2.0, (midi - 69) as f64 / 12.0)
}

#[derive(Clone, Debug)]
pub struct Pipe {
    pub note: String,
    pub f: f64,
    pub w: f64,
    pub a: f64,
    pub air: f64,
    pub length: f64,
}

/// Each pipe's bore width, air column and cut length, low to high.
pub fn design(notes: &[String], end: f64) -> Vec<Pipe> {
    let fs: Vec<f64> = notes.iter().map(|n| freq(n)).collect();
    if fs.windows(2).any(|w| w[1] <= w[0]) {
        die("error: --notes= runs from the lowest pipe to the highest.");
    }
    let n = notes.len();
    notes
        .iter()
        .zip(&fs)
        .enumerate()
        .map(|(i, (nm, &f))| {
            let t = if n > 1 { i as f64 / (n - 1) as f64 } else { 0.0 };
            let w = W_LOW + (W_HIGH - W_LOW) * t;
            let a = (w * DEPTH / std::f64::consts::PI).sqrt();
            let air = c() / (4.0 * f) - end * a;
            Pipe { note: nm.clone(), f, w, a, air, length: air + PLUG_LAYERS as f64 * T + SPARE }
        })
        .collect()
}

/// Where each wall stands across the raft, and how long it is: wall j is
/// between pipes j-1 and j, as long as the longer of them.
pub fn layout(pipes: &[Pipe]) -> (Vec<f64>, Vec<f64>, f64) {
    let (mut ys, mut y) = (vec![], 0.0);
    for p in pipes {
        ys.push(y);
        y += T + p.w;
    }
    ys.push(y);
    let mut lens = vec![pipes[0].length];
    lens.extend(pipes.iter().map(|p| p.length));
    (ys, lens, y + T)
}

/// Where an inner wall's tabs go along L: FINGER wide, TAB_PITCH apart,
/// centred, FINGER_END clear of each end, as the end walls' fingers are.
pub fn tabs_x(l: f64) -> Vec<f64> {
    let n = ((l - 2.0 * FINGER_END - FINGER) / TAB_PITCH).floor() as i64 + 1;
    let span = (n - 1) as f64 * TAB_PITCH;
    (0..n.max(0)).map(|i| l / 2.0 - span / 2.0 + i as f64 * TAB_PITCH).collect()
}

/// Tabs along the stretch of an inner wall below its shorter pipe, x0 to x1:
/// fingers in notches in the plate's staircase edge. One in the middle, or
/// more at TAB_PITCH if the stretch is long; none if it is too short.
pub fn overhang_x(x0: f64, x1: f64) -> Vec<f64> {
    let l = x1 - x0;
    if l < FINGER + 2.0 * OVER_CLEAR {
        return vec![];
    }
    let n = ((l - 2.0 * OVER_CLEAR - FINGER) / TAB_PITCH).floor() as i64 + 1;
    let span = (n - 1) as f64 * TAB_PITCH;
    (0..n.max(0)).map(|i| x0 + l / 2.0 - span / 2.0 + i as f64 * TAB_PITCH).collect()
}

/// Each wall's tab positions: the end walls' fingers; an inner wall's tabs
/// in slots down its shorter pipe, and in notches below it.
pub fn joints(pipes: &[Pipe], lens: &[f64]) -> Vec<Vec<f64>> {
    let last = lens.len() - 1;
    lens.iter()
        .enumerate()
        .map(|(j, &l)| {
            if j == 0 || j == last {
                fingers_x(l, &[])
            } else {
                let short = pipes[j].length;
                let mut v = tabs_x(short);
                v.extend(overhang_x(short, l));
                v
            }
        })
        .collect()
}

/// A U into the rim (x = 0) centred at yc: straight sides, a half-round
/// bottom, `depth` deep at its centre, from the +y side to the -y side.
pub fn half_round_notch(yc: f64, w: f64, depth: f64) -> Vec<(f64, f64)> {
    let r = w / 2.0;
    let s = depth - r;
    let mut pts = vec![(0.0, yc + r)];
    for k in 0..=STEPS {
        let t = std::f64::consts::PI * k as f64 / STEPS as f64;
        pts.push((s + r * py::msin(t), yc + r * py::mcos(t)));
    }
    pts.push((0.0, yc - r));
    pts
}

/// The bevel's line round a notch: the U's outline moved CHAMFER_DOWN out
/// into the plate, from the rim round to the rim, an open line to engrave.
pub fn notch_bevel(yc: f64, w: f64, depth: f64) -> Vec<(f64, f64)> {
    let r = w / 2.0 + CHAMFER_DOWN;
    let s = depth - w / 2.0;
    let mut pts = vec![(0.0, yc + r)];
    for k in 0..=STEPS {
        let t = std::f64::consts::PI * k as f64 / STEPS as f64;
        pts.push((s + r * py::msin(t), yc + r * py::mcos(t)));
    }
    pts.push((0.0, yc - r));
    pts
}

/// A part of the raft: the flat part, its engraved strokes, and where it
/// goes, (u, v, up-from-the-sheet) into the raft.
#[derive(Clone, Debug)]
pub struct RPart {
    pub p: Part,
    pub strokes: Vec<Vec<(f64, f64)>>,
    pub m: Mat,
    pub plug: bool,
}

pub fn fpts(v: &[(f64, f64)]) -> Vec<Pt> {
    v.iter().map(|&(x, y)| [F(x), F(y)]).collect()
}

/// A plate as seen from its outside face, x down the pipes from the rim, y
/// across the raft from the lowest pipe.
#[allow(clippy::too_many_arguments)]
fn plate(name: &str, num: usize, pipes: &[Pipe], ys: &[f64], lens: &[f64], y_: f64, edge: &str, back: bool) -> (Part, Vec<Vec<(f64, f64)>>) {
    let n = pipes.len();
    let xs = joints(pipes, lens);
    let top_x = if back && edge == "strip" { STRIP_H } else { 0.0 };
    let h = FINGER / 2.0;
    let mut o: Vec<(f64, f64)> = vec![(top_x, 0.0)];
    for &x in &xs[0] {
        o.extend([(x - h, 0.0), (x - h, T), (x + h, T), (x + h, 0.0)]);
    }
    o.push((lens[0], 0.0));
    for j in 1..n {
        o.push((lens[j], ys[j] + T));
        let mut below: Vec<f64> = xs[j].iter().copied().filter(|&x| x > pipes[j].length).collect();
        below.sort_by(|a, b| b.partial_cmp(a).unwrap());
        for x in below {
            o.extend([(x + h, ys[j] + T), (x + h, ys[j]), (x - h, ys[j]), (x - h, ys[j] + T)]);
        }
        o.push((pipes[j].length, ys[j] + T));
    }
    o.push((lens[n], y_));
    for &x in xs[n].iter().rev() {
        o.extend([(x + h, y_), (x + h, y_ - T), (x - h, y_ - T), (x - h, y_)]);
    }
    o.push((top_x, y_));
    if back && edge == "notch" {
        for i in (0..n).rev() {
            let yc = ys[i] + T + pipes[i].w / 2.0;
            let w = NOTCH_OF * pipes[i].w;
            o.extend(half_round_notch(yc, w, w / 2.0 + NOTCH_PAST));
        }
    }
    let o: Vec<(f64, f64)> = o.iter().enumerate().filter(|&(k, q)| k == 0 || *q != o[k - 1]).map(|x| *x.1).collect();
    let mut p = Part::new(name, num, fpts(&o), "");
    let mut strokes = vec![];
    for j in 1..n {
        for &x in xs[j].iter().filter(|&&x| x < pipes[j].length) {
            p.cuts.push(Cut::Poly(sf::rect(F(x - h), F(ys[j]), F(x + h), F(ys[j] + T))));
        }
    }
    if back && edge == "chamfer" {
        strokes.push(vec![(CHAMFER_DOWN, 0.0), (CHAMFER_DOWN, y_)]);
    }
    if back && edge == "notch" {
        for i in 0..n {
            let yc = ys[i] + T + pipes[i].w / 2.0;
            let w = NOTCH_OF * pipes[i].w;
            strokes.push(notch_bevel(yc, w, w / 2.0 + NOTCH_PAST));
        }
    }
    if !back {
        for (i, pp) in pipes.iter().enumerate() {
            strokes.push(vec![(pp.air, ys[i] + T), (pp.air, ys[i + 1])]);
        }
    }
    (p, strokes)
}

/// The part flipped across the raft, y -> Y - y.
fn mirror(p: &Part, strokes: &[Vec<(f64, f64)>], y_: f64) -> (Part, Vec<Vec<(f64, f64)>>) {
    let fl = |v: &[Pt]| -> Vec<Pt> { v.iter().rev().map(|q| [q[0], F(y_) - q[1]]).collect() };
    let mut q = Part::new(&p.name, p.num, fl(&p.outline), &p.note);
    q.cuts = p
        .cuts
        .iter()
        .map(|c| match c {
            Cut::Poly(v) => Cut::Poly(fl(v)),
            Cut::Circle(..) => unreachable!(),
        })
        .collect();
    (q, strokes.iter().map(|s| s.iter().map(|&(x, y)| (x, y_ - y)).collect()).collect())
}

fn row(v: [Num; 4]) -> Vec<Num> {
    v.to_vec()
}

pub struct Info {
    pub ys: Vec<f64>,
    pub lens: Vec<f64>,
    pub y: f64,
    pub xs: Vec<Vec<f64>>,
}

/// Every part, in its flat coordinates, with m taking it into the raft: x
/// down the pipes from the rim, y across from the lowest pipe, z from the
/// front plate's inside face to the back.
pub fn build(pipes: &[Pipe], edge: &str) -> (Vec<RPart>, Vec<RPart>, Info) {
    let (ys, lens, y_) = layout(pipes);
    let n = pipes.len();
    let mut num = 0usize;
    let mut next = || {
        num += 1;
        num
    };
    let mut parts = vec![];
    let (fp, fs) = plate("front plate", next(), pipes, &ys, &lens, y_, edge, false);
    let (mut front, fstrokes) = mirror(&fp, &fs, y_);
    front.note = "engraved face out, toward the player; the blue lines are the plugs' faces; round its top edge where the lip rests".into();
    parts.push(RPart {
        p: front,
        strokes: fstrokes,
        m: vec![row([I(1), I(0), I(0), I(0)]), row([I(0), I(-1), I(0), F(y_)]), row([I(0), I(0), I(-1), I(0)])],
        plug: false,
    });
    let (mut back, bstrokes) = plate("back plate", next(), pipes, &ys, &lens, y_, edge, true);
    back.note = match edge {
        "chamfer" => "engraved face out; sand the rim to a 45 degree bevel down to the blue line".into(),
        "plain" => "its top edge is the blowing edge, as cut".into(),
        "notch" => "engraved face out; file each notch to a 45 degree bevel down to its blue line".into(),
        _ => format!("stops {}mm short of the rim; the strip glues along that edge", g(STRIP_H)),
    };
    parts.push(RPart {
        p: back,
        strokes: bstrokes,
        m: vec![row([I(1), I(0), I(0), I(0)]), row([I(0), I(1), I(0), I(0)]), row([I(0), I(0), I(1), F(DEPTH)])],
        plug: false,
    });
    let js = joints(pipes, &lens);
    for j in 0..=n {
        let mut w = sf::finger_side(lens[j], DEPTH, &js[j], next());
        w.name = if j == 0 || j == n { "end wall" } else { "wall" }.into();
        w.note = "tabs through the slots in both plates".into();
        parts.push(RPart {
            p: w,
            strokes: vec![],
            m: vec![row([I(1), I(0), I(0), I(0)]), row([I(0), I(0), I(-1), F(ys[j] + T)]), row([I(0), I(1), I(0), I(0)])],
            plug: false,
        });
    }
    for (i, pp) in pipes.iter().enumerate() {
        for k in 0..PLUG_LAYERS {
            let gp = Part::new(
                &format!("plug, {}", pp.note),
                next(),
                sf::rect(Z, Z, F(pp.w + INTERF), F(DEPTH + INTERF)),
                &format!("press-fit into the {} pipe from below, up to its blue line", pp.note),
            );
            parts.push(RPart {
                p: gp,
                strokes: vec![],
                m: vec![
                    row([I(0), I(0), I(1), F(pp.air + k as f64 * T)]),
                    row([I(1), I(0), I(0), F(ys[i] + T - INTERF / 2.0)]),
                    row([I(0), I(1), I(0), F(-INTERF / 2.0)]),
                ],
                plug: true,
            });
        }
    }
    let mut strips = vec![];
    if edge == "strip" {
        let s = Part::new(
            "edge strip",
            next(),
            sf::rect(Z, Z, F(y_), F(STRIP_H)),
            &format!("{}mm stock; glued along the back plate's cut-down top, inner face flush with the bore", g(STRIP_T)),
        );
        strips.push(RPart {
            p: s,
            strokes: vec![],
            m: vec![row([I(0), I(1), I(0), I(0)]), row([I(-1), I(0), I(0), F(y_)]), row([I(0), I(0), F(STRIP_T / T), F(DEPTH)])],
            plug: false,
        });
    }
    (parts, strips, Info { ys, lens, y: y_, xs: js })
}

pub fn seg_dist(q: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { py::max(0.0, py::min(1.0, ((q.0 - a.0) * dx + (q.1 - a.1) * dy) / l2)) };
    py::hypot(q.0 - a.0 - t * dx, q.1 - a.1 - t * dy)
}

pub type Checks = Vec<(bool, String, String)>;

fn checks(pipes: &[Pipe], parts: &[RPart], strips: &[RPart], info: &Info, edge: &str, sheets: &[Sheet]) -> Checks {
    let mut out: Checks = vec![];
    let mut note = |ok: bool, what: &str, detail: String| out.push((ok, what.to_string(), detail));
    let (ys, lens) = (&info.ys, &info.lens);
    let n = pipes.len();
    let front = parts.iter().find(|p| p.p.name == "front plate").unwrap();
    let mut marks: Vec<f64> = front.strokes.iter().map(|s| s[0].0).collect();
    marks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut by_air: Vec<&Pipe> = pipes.iter().collect();
    by_air.sort_by(|a, b| a.air.partial_cmp(&b.air).unwrap());
    note(
        marks.len() == n && marks.iter().zip(&by_air).all(|(m, p)| (m - p.air).abs() < 1e-9),
        "every blue line on the front is its plug's face",
        format!("{} lines, {:.1} to {:.1}mm from the rim", marks.len(), marks[0], marks[marks.len() - 1]),
    );
    let bores: Vec<f64> = (0..n).map(|i| ys[i + 1] - ys[i] - T).collect();
    note(
        bores.iter().zip(pipes).all(|(b, p)| (b - p.w).abs() < 1e-9),
        "the walls stand a bore apart",
        format!("{:.1} to {:.1}mm across, {}mm deep", sf::fmin(&bores), sf::fmax(&bores), g(DEPTH)),
    );
    let plugs: Vec<&RPart> = parts.iter().filter(|p| p.plug).collect();
    let each: Vec<&Pipe> = pipes.iter().flat_map(|p| std::iter::repeat(p).take(PLUG_LAYERS)).collect();
    note(
        plugs.len() == PLUG_LAYERS * n
            && plugs.iter().zip(&each).all(|(gp, pp)| (gp.p.w - pp.w - INTERF).abs() < 1e-9 && (gp.p.h - DEPTH - INTERF).abs() < 1e-9),
        "every plug is a press-fit in its bore",
        format!("{} layers, {}mm over each way", plugs.len(), g(INTERF)),
    );
    let room = sf::fmin(&pipes.iter().map(|p| p.length - p.air - PLUG_LAYERS as f64 * T).collect::<Vec<_>>());
    note((room - SPARE).abs() < 1e-9, "every plug can come down as well as go up", format!("{}mm of pipe below each plug", g(room)));
    let lens_ok = (0..=n).all(|j| {
        let lo = j.saturating_sub(1);
        let m = sf::fmax(&pipes[lo..(j + 1).min(n)].iter().map(|p| p.length).collect::<Vec<_>>());
        lens[j] >= m - 1e-9
    });
    note(lens_ok, "every pipe is walled to its bottom", format!("{} walls", n + 1));
    let first = sf::fmin(&info.xs.iter().filter(|xs| !xs.is_empty()).map(|xs| sf::fmin(xs)).collect::<Vec<_>>());
    note(
        first - FINGER / 2.0 >= STRIP_H + MIN_WEB,
        "the rim is clear of every joint",
        format!("the first finger {:.1}mm down, against {}mm of strip", first - FINGER / 2.0, g(STRIP_H)),
    );
    let (mut worst, mut where_) = (f64::INFINITY, String::new());
    for pl in parts.iter().filter(|p| p.p.name.ends_with("plate")) {
        let mut ring = pl.p.of();
        ring.push(ring[0]);
        for c in &pl.p.cuts {
            let Cut::Poly(v) = c else { continue };
            for q in v {
                let q = (q[0].f(), q[1].f());
                let inside = ring
                    .windows(2)
                    .filter(|w| {
                        let (a, b) = (w[0], w[1]);
                        (a.1 > q.1) != (b.1 > q.1) && q.0 < a.0 + (q.1 - a.1) * (b.0 - a.0) / (b.1 - a.1)
                    })
                    .count()
                    % 2;
                let mut d = sf::fmin(&ring.windows(2).map(|w| seg_dist(q, w[0], w[1])).collect::<Vec<_>>());
                if inside == 0 {
                    d = -d;
                }
                if d < worst {
                    worst = d;
                    where_ = pl.p.name.clone();
                }
            }
        }
    }
    note(
        worst >= MIN_WEB,
        "every slot keeps ply between it and the plate's edge",
        format!("{worst:.2}mm at the least, on the {where_}, against {}mm", g(MIN_WEB)),
    );
    for j in 1..n {
        let short = pipes[j].length;
        if info.xs[j].iter().any(|&x| x - FINGER / 2.0 - MIN_WEB < short && short < x + FINGER / 2.0 + MIN_WEB) {
            note(false, "no tab straddles a pipe's bottom", format!("wall {j}"));
        }
    }
    if edge == "notch" {
        let widest = sf::fmax(&pipes.iter().map(|p| NOTCH_OF * p.w).collect::<Vec<_>>());
        let deepest = sf::fmax(&pipes.iter().map(|p| NOTCH_OF * p.w / 2.0 + NOTCH_PAST).collect::<Vec<_>>());
        note(
            !pipes.is_empty() && deepest + MIN_WEB <= first - FINGER / 2.0,
            "every notch stays within its pipe and clear of the joints",
            format!("{widest:.1}mm wide and {deepest:.1}mm deep at the most"),
        );
        let back = parts.iter().find(|q| q.p.name == "back plate").unwrap();
        let lines = &back.strokes;
        let xmax = |s: &Vec<(f64, f64)>| sf::fmax(&s.iter().map(|q| q.0).collect::<Vec<_>>());
        let ysp = |s: &Vec<(f64, f64)>| {
            let v: Vec<f64> = s.iter().map(|q| q.1).collect();
            sf::fmax(&v) - sf::fmin(&v)
        };
        let reach = sf::fmax(&lines.iter().map(xmax).collect::<Vec<_>>());
        let spread = sf::fmax(&lines.iter().map(ysp).collect::<Vec<_>>());
        let over = lines.iter().zip(pipes).filter(|(s, pp)| ysp(s) > pp.w).count();
        note(
            lines.len() == n && over == 0 && reach + MIN_WEB <= first - FINGER / 2.0,
            "every notch has its bevel line, clear of the joints",
            format!(
                "{} lines, {}mm out from each notch, reaching {reach:.1}mm down and {spread:.1}mm across, each within its pipe",
                lines.len(),
                g(CHAMFER_DOWN)
            ),
        );
    }
    if edge == "chamfer" {
        note(
            0.0 < CHAMFER_DOWN && CHAMFER_DOWN < T,
            "the bevel leaves an edge inside the bore",
            format!("{}mm left of {}mm, {}mm down the outside", g(CHAMFER_LEFT), g(T), g(CHAMFER_DOWN)),
        );
    }
    let ratio = sf::fmin(&pipes.iter().map(|p| p.air / (2.0 * p.a)).collect::<Vec<_>>());
    note(ratio >= 8.0, "no pipe is fat for its length", format!("{ratio:.1} diameters at the least"));
    let longest = sf::fmax(&parts.iter().chain(strips).map(|p| py::max(p.p.w, p.p.h)).collect::<Vec<_>>());
    note(
        longest + kerf() + 2.0 * sf::MARGIN <= sf::BED.0,
        "the longest part fits the bed",
        format!("{:.1}mm with the kerf, against {}", longest + kerf(), g(sf::BED.0 - 2.0 * sf::MARGIN)),
    );
    sheet_checks(&mut note, sheets, parts.len() + strips.len());
    out
}

/// The two checks every generator here ends with.
pub fn sheet_checks(note: &mut impl FnMut(bool, &str, String), sheets: &[Sheet], total: usize) {
    note(
        sheets.iter().all(|s| s.w <= sf::BED.0 && s.h <= sf::BED.1),
        "every sheet fits the bed",
        format!(
            "{} sheet(s), largest {:.0} x {:.0}mm",
            sheets.len(),
            sf::fmax(&sheets.iter().map(|s| s.w).collect::<Vec<_>>()),
            sf::fmax(&sheets.iter().map(|s| s.h).collect::<Vec<_>>())
        ),
    );
    let placed: usize = sheets.iter().map(|s| s.parts.len()).sum();
    note(placed == total, "every part is on a sheet", format!("{placed} of {total}"));
}

pub fn group(col: &str, items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut s = format!("  <g fill=\"none\" stroke=\"{col}\" stroke-width=\"0.2\">\n");
    for i in items {
        s += &format!("    <path d=\"{i}\"/>\n");
    }
    s + "  </g>\n"
}

fn svg_sheet(k: usize, n: usize, s: &Sheet, parts: &[&RPart], what: &str, edge: &str, stock: f64) -> String {
    let (mut orange, mut black, mut blue) = (vec![], vec![], vec![]);
    let kf = kerf();
    for &(i, sx, sy) in &s.parts {
        let p = parts[i];
        let (dx, dy) = (sx - p.p.x0 + kf / 2.0, sy - p.p.y0 + kf / 2.0);
        let mv = |pts: &[(f64, f64)]| -> Vec<(f64, f64)> { pts.iter().map(|&(x, y)| (x + dx, y + dy)).collect() };
        black.push(path(&mv(&offset(&p.p.of(), kf / 2.0)), true));
        for c in &p.p.cuts {
            if let Cut::Poly(v) = c {
                let v: Vec<(f64, f64)> = v.iter().map(|q| (q[0].f(), q[1].f())).collect();
                orange.push(path(&mv(&offset(&v, -kf / 2.0)), true));
            }
        }
        for st in &p.strokes {
            blue.push(path(&mv(st), false));
        }
    }
    let eng = match edge {
        "chamfer" => "the plugs' places on the front plate and the bevel's line on the back",
        "notch" => "the plugs' places on the front plate and the bevel's line round every notch on the back",
        _ => "the plugs' places on the front plate",
    };
    let desc = format!(
        "1 user unit = 1mm. Panpipes, E3 to E4, {edge} edge, sheet {k} of {n}: {what}. {}mm ply, drawn for a {}mm kerf. \
         Blue {} engraves {eng}, on the plates' outside faces, which lie face up. Then orange {} cuts the slots, and \
         black {} frees the parts.",
        g(stock),
        g(kf),
        sf::BLUE,
        sf::ORANGE,
        sf::BLACK
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.2}mm\" \
         height=\"{h:.2}mm\" viewBox=\"0 0 {w:.2} {h:.2}\">\n<title>Panpipes E3 to E4, {edge} edge, sheet {k} of {n}</title>\n\
         <desc>{desc}</desc>\n{}{}{}</svg>\n",
        group(sf::BLUE, &blue),
        group(sf::ORANGE, &orange),
        group(sf::BLACK, &black),
        w = s.w,
        h = s.h
    )
}

pub fn up() -> Mat {
    vec![
        row([I(0), I(-1), I(0), I(0)]),
        row([I(0), I(0), I(1), I(0)]),
        row([I(-1), I(0), I(0), I(0)]),
        row([I(0), I(0), I(0), I(1)]),
    ]
}

/// The explode direction in the raft's frame, turned to the viewer's.
pub fn turned(ex: [Num; 3]) -> Vec<Num> {
    vec![-ex[1], ex[2], -ex[0]]
}

pub const PAGE_NOTE_HEAD: &str = "Drag to turn; right-drag, shift-drag or move two fingers to slide the view; \
     scroll or pinch to zoom in on whatever is under the pointer; double-click a part \
     to turn round it, and Reset view to go back. Draw Explode to take it apart. Every part is \
     the cut files’ own outline";

/// The 3D view: every edge as a design of its own, drawn from the same parts
/// the sheets are, plugs at their blue lines.
fn page(path_out: &str, notes: &[String], end: f64) {
    let mut designs: Vec<(String, J)> = vec![];
    for edge in EDGES {
        let pipes = design(notes, end);
        let (parts, strips, _) = build(&pipes, edge);
        let out: Vec<J> = parts
            .iter()
            .chain(&strips)
            .map(|p| {
                let mut m = p.m.clone();
                m.push(row([I(0), I(0), I(0), I(1)]));
                let ex = if p.p.name == "front plate" {
                    [I(0), I(0), F(-1.4)]
                } else if p.p.name == "back plate" {
                    [I(0), I(0), F(1.4)]
                } else if p.plug {
                    [F(1.2), I(0), I(0)]
                } else if p.p.name == "edge strip" {
                    [I(-1), I(0), F(1.6)]
                } else {
                    [I(0), I(0), I(0)]
                };
                let wood = if p.plug || p.p.name == "edge strip" { "dark" } else { "ply" };
                let h: Vec<Vec<Pt>> = p.p.cuts.iter().map(|c| if let Cut::Poly(v) = c { v.clone() } else { unreachable!() }).collect();
                let e: Vec<Vec<Pt>> = p.strokes.iter().map(|s| fpts(s)).collect();
                placed_raw(&p.p.outline, &h, &e, &chain(&[up(), m]), &turned(ex), wood)
            })
            .collect();
        designs.push((
            format!("{edge} edge"),
            J::obj(vec![("parts", J::A(out)), ("ex", J::N(I(30))), ("view", turn3d::nums(&[F(-0.6), F(-0.3), F(4.2)]))]),
        ));
    }
    turn3d::write(
        path_out,
        "Panpipes in 3D",
        &mut designs,
        &format!(
            "{PAGE_NOTE_HEAD}: the front plate toward you, the back plate behind \
             with its blowing edge along the top, the walls between the pipes, and the plugs, \
             darker, at their blue lines. The buttons switch between the four blowing edges."
        ),
        BY,
        true,
    );
}

pub fn basename(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

pub fn print_checks(res: &Checks, width: usize) -> usize {
    let mut bad = 0;
    for (ok, what, detail) in res {
        println!("  {}  {:<width$} {}", if *ok { "pass" } else { "FAIL" }, what, detail);
        bad += !*ok as usize;
    }
    if bad > 0 {
        println!("\n  {bad} check(s) failed. Nothing written.");
    }
    bad
}

pub fn write_file(path: &str, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|e| die(format!("{path}: {e}")));
}

pub fn mkdirs(dir: &str) {
    std::fs::create_dir_all(dir).unwrap_or_else(|e| die(format!("{dir}: {e}")));
}

pub fn join(dir: &str, f: &str) -> String {
    if dir.is_empty() || dir.ends_with('/') {
        format!("{dir}{f}")
    } else {
        format!("{dir}/{f}")
    }
}

fn main() {
    let (mut out, mut write, mut edge, mut page_to) = (None::<String>, true, "chamfer".to_string(), None::<String>);
    let mut notes: Vec<String> = NOTES.iter().map(|s| s.to_string()).collect();
    let mut end = END;
    for x in std::env::args().skip(1) {
        let val = || x.split_once('=').unwrap().1.to_string();
        if x == "--no-write" {
            write = false;
        } else if x.starts_with("--out=") {
            out = Some(val());
        } else if x.starts_with("--edge=") {
            edge = val();
            if !EDGES.contains(&edge.as_str()) {
                die(format!("error: --edge={edge}: the edges are {}.", EDGES.join(", ")));
            }
        } else if x.starts_with("--notes=") {
            notes = val().split(',').filter(|s| !s.is_empty()).map(String::from).collect();
        } else if x.starts_with("--end=") {
            end = py::float(&val()).unwrap_or_else(|| die(format!("error: {x}: not a number.")));
        } else if x.starts_with("--kerf=") {
            sf::set_kerf(&x);
        } else if x.starts_with("--page-to=") {
            page_to = Some(val());
        } else {
            die(format!(
                "error: {x} is not a flag this generator reads. It takes --out=, --edge=, --notes=, --end=, --kerf=, --page-to= and --no-write."
            ));
        }
    }
    if let Some(pt) = page_to {
        page(&pt, &notes, end);
        println!("  wrote {}", basename(&pt));
        return;
    }
    if write && out.is_none() {
        die("error: --out=DIR, or --no-write to run the checks only.");
    }
    let pipes = design(&notes, end);
    let (parts, strips, info) = build(&pipes, &edge);
    let sizes = |v: &[RPart]| v.iter().map(|p| (p.p.w, p.p.h)).collect::<Vec<_>>();
    let sheets = pack(&sizes(&parts));
    let s_sheets = if strips.is_empty() { vec![] } else { pack(&sizes(&strips)) };
    let all_sheets: Vec<Sheet> = sheets.iter().chain(&s_sheets).cloned().collect();
    let res = checks(&pipes, &parts, &strips, &info, &edge, &all_sheets);
    println!(
        "panpipes, {} to {}, {edge} edge: {} closed pipes, {}mm deep, the raft {:.1}mm across",
        notes[0],
        notes[notes.len() - 1],
        pipes.len(),
        g(DEPTH),
        info.y
    );
    for p in &pipes {
        println!(
            "  {:<4} {:7.2}Hz  {:5.2} x {}mm  plug face {:6.1}mm from the rim, pipe {:6.1}mm",
            p.note,
            p.f,
            p.w,
            g(DEPTH),
            p.air,
            p.length
        );
    }
    if print_checks(&res, 56) > 0 {
        std::process::exit(1);
    }
    if write {
        let out = out.unwrap();
        mkdirs(&out);
        let stem = format!("panpipes-{}-{edge}", notes[0].replace('#', "s"));
        let pr: Vec<&RPart> = parts.iter().collect();
        let sr: Vec<&RPart> = strips.iter().collect();
        let mut runs: Vec<(&Vec<Sheet>, &Vec<&RPart>, String, f64)> = vec![(&sheets, &pr, "sheet".into(), T)];
        if !strips.is_empty() {
            runs.push((&s_sheets, &sr, format!("{}mm-sheet", g(STRIP_T)), STRIP_T));
        }
        for (grp, ps, label, stock) in runs {
            for (k, s) in grp.iter().enumerate() {
                let mut names: Vec<String> = s.parts.iter().map(|&(i, _, _)| ps[i].p.name.split(',').next().unwrap().to_string()).collect();
                names.sort();
                names.dedup();
                let what = names.join(", ");
                let f = join(&out, &format!("{stem}-{label}{}-cut-files.svg", k + 1));
                write_file(&f, &svg_sheet(k + 1, grp.len(), s, ps, &what, &edge, stock));
                println!("  wrote {}  {:.0} x {:.0}mm, {} parts", basename(&f), s.w, s.h, s.parts.len());
            }
        }
    }
}
