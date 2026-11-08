//! Cut files for the same low panpipes, E3 to E4, as separate tubes glued
//! side to side: octagonal in a row or in two rows, hexagonal in a zigzag,
//! or hexagonal in a row.
//!
//!     target/release/tube_panpipes --shape=octagon-row --no-write      # the checks
//!     target/release/tube_panpipes --shape=hex-zigzag --out=DIR        # the sheets
//!     target/release/tube_panpipes --shape=hex-row --edge=plain ...    # another edge
//!     target/release/tube_panpipes --kerf=0.18 ...                     # for that kerf
//!     target/release/tube_panpipes --page-to=turn-tubes.html           # the 3D view
//!     target/release/tube_panpipes --shape=hex-row --solidcraft=DIR    # the parametric SolidCraft design
//!
//! EACH PIPE is a tube of N strips of 3mm ply, finger-jointed edge to edge by
//! the four-hole ocarinas' polyhedron joint, as the octagonal flute's tube
//! is: open at the rim, open at the bottom, a plug of PLUG_LAYERS polygons
//! pressed up from below to a blue line on its front, or on its back where
//! the front is glued to another tube. The acoustics are panpipes.rs's: the
//! same notes, the same air columns for the same bore areas, AREA_LOW at E3
//! falling to AREA_HIGH at E4, the raft's. The tubes are press-fit and
//! sealed; NEIGHBOURS ARE GLUED, face to face, since separate tubes share no
//! wall.
//!
//! THE SHAPES, the y axis across the set, z from the player to the back:
//!
//!   octagon-row  octagons in a line, a flat to the front for the lip, a flat
//!                to the back for the blowing edge, and flats either side,
//!                glued to the neighbours
//!   octagon-two-rows
//!                a siku's layout: two rows of octagons, each glued flat to
//!                flat in a line, the back row glued behind the front one,
//!                back face to front face, each pipe behind the note below
//!                it; the scale alternates between the rows, the rims level
//!   hex-zigzag   hexagons with a flat to the front and to the back, each pipe
//!                glued to the next on a diagonal face, so they alternate
//!                between a front row and a back row half a pipe behind it;
//!                the rims are level unless --raise= stands the back row taller
//!   hex-row      hexagons with flats either side, glued in a line; a corner
//!                faces the lip and the blowing edge is a V of two faces
//!
//! THE BLOWING EDGE (--edge=) is the far side of the rim, as on the raft:
//!
//!   chamfer  the default: a blue line engraved CHAMFER_DOWN below the rim on
//!            the outside of the back face (both back faces on hex-row); sand
//!            a 45 degree bevel down to it. On octagon-two-rows the front
//!            row's back faces are glued to the back row, so they have no
//!            line: the bevel is sanded across the joint, judged from the rim
//!   plain    the rim as cut
//!   notch    a U cut into the back face's rim with a bevel line round it, the
//!            raft's quena notch, narrower to fit one face; not on hex-row,
//!            which has a corner there, nor on octagon-two-rows, whose front
//!            row's back faces are glued to the back row
#[path = "panpipes.rs"]
mod pp;
mod tubes_solidcraft;

use four_hole_ocarinas::offset;
use four_hole_ocarinas::polyhedron::{self as ph, Edge, Face, V3};
use lasermade_tools::die;
use lasermade_tools::py::{self, g};
use lasermade_tools::turn3d::{self, chain, placed_raw, Mat, Num, Pt, F, I, J};
use pp::{fpts, group, print_checks, sheet_checks, Checks};
use ribbon_bore::path;
use square_flute::{self as sf, kerf, pack, Part, Sheet};
use std::collections::BTreeSet;
use std::f64::consts::PI;

const T: f64 = sf::T;
const MIN_WEB: f64 = sf::MIN_WEB;
const INTERF: f64 = sf::INTERF_ACROSS;
const AREA_LOW: f64 = pp::W_LOW * pp::DEPTH; // 440mm^2, the raft's lowest pipe
const AREA_HIGH: f64 = pp::W_HIGH * pp::DEPTH; // 320mm^2, its highest

/// How the pipes are glued: across a face at these angles (alternating for
/// the zigzag), or in two rows, which centres() lays out itself.
#[derive(Clone, Copy, PartialEq)]
enum Glue {
    Faces(&'static [f64]),
    Rows,
}
// sides, the first face's angle from the front, and the glue, in degrees
const SHAPES: [(&str, usize, f64, Glue); 4] = [
    ("octagon-row", 8, 0.0, Glue::Faces(&[90.0])),
    ("octagon-two-rows", 8, 0.0, Glue::Rows),
    ("hex-zigzag", 6, 0.0, Glue::Faces(&[120.0, 60.0])),
    ("hex-row", 6, 30.0, Glue::Faces(&[90.0])),
];
const EDGES: [&str; 3] = ["chamfer", "plain", "notch"];
const NOTCH_OF: f64 = 0.6; // notch width, of the back face's inside width
const RIM_OWN: f64 = 3.0 * ph::FINGER; // the back face owns its joints this far down
const LINE_IN: f64 = 0.5; // an engraved line's least distance in from a face's edge
// The least flat face two glued tubes share, clear of both tubes' joints. It
// runs the length of the shorter tube, so even the shortest, 277mm, has well
// over 1000mm^2 of glue at 5mm. Two rows share less than one, since each
// back pipe is narrower than the front one it sits behind.
const GLUE_MIN: f64 = 5.0;
// How much taller the zigzag's back row stands at the rim: level, as a
// siku's rows are, by default; --raise= stands the back row taller.
const RAISE: f64 = 0.0;
const BY: &str = "tube_panpipes.rs";

fn shape_of(name: &str) -> (usize, f64, Glue) {
    let s = SHAPES.iter().find(|s| s.0 == name).unwrap();
    (s.1, s.2, s.3)
}

/// Python's float %: the sign of the divisor.
fn pymod(a: f64, b: f64) -> f64 {
    let m = a % b;
    if m != 0.0 {
        if (b < 0.0) != (m < 0.0) {
            m + b
        } else {
            m
        }
    } else {
        0.0f64.copysign(b)
    }
}

/// A face's outward normal, theta degrees round from the front.
fn normal(theta: f64) -> V3 {
    let t = py::radians(theta);
    [0.0, py::msin(t), -py::mcos(t)]
}

fn tangent(theta: f64) -> V3 {
    let t = py::radians(theta);
    [0.0, py::mcos(t), py::msin(t)]
}

#[derive(Clone, Debug)]
struct Pipe {
    note: String,
    f: f64,
    a_: f64,
    fl: f64, // across the flats, inside
    fo: f64, // and outside
    a: f64,
    air: f64,
    rim: f64,
    length: f64,
}

fn design(notes: &[String], shape: &str, end: f64, raise: f64) -> Vec<Pipe> {
    let (n, _, _) = shape_of(shape);
    let k = n as f64 * py::mtan(PI / n as f64) / 4.0; // area = k x flats^2
    if notes.len() < 2 {
        die("error: --notes= names at least two pipes: a tube set is pipes glued to each other.");
    }
    let fs: Vec<f64> = notes.iter().map(|x| pp::freq(x)).collect();
    if fs.windows(2).any(|w| w[1] <= w[0]) {
        die("error: --notes= runs from the lowest pipe to the highest.");
    }
    notes
        .iter()
        .zip(&fs)
        .enumerate()
        .map(|(i, (nm, &f))| {
            let t = if notes.len() > 1 { i as f64 / (notes.len() - 1) as f64 } else { 0.0 };
            let a_ = AREA_LOW + (AREA_HIGH - AREA_LOW) * t;
            let fl = (a_ / k).sqrt();
            let a = (a_ / PI).sqrt();
            let air = pp::c() / (4.0 * f) - end * a;
            let rim = if shape == "hex-zigzag" && i % 2 == 1 { -raise } else { 0.0 };
            Pipe { note: nm.clone(), f, a_, fl, fo: fl + 2.0 * T, a, air, rim, length: air + pp::PLUG_LAYERS as f64 * T + pp::SPARE }
        })
        .collect()
}

type Yz = (f64, f64);

/// Pipe j's axis, placed against pipe i across i's face at theta.
fn against(pipes: &[Pipe], cs: &[Option<Yz>], i: usize, j: usize, theta: f64) -> Yz {
    let nv = normal(theta);
    let d = (pipes[i].fo + pipes[j].fo) / 2.0;
    let c = cs[i].unwrap();
    (c.0 + d * nv[1], c.1 + d * nv[2])
}

/// Each tube's axis, (y, z), and the glued pairs, (i, j, theta): j glued to
/// i across i's face at theta.
fn centres(pipes: &[Pipe], shape: &str) -> (Vec<Yz>, Vec<(usize, usize, f64)>) {
    let (_, _, glue) = shape_of(shape);
    let mut cs: Vec<Option<Yz>> = vec![None; pipes.len()];
    cs[0] = Some((0.0, 0.0));
    let mut pairs = vec![];
    match glue {
        Glue::Rows => {
            for i in 1..pipes.len() {
                if i % 2 == 1 {
                    pairs.push((i - 1, i, 180.0));
                }
                if i >= 2 {
                    pairs.push((i - 2, i, 90.0));
                }
                let (k, th) = if i % 2 == 1 { (i - 1, 180.0) } else { (i - 2, 90.0) };
                cs[i] = Some(against(pipes, &cs, k, i, th));
                if i % 2 == 1 && i >= 3 {
                    let y = against(pipes, &cs, i - 2, i, 90.0).0;
                    cs[i] = Some((y, cs[i].unwrap().1));
                }
            }
            let drift: Vec<f64> = (1..pipes.len()).step_by(2).map(|i| cs[i].unwrap().0 - cs[i - 1].unwrap().0).collect();
            let mid = (sf::fmin(&drift) + sf::fmax(&drift)) / 2.0;
            for i in (1..pipes.len()).step_by(2) {
                let c = cs[i].unwrap();
                cs[i] = Some((c.0 - mid, c.1));
            }
        }
        Glue::Faces(gl) => {
            for i in 1..pipes.len() {
                let th = gl[(i - 1) % gl.len()];
                pairs.push((i - 1, i, th));
                cs[i] = Some(against(pipes, &cs, i - 1, i, th));
            }
        }
    }
    (cs.into_iter().map(Option::unwrap).collect(), pairs)
}

/// The tube's faces: the axis x, from the rim at 0 to the bottom.
fn tube(n: usize, phi: f64, fo: f64, length: f64) -> Vec<Face> {
    let r = fo / 2.0 / py::mcos(PI / n as f64);
    let ring: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let t = py::radians(phi + (k as f64 + 0.5) * 360.0 / n as f64);
            (r * py::msin(t), -r * py::mcos(t))
        })
        .collect();
    let verts: Vec<V3> = ring.iter().flat_map(|&(y, z)| [[0.0, y, z], [length, y, z]]).collect();
    let mut planes: Vec<(String, V3, f64)> = vec![("rim".into(), [-1.0, 0.0, 0.0], 0.0), ("bottom".into(), [1.0, 0.0, 0.0], length)];
    for k in 0..n {
        planes.push((format!("face {k}"), normal(phi + (k * 360) as f64 / n as f64), fo / 2.0));
    }
    ph::solid(&planes, &verts)
}

/// The faces whose normals point most nearly along `toward` (+1 back, -1
/// front): one for a flat, two for a corner.
fn facing(n: usize, phi: f64, toward: f64) -> Vec<usize> {
    let zs: Vec<f64> = (0..n).map(|k| normal(phi + (k * 360) as f64 / n as f64)[2] * toward).collect();
    let best = sf::fmax(&zs);
    (0..n).filter(|&k| zs[k] > best - 1e-6).collect()
}

#[derive(Clone, Debug)]
struct TPart {
    p: Part,
    strokes: Vec<Vec<(f64, f64)>>,
    pipe: usize,
    m: Mat,
    plug: bool,
    plug_mark: bool,
    theta: f64,
    notch: Option<(f64, f64, f64, f64)>,
    edge_back: bool,
}

struct TubeInfo {
    end_poly: Vec<V3>,
    face_out: f64,
    back: Vec<usize>,
}

#[allow(clippy::too_many_arguments)]
fn build_tube(p: &Pipe, idx: usize, shape: &str, edge: &str, c: Yz, num: &mut usize, back_glued: bool, front_glued: bool) -> (Vec<TPart>, TubeInfo) {
    let (n, phi, _) = shape_of(shape);
    let faces = tube(n, phi, p.fo, p.length);
    let ends: Vec<usize> = (0..faces.len()).filter(|&i| faces[i].name == "rim" || faces[i].name == "bottom").collect();
    let walls: Vec<usize> = (0..faces.len()).filter(|i| !ends.contains(i)).collect();
    let back: Vec<usize> = facing(n, phi, 1.0).iter().map(|&k| walls[k]).collect();
    let front: Vec<usize> = facing(n, phi, -1.0).iter().map(|&k| walls[k]).collect();
    let owner_at = |v: V3| *ends.iter().find(|&&i| (ph::dot(v, faces[i].n) - faces[i].off).abs() < 1e-6).unwrap();
    // The ends are open, so a face owns all of its ends; a notched back face
    // owns both its edges near the rim, to keep its width round the U.
    let force = |e: &Edge, s0: f64, s1: f64, o: usize| -> usize {
        if ends.contains(&e.a) || ends.contains(&e.b) {
            return if ends.contains(&e.a) { e.b } else { e.a };
        }
        if edge == "notch" && (back[0] == e.a || back[0] == e.b) {
            let at = |s: f64| e.p[0] + s * (e.q[0] - e.p[0]) / e.l;
            if py::min(at(s0), at(s1)) < RIM_OWN {
                return back[0];
            }
        }
        o
    };
    let es = ph::edges(&faces);
    let cellmap: Vec<Vec<(f64, f64, usize)>> = es.iter().map(|e| ph::cells(e, owner_at, Some(&force))).collect();
    let mut parts = vec![];
    let face_in = p.fl * py::mtan(PI / n as f64);
    let face_out = p.fo * py::mtan(PI / n as f64);
    for &i in &walls {
        let f = &faces[i];
        let o_ = ph::mul(f.n, f.off);
        let fr = ph::frame(f, Some([1.0, 0.0, 0.0]), Some(o_));
        let o = ph::outline(i, f, &fr, &es, &cellmap);
        *num += 1;
        let mut part = Part::new(&format!("{} wall", p.note), *num, fpts(&o), "");
        let mut strokes = vec![];
        let theta = py::degrees(py::matan2(f.n[1], -f.n[2]));
        let on_face = |x: f64, s: f64| {
            let q = ph::add(ph::add([x, 0.0, 0.0], ph::mul(f.n, f.off)), ph::mul(tangent(theta), s));
            ph::to2d(&fr, q)
        };
        let mut plug_mark = false;
        if i == (if front_glued { back[0] } else { front[0] }) {
            // the plug's line, on the front unless that is glued out of sight
            strokes.push(vec![on_face(p.air, -face_in / 2.0), on_face(p.air, face_in / 2.0)]);
            plug_mark = true;
        }
        if back.contains(&i) && edge == "chamfer" && !back_glued {
            strokes.push(vec![on_face(pp::CHAMFER_DOWN, -face_out / 2.0 + 1.0), on_face(pp::CHAMFER_DOWN, face_out / 2.0 - 1.0)]);
        }
        let mut notch = None;
        if i == back[0] && edge == "notch" {
            let w = NOTCH_OF * face_in;
            let depth = w / 2.0 + pp::NOTCH_PAST;
            let vc = on_face(0.0, 0.0).1;
            let rim: Vec<usize> = (0..o.len()).filter(|&k| o[k].0.abs() < 1e-6).collect();
            let k = *rim.iter().find(|&&k| rim.contains(&((k + 1) % o.len()))).unwrap();
            let (a, b) = (o[k], o[(k + 1) % o.len()]);
            let mut u_ = pp::half_round_notch(vc, w, depth);
            if a.1 < b.1 {
                u_.reverse();
            }
            let mut no = o[..k + 1].to_vec();
            no.extend(u_);
            no.extend_from_slice(&o[k + 1..]);
            part.outline = fpts(&no);
            strokes.push(pp::notch_bevel(vc, w, depth));
            notch = Some((w, depth, face_in, face_out));
        }
        let mw = ph::matrix(&fr);
        let m = vec![
            vec![F(mw[0][0]), F(mw[0][1]), F(mw[0][2]), F(mw[0][3] + p.rim)],
            vec![F(mw[1][0]), F(mw[1][1]), F(mw[1][2]), F(mw[1][3] + c.0)],
            vec![F(mw[2][0]), F(mw[2][1]), F(mw[2][2]), F(mw[2][3] + c.1)],
        ];
        part.bounds();
        parts.push(TPart {
            p: part,
            strokes,
            pipe: idx,
            m,
            plug: false,
            plug_mark,
            theta: pymod(theta, 360.0),
            notch,
            edge_back: back.contains(&i),
        });
    }
    let rp = (p.fl + INTERF) / 2.0 / py::mcos(PI / n as f64);
    let disc: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let t = py::radians(phi + (k as f64 + 0.5) * 360.0 / n as f64);
            (rp * py::msin(t), -rp * py::mcos(t))
        })
        .collect();
    for k in 0..pp::PLUG_LAYERS {
        *num += 1;
        let gp = Part::new(
            &format!("plug, {}", p.note),
            *num,
            fpts(&disc),
            &format!("press-fit into the {} pipe from below, up to its blue line", p.note),
        );
        parts.push(TPart {
            p: gp,
            strokes: vec![],
            pipe: idx,
            m: vec![
                vec![I(0), I(0), I(1), F(p.rim + p.air + k as f64 * T)],
                vec![I(1), I(0), I(0), F(c.0)],
                vec![I(0), I(1), I(0), F(c.1)],
            ],
            plug: true,
            plug_mark: false,
            theta: 0.0,
            notch: None,
            edge_back: false,
        });
    }
    let (_, polys) = ph::inner(&faces);
    (parts, TubeInfo { end_poly: polys[ends[0]].clone(), face_out, back })
}

struct Info {
    centres: Vec<Yz>,
    pairs: Vec<(usize, usize, f64)>,
    glued_back: BTreeSet<usize>,
    tubes: Vec<TubeInfo>,
}

fn build(pipes: &[Pipe], shape: &str, edge: &str) -> (Vec<TPart>, Info) {
    let (n, phi, glue) = shape_of(shape);
    if edge == "notch" && facing(n, phi, 1.0).len() > 1 {
        die(format!(
            "error: --shape={shape} has a corner at the back, and a notch cannot be cut into a corner. Its edges are chamfer and plain."
        ));
    }
    if edge == "notch" && glue == Glue::Rows {
        die(format!(
            "error: --shape={shape} glues the front row's back faces to the back row, so they have no free rim to notch. Its edges are chamfer and plain."
        ));
    }
    let (cs, pairs) = centres(pipes, shape);
    let glued_back: BTreeSet<usize> = pairs.iter().filter(|p| p.2 == 180.0).map(|p| p.0).collect();
    let glued_front: BTreeSet<usize> = pairs.iter().filter(|p| p.2 == 180.0).map(|p| p.1).collect();
    let mut num = 0;
    let (mut parts, mut infos) = (vec![], vec![]);
    for (i, (p, &c)) in pipes.iter().zip(&cs).enumerate() {
        let (ps, info) = build_tube(p, i, shape, edge, c, &mut num, glued_back.contains(&i), glued_front.contains(&i));
        parts.extend(ps);
        infos.push(info);
    }
    (parts, Info { centres: cs, pairs, glued_back, tubes: infos })
}

/// Each tube's outside, its corners in (y, z).
fn outsides(pipes: &[Pipe], cs: &[Yz], shape: &str) -> Vec<Vec<(f64, f64)>> {
    let (n, phi, _) = shape_of(shape);
    pipes
        .iter()
        .zip(cs)
        .map(|(p, c)| {
            let r = p.fo / 2.0 / py::mcos(PI / n as f64);
            (0..n)
                .map(|k| {
                    let a = py::radians(phi + (k as f64 + 0.5) * 360.0 / n as f64);
                    (c.0 + r * py::msin(a), c.1 - r * py::mcos(a))
                })
                .collect()
        })
        .collect()
}

fn poly_area(pts: &[(f64, f64)]) -> f64 {
    let n = pts.len();
    let mut s = 0.0;
    for i in 0..n {
        let ((x0, y0), (x1, y1)) = (pts[i], pts[(i + 1) % n]);
        s += x0 * y1 - x1 * y0;
    }
    s.abs() / 2.0
}

fn checks(pipes: &[Pipe], parts: &[TPart], info: &Info, shape: &str, edge: &str, sheets: &[Sheet]) -> Checks {
    let mut out: Checks = vec![];
    let mut note = |ok: bool, what: &str, detail: String| out.push((ok, what.to_string(), detail));
    let (n, phi, _) = shape_of(shape);
    let tubes = &info.tubes;
    let fronts: Vec<&TPart> = parts.iter().filter(|p| p.plug_mark).collect();
    let mut marks: Vec<(usize, f64)> = fronts.iter().map(|p| (p.pipe, p.strokes[0][0].0)).collect();
    marks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut glued: std::collections::HashMap<usize, Vec<f64>> = Default::default();
    for &(i, j, th) in &info.pairs {
        glued.entry(i).or_default().push(pymod(th, 360.0));
        glued.entry(j).or_default().push(pymod(th + 180.0, 360.0));
    }
    let hidden = fronts.iter().filter(|p| glued.get(&p.pipe).map(|v| v.iter().any(|g| (p.theta - g).abs() < 1e-6)).unwrap_or(false)).count();
    let on_back = fronts.iter().filter(|p| (p.theta - 180.0).abs() < 1e-6).count();
    let us: Vec<f64> = marks.iter().map(|m| m.1).collect();
    note(
        marks.len() == pipes.len() && hidden == 0 && marks.iter().all(|&(i, u)| (u - pipes[i].air).abs() < 1e-6),
        "every blue line is its plug's face, on a face left unglued",
        format!("{} lines, {:.1} to {:.1}mm from the rim", marks.len(), sf::fmin(&us), sf::fmax(&us))
            + &if on_back > 0 { format!(", {on_back} on the back, their fronts glued") } else { String::new() },
    );
    let mut worst = 0.0;
    for (t, p) in tubes.iter().zip(pipes) {
        let mut sec: Vec<(f64, f64)> = t.end_poly.iter().map(|q| (q[1], q[2])).collect();
        let mut keyed: Vec<(f64, (f64, f64))> = sec.iter().map(|&q| (py::matan2(q.1, q.0), q)).collect();
        keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        sec = keyed.into_iter().map(|x| x.1).collect();
        let a = poly_area(&sec);
        worst = py::max(worst, (a / p.a_ - 1.0).abs());
    }
    let (p0, pl) = (&pipes[0], &pipes[pipes.len() - 1]);
    note(
        worst < 1e-6,
        "every bore is the area the pipe is tuned for",
        format!("{:.0} to {:.0}mm^2, {:.2} to {:.2}mm across the flats", p0.a_, pl.a_, p0.fl, pl.fl),
    );
    let plugs: Vec<&TPart> = parts.iter().filter(|p| p.plug).collect();
    let nv = normal(phi + 0.0);
    let sizes: Vec<f64> = plugs
        .iter()
        .map(|gp| {
            let fmax = sf::fmax(&gp.p.outline.iter().map(|q| ph::dot([0.0, q[0].f(), q[1].f()], nv).abs()).collect::<Vec<_>>());
            2.0 * fmax - pipes[gp.pipe].fl
        })
        .collect();
    note(
        plugs.len() == pp::PLUG_LAYERS * pipes.len() && sizes.iter().all(|s| (s - INTERF).abs() < 1e-6),
        "every plug is a press-fit in its bore",
        format!("{} layers, {}mm over across the flats", plugs.len(), g(INTERF)),
    );
    let room = sf::fmin(&pipes.iter().map(|p| p.length - p.air - pp::PLUG_LAYERS as f64 * T).collect::<Vec<_>>());
    note((room - pp::SPARE).abs() < 1e-9, "every plug can come down as well as go up", format!("{}mm of pipe below each plug", g(room)));
    let (cs, pairs) = (&info.centres, &info.pairs);
    let (mut share, mut plane) = (f64::INFINITY, 0.0);
    let cl = ph::clear(180.0 - 360.0 / n as f64);
    for &(i, j, th) in pairs {
        let (nv, tv) = (normal(th), tangent(th));
        let a = cs[i].0 * nv[1] + cs[i].1 * nv[2] + pipes[i].fo / 2.0;
        let b = cs[j].0 * nv[1] + cs[j].1 * nv[2] - pipes[j].fo / 2.0;
        plane = py::max(plane, (a - b).abs());
        let span: Vec<(f64, f64)> = [i, j]
            .iter()
            .map(|&k| {
                let m = cs[k].0 * tv[1] + cs[k].1 * tv[2];
                let h = tubes[k].face_out / 2.0 - cl;
                (m - h, m + h)
            })
            .collect();
        share = py::min(share, py::min(span[0].1, span[1].1) - py::max(span[0].0, span[1].0));
    }
    note(
        pairs.len() + 1 >= pipes.len() && plane < 1e-6 && share >= GLUE_MIN,
        "every glued face lies flat on its neighbour's",
        format!("{} glue joints, at least {share:.1}mm wide of solid face, against {}", pairs.len(), g(GLUE_MIN)),
    );
    let rings = outsides(pipes, cs, shape);
    // separating-axis distance between two convex rings: > 0 apart, 0
    // touching, < 0 overlapping
    let gap = |a: &[(f64, f64)], b: &[(f64, f64)]| {
        let mut best = f64::NEG_INFINITY;
        for ring in [a, b] {
            let k = ring.len();
            for e in 0..k {
                let ((x0, y0), (x1, y1)) = (ring[e], ring[(e + 1) % k]);
                let l = py::hypot(x1 - x0, y1 - y0);
                let (nx, ny) = ((y1 - y0) / l, -(x1 - x0) / l);
                let pa: Vec<f64> = a.iter().map(|&(x, y)| x * nx + y * ny).collect();
                let pb: Vec<f64> = b.iter().map(|&(x, y)| x * nx + y * ny).collect();
                best = py::max(py::max(best, sf::fmin(&pb) - sf::fmax(&pa)), sf::fmin(&pa) - sf::fmax(&pb));
            }
        }
        best
    };
    let mut gaps = vec![];
    for i in 0..rings.len() {
        for j in i + 1..rings.len() {
            gaps.push(gap(&rings[i], &rings[j]));
        }
    }
    let worst = sf::fmin(&gaps);
    note(
        worst > -1e-6,
        "no two tubes overlap",
        format!("{} pairs, ", rings.len() * (rings.len() - 1) / 2)
            + &if worst > -1e-6 { "neighbours touching, none overlapping".to_string() } else { format!("two overlap by {:.2}mm", -worst) },
    );
    let web = sf::fmin(&tubes.iter().map(|t| t.face_out).collect::<Vec<_>>()) - 2.0 * ph::clear(180.0 - 360.0 / n as f64);
    note(
        web >= 2.0 * MIN_WEB,
        "a face keeps ply between its two joints",
        format!("{web:.2}mm where both neighbours own the edge, at {} degrees", g(180.0 - 360.0 / n as f64)),
    );
    if edge == "notch" {
        let nts: Vec<(f64, f64, f64, f64)> = parts.iter().filter_map(|p| p.notch).collect();
        let ok = nts.iter().all(|&(w, d, fi, fo)| {
            w + 2.0 * MIN_WEB <= fi && w + 2.0 * pp::CHAMFER_DOWN <= fo - 2.0 * LINE_IN && d + pp::CHAMFER_DOWN + MIN_WEB <= RIM_OWN
        });
        let wmax = sf::fmax(&nts.iter().map(|x| x.0).collect::<Vec<_>>());
        let dmax = sf::fmax(&nts.iter().map(|x| x.1).collect::<Vec<_>>());
        note(
            nts.len() == pipes.len() && ok,
            "every notch and its bevel line fit the back face",
            format!("{wmax:.1}mm wide, {dmax:.1}mm deep, the line {:.1}mm across", wmax + 2.0 * pp::CHAMFER_DOWN),
        );
    }
    if edge == "chamfer" {
        let lines: usize = parts.iter().filter(|p| p.edge_back).map(|p| p.strokes.len() - p.plug_mark as usize).sum();
        let free = pipes.len() - info.glued_back.len();
        note(
            lines == free * tubes[0].back.len(),
            "every free back face has its bevel line",
            format!("{lines} lines, {}mm below the rim", g(pp::CHAMFER_DOWN))
                + &if info.glued_back.is_empty() {
                    String::new()
                } else {
                    format!("; {} back faces glued to the back row, sanded across the joint", info.glued_back.len())
                },
        );
    }
    let ratio = sf::fmin(&pipes.iter().map(|p| p.air / (2.0 * p.a)).collect::<Vec<_>>());
    note(ratio >= 8.0, "no pipe is fat for its length", format!("{ratio:.1} diameters at the least"));
    let longest = sf::fmax(&parts.iter().map(|p| py::max(p.p.w, p.p.h)).collect::<Vec<_>>());
    note(
        longest + kerf() + 2.0 * sf::MARGIN <= sf::BED.0,
        "the longest part fits the bed",
        format!("{:.1}mm with the kerf, against {}", longest + kerf(), g(sf::BED.0 - 2.0 * sf::MARGIN)),
    );
    sheet_checks(&mut note, sheets, parts.len());
    out
}

fn svg_sheet(k: usize, n: usize, s: &Sheet, parts: &[TPart], shape: &str, edge: &str) -> String {
    let (mut black, mut blue) = (vec![], vec![]);
    let kf = kerf();
    for &(i, sx, sy) in &s.parts {
        let p = &parts[i];
        let (dx, dy) = (sx - p.p.x0 + kf / 2.0, sy - p.p.y0 + kf / 2.0);
        let mv = |pts: &[(f64, f64)]| -> Vec<(f64, f64)> { pts.iter().map(|&(x, y)| (x + dx, y + dy)).collect() };
        black.push(path(&mv(&offset(&p.p.of(), kf / 2.0)), true));
        for st in &p.strokes {
            blue.push(path(&mv(st), false));
        }
    }
    let where_ = if shape_of(shape).2 == Glue::Rows { "the front faces, or the back where a front is glued" } else { "the front faces" };
    let eng = match edge {
        "chamfer" => format!("the plugs' places on {where_} and the bevel's line on the free backs"),
        "plain" => format!("the plugs' places on {where_}"),
        _ => format!("the plugs' places on {where_} and the bevel's line round every notch"),
    };
    let desc = format!(
        "1 user unit = 1mm. Panpipes, E3 to E4, {shape}, {edge} edge, sheet {k} of {n}. 3mm ply, drawn for a {}mm kerf. \
         Blue {} engraves {eng}, on the faces' outsides, which lie face up. Then black {} frees the parts.",
        g(kf),
        sf::BLUE,
        sf::BLACK
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.2}mm\" \
         height=\"{h:.2}mm\" viewBox=\"0 0 {w:.2} {h:.2}\">\n<title>Panpipes E3 to E4, {shape}, {edge} edge, sheet {k} of {n}</title>\n\
         <desc>{desc}</desc>\n{}{}</svg>\n",
        group(sf::BLUE, &blue),
        group(sf::BLACK, &black),
        w = s.w,
        h = s.h
    )
}

/// The 3D view: each shape, chamfer edge, plugs at their lines.
fn page(path_out: &str, notes: &[String], end: f64, raise: f64) {
    let mut designs: Vec<(String, J)> = vec![];
    for (shape, ..) in SHAPES {
        let pipes = design(notes, shape, end, raise);
        let (parts, info) = build(&pipes, shape, "chamfer");
        let out: Vec<J> = parts
            .iter()
            .map(|p| {
                let mut m = p.m.clone();
                m.push(vec![I(0), I(0), I(0), I(1)]);
                let c = info.centres[p.pipe];
                let ex: [Num; 3] = if p.plug {
                    [F(1.2), I(0), I(0)]
                } else {
                    // each face out from its own tube's axis
                    let cx = (m[1][3].f() - c.0, m[2][3].f() - c.1);
                    let l = py::hypot(cx.0, cx.1);
                    let l = if l == 0.0 { 1.0 } else { l };
                    [I(0), F(0.9 * cx.0 / l), F(0.9 * cx.1 / l)]
                };
                let e: Vec<Vec<Pt>> = p.strokes.iter().map(|s| fpts(s)).collect();
                placed_raw(&p.p.outline, &[], &e, &chain(&[pp::up(), m]), &pp::turned(ex), if p.plug { "dark" } else { "ply" })
            })
            .collect();
        designs.push((
            shape.to_string(),
            J::obj(vec![("parts", J::A(out)), ("ex", J::N(I(30))), ("view", turn3d::nums(&[F(-0.6), F(-0.3), F(4.2)]))]),
        ));
    }
    turn3d::write(
        path_out,
        "Tube panpipes in 3D",
        &mut designs,
        &format!(
            "{}: each pipe a tube of finger-jointed strips, glued to \
             its neighbours, with its plug, darker, at its blue line. The buttons switch between \
             the octagonal row, the octagons in two rows, the hexagonal zigzag and the hexagonal \
             row, each with the chamfer edge’s line along the back of the rim where the back \
             is free.",
            pp::PAGE_NOTE_HEAD
        ),
        BY,
        true,
    );
}

fn main() {
    let (mut out, mut write, mut page_to) = (None::<String>, true, None::<String>);
    let mut solidcraft = None::<String>;
    let (mut shape, mut edge) = ("octagon-row".to_string(), "chamfer".to_string());
    let mut notes: Vec<String> = pp::NOTES.iter().map(|s| s.to_string()).collect();
    let (mut end, mut raise) = (pp::END, RAISE);
    let names: Vec<&str> = SHAPES.iter().map(|s| s.0).collect();
    for x in std::env::args().skip(1) {
        let val = || x.split_once('=').unwrap().1.to_string();
        let num = |v: String| py::float(&v).unwrap_or_else(|| die(format!("error: {x}: not a number.")));
        if x == "--no-write" {
            write = false;
        } else if x.starts_with("--out=") {
            out = Some(val());
        } else if x.starts_with("--shape=") {
            shape = val();
            if !names.contains(&shape.as_str()) {
                die(format!("error: --shape={shape}: the shapes are {}.", names.join(", ")));
            }
        } else if x.starts_with("--edge=") {
            edge = val();
            if !EDGES.contains(&edge.as_str()) {
                die(format!("error: --edge={edge}: the edges are {}.", EDGES.join(", ")));
            }
        } else if x.starts_with("--notes=") {
            notes = val().split(',').filter(|s| !s.is_empty()).map(String::from).collect();
        } else if x.starts_with("--raise=") {
            raise = num(val());
        } else if x.starts_with("--end=") {
            end = num(val());
        } else if x.starts_with("--kerf=") {
            sf::set_kerf(&x);
        } else if x.starts_with("--page-to=") {
            page_to = Some(val());
        } else if x.starts_with("--solidcraft=") {
            solidcraft = Some(val());
        } else {
            die(format!(
                "error: {x} is not a flag this generator reads. It takes --out=, --shape=, --edge=, --notes=, --raise=, --end=, --kerf=, --page-to=, --solidcraft= and --no-write."
            ));
        }
    }
    if let Some(pt) = page_to {
        page(&pt, &notes, end, raise);
        println!("  wrote {}", pp::basename(&pt));
        return;
    }
    if write && out.is_none() {
        die("error: --out=DIR, or --no-write to run the checks only.");
    }
    let pipes = design(&notes, &shape, end, raise);
    let (parts, info) = build(&pipes, &shape, &edge);
    let sheets = pack(&parts.iter().map(|p| (p.p.w, p.p.h)).collect::<Vec<_>>());
    let res = checks(&pipes, &parts, &info, &shape, &edge, &sheets);
    let pts: Vec<(f64, f64)> = outsides(&pipes, &info.centres, &shape).into_iter().flatten().collect();
    let xs: Vec<f64> = pts.iter().map(|q| q.0).collect();
    let ys: Vec<f64> = pts.iter().map(|q| q.1).collect();
    let (width, depth) = (sf::fmax(&xs) - sf::fmin(&xs), sf::fmax(&ys) - sf::fmin(&ys));
    let tail = if shape == "hex-zigzag" {
        if raise != 0.0 {
            format!("; the back row {}mm taller at the rim", g(raise))
        } else {
            "; the rims level".to_string()
        }
    } else {
        String::new()
    };
    println!(
        "panpipes, {} to {}, {shape}, {edge} edge: {} tubes of {} strips, the set {width:.1}mm across, {depth:.1}mm deep{tail}",
        notes[0],
        notes[notes.len() - 1],
        pipes.len(),
        shape_of(&shape).0
    );
    for p in &pipes {
        println!(
            "  {:<4} {:7.2}Hz  {:5.2}mm across the flats  plug face {:6.1}mm from the rim, tube {:6.1}mm",
            p.note, p.f, p.fl, p.air, p.length
        );
    }
    if print_checks(&res, 56) > 0 {
        std::process::exit(1);
    }
    if let Some(dir) = &solidcraft {
        pp::mkdirs(dir);
        let f = pp::join(dir, &format!("panpipes-tubes-{shape}-{edge}.solidcraft.json"));
        pp::write_file(&f, &tubes_solidcraft::script(&pipes, &shape, &edge, end, raise));
        println!("  wrote {}", pp::basename(&f));
    }
    if write {
        let out = out.unwrap();
        pp::mkdirs(&out);
        let stem = format!("panpipes-{}-{shape}-{edge}", notes[0].replace('#', "s"));
        for (k, s) in sheets.iter().enumerate() {
            let f = pp::join(&out, &format!("{stem}-sheet{}-cut-files.svg", k + 1));
            pp::write_file(&f, &svg_sheet(k + 1, sheets.len(), s, &parts, &shape, &edge));
            println!("  wrote {}  {:.0} x {:.0}mm, {} parts", pp::basename(&f), s.w, s.h, s.parts.len());
        }
    }
}
