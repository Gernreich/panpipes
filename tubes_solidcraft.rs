//! The tube sets as parametric SolidCraft designs (2026-11-08), one per shape
//! and edge: every rule tube_panpipes follows, as expressions. Type another
//! frequency into `f_1` ... `f_8` and that tube's length and its plug follow,
//! its finger joints re-counted along every edge; change `area_low` or
//! `area_high` and the tubes widen, the set re-laid so each stays glued face
//! to face to its neighbour; change `ply` and the strips, the joints' depth
//! and the plugs follow.
//!
//! The frame is the 3D view's and the STEP files': in the generator's frame
//! (x down the tubes from the rim, y across the set, z from the player to the
//! back) the world is X = -y, Y = z, Z = -x, the rims at Z = 0.
//!
//! The joint is the four-hole ocarinas': along each edge the cells alternate
//! between its two strips, as many as fit with none under `finger`, an even
//! count since both ends are open; each strip loses its neighbour's cells as a
//! strip ply x sin(the joint's angle) deep. Where the back face is notched it
//! owns its edges' cells within `rim_own` of the rim.
//!
//! The chamfer is drawn sanded, down to its blue line, on every free back
//! face; on octagon-two-rows the front row's back faces are glued to the back
//! row and are left as cut. The notch's U is cut, its filed bevel left out.

use super::{facing, shape_of, tube, Glue, Pipe, NOTCH_OF, RIM_OWN};
use four_hole_ocarinas::polyhedron::{self as ph, V3};
use lasermade_tools::solidcraft::{n, Script};
use std::f64::consts::PI;

/// A point in the generator's frame as expressions.
#[derive(Clone)]
struct R {
    x: String,
    y: String,
    z: String,
}

/// The world vector of a generator-frame vector.
fn w(v: V3) -> [f64; 3] {
    [-v[1], v[2], -v[0]]
}

/// sum of coef * (expr), the zero terms dropped.
fn lin(terms: &[(f64, &str)]) -> String {
    let t: Vec<String> = terms.iter().filter(|(c, _)| c.abs() > 1e-12).map(|(c, e)| format!("{}*({e})", n(*c))).collect();
    if t.is_empty() { "0 mm".into() } else { t.join(" + ") }
}

/// The world point's component along the world direction d.
fn dotw(p: &R, d: [f64; 3]) -> String {
    lin(&[(-d[0], &p.y), (d[1], &p.z), (-d[2], &p.x)])
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Each tube's axis, (y, z), as expressions: centres() written out.
fn centres(s: &mut Script, np: usize, glue: Glue) {
    let fo = |i: usize| format!("fo_{}", i + 1);
    // (y, z) of pipe j against pipe i across i's face at theta
    let against = |i: usize, j: usize, th: f64, cy: &[String], cz: &[String]| -> (String, String) {
        let t = th.to_radians();
        let d = format!("(({}) + ({}))/2", fo(i), fo(j));
        (lin(&[(1.0, &cy[i]), (t.sin(), &d)]), lin(&[(1.0, &cz[i]), (-t.cos(), &d)]))
    };
    let mut cy: Vec<String> = vec!["0 mm".into(); np];
    let mut cz: Vec<String> = vec!["0 mm".into(); np];
    let pname = |c: &str, i: usize| format!("{c}_{}", i + 1);
    match glue {
        Glue::Faces(gl) => {
            for i in 1..np {
                let th = gl[(i - 1) % gl.len()];
                let (y, z) = against(i - 1, i, th, &cy, &cz);
                s.len(&pname("cy", i), y, &format!("read only: tube {}'s axis across the set", i + 1));
                s.len(&pname("cz", i), z, &format!("read only: tube {}'s axis front to back", i + 1));
                cy[i] = pname("cy", i);
                cz[i] = pname("cz", i);
            }
        }
        Glue::Rows => {
            // before the back row is centred on the front: cy0
            let mut cy0: Vec<String> = cy.clone();
            for i in 1..np {
                let (k, th) = if i % 2 == 1 { (i - 1, 180.0) } else { (i - 2, 90.0) };
                let (mut y, z) = against(k, i, th, &cy0, &cz);
                if i % 2 == 1 && i >= 3 {
                    y = against(i - 2, i, 90.0, &cy0, &cz).0;
                }
                s.len(&pname("cy0", i), y, &format!("read only: tube {}'s axis across, before the back row is centred", i + 1));
                s.len(&pname("cz", i), z, &format!("read only: tube {}'s axis front to back", i + 1));
                cy0[i] = pname("cy0", i);
                cz[i] = pname("cz", i);
            }
            let drift: Vec<String> = (1..np).step_by(2).map(|i| format!("{} - {}", cy0[i], cy0[i - 1])).collect();
            s.len("mid", format!("(min({0}) + max({0}))/2", drift.join(", ")), "read only: how far the back row is moved across to centre it on the front");
            for i in 0..np {
                if i % 2 == 1 {
                    s.len(&pname("cy", i), format!("{} - mid", cy0[i]), &format!("read only: tube {}'s axis across the set", i + 1));
                    cy[i] = pname("cy", i);
                } else if i > 0 {
                    s.len(&pname("cy", i), cy0[i].clone(), &format!("read only: tube {}'s axis across the set", i + 1));
                    cy[i] = pname("cy", i);
                }
            }
            let _ = cy;
        }
    }
}

pub fn script(pipes: &[Pipe], shape: &str, edge: &str, end: f64, raise: f64) -> String {
    let (ns, phi, glue) = shape_of(shape);
    let np = pipes.len();
    let nf = ns as f64;
    let mut s = Script::new();
    s.len("ply", format!("{} mm", n(super::T)), "sheet thickness: every strip, each plug layer, and the joints' depth");
    s.len("finger", format!("{} mm", n(ph::FINGER)), "the shortest cell along an edge");
    s.len("interf", format!("{} mm", n(super::INTERF)), "each plug's press-fit, over the bore across its flats");
    s.num("temp_c", n(square_flute::T_C), "the air's temperature, warm from the breath, degrees C");
    s.num("c_mm_s", "331300*sqrt(1 + temp_c/273.15)", "speed of sound, mm/s");
    s.num("a4", n(super::pp::A4), "concert pitch, Hz");
    s.num("end", n(end), "the open end's correction, times the bore's equivalent radius");
    s.num("area_low", n(super::AREA_LOW), "the lowest tube's bore area, mm²");
    s.num("area_high", n(super::AREA_HIGH), "the highest tube's, mm²; those between step evenly");
    s.len("spare", format!("{} mm", n(super::pp::SPARE)), "empty tube below each plug, so it can come down and flatten the note");
    if shape == "hex-zigzag" {
        s.len("raise", format!("{} mm", n(raise)), "how much taller the back row stands at the rim");
    }
    let kshape = nf * (PI / nf).tan() / 4.0;
    for (i, p) in pipes.iter().enumerate() {
        let k = i + 1;
        let t = if np > 1 { i as f64 / (np - 1) as f64 } else { 0.0 };
        let sm = (12.0 * (p.f / super::pp::A4).log2()).round() as i64;
        s.num(&format!("f_{k}"), format!("a4*pow(2, ({sm})/12)"), &format!("tube {k}'s note, Hz: {} by default; type a frequency to retune it. Keep them rising", p.note));
        s.num(&format!("area_{k}"), format!("area_low + (area_high - area_low)*{}", n(t)), &format!("read only: tube {k}'s bore area, mm²"));
        s.len(&format!("fl_{k}"), format!("sqrt(area_{k}/{})*1 mm", n(kshape)), &format!("read only: tube {k}'s bore across the flats"));
        s.len(&format!("fo_{k}"), format!("fl_{k} + 2*ply"), &format!("read only: tube {k} across the flats outside"));
        s.len(&format!("air_{k}"), format!("c_mm_s/(4*f_{k})*1 mm - end*sqrt(area_{k}/pi)*1 mm"), &format!("read only: tube {k}'s air column, rim to plug face"));
        s.len(&format!("len_{k}"), format!("air_{k} + {}*ply + spare", super::pp::PLUG_LAYERS), &format!("read only: tube {k}'s length"));
        let rim = if shape == "hex-zigzag" && i % 2 == 1 { "-raise" } else { "0 mm" };
        s.len(&format!("rim_{k}"), rim, &format!("read only: where tube {k}'s rim stands, down from the set's"));
        s.num(&format!("ncell_{k}"), format!("floor(len_{k}/finger + 1e-9) - mod(floor(len_{k}/finger + 1e-9), 2)"), &format!("read only: cells along each of tube {k}'s edges"));
        s.len(&format!("cell_{k}"), format!("len_{k}/ncell_{k}"), &format!("read only: each cell's length on tube {k}"));
    }
    s.len("cy_1", "0 mm", "tube 1's axis across the set");
    s.len("cz_1", "0 mm", "tube 1's axis front to back");
    centres(&mut s, np, glue);
    s.len(
        "bed_room",
        format!("{} mm - len_1", n(square_flute::BED.0 - 2.0 * square_flute::MARGIN - square_flute::kerf())),
        "read only: what the bed leaves past the longest strip; below 0 it does not fit the laser",
    );
    let chamfer = edge == "chamfer";
    let notch = edge == "notch";
    if chamfer {
        s.len("chamfer_left", format!("{} mm", n(super::pp::CHAMFER_LEFT)), "the edge left on the inside of the bore, when the rim is sanded to 45 degrees");
        s.len("chamfer_down", "ply - chamfer_left", "read only: how far down the outside the bevel runs, the blue line");
    }
    if notch {
        s.num("notch_of", n(NOTCH_OF), "the notch's width, of the back face's inside width");
        s.len("notch_past", format!("{} mm", n(super::pp::NOTCH_PAST)), "the notch's depth past its half-round bottom");
        s.len("rim_own", format!("{} mm", n(RIM_OWN)), "the notched face owns its joints this far down from the rim");
    }
    // which tubes have their back glued out of sight (two rows: the front row)
    let glued_back: Vec<bool> = (0..np).map(|i| glue == Glue::Rows && i % 2 == 0 && i + 1 < np).collect();

    let mut num = 0usize;
    let mut nfeat = 0usize;
    for (i, p) in pipes.iter().enumerate() {
        let k = i + 1;
        let faces = tube(ns, phi, p.fo, p.length);
        let ends: Vec<usize> = (0..faces.len()).filter(|&j| faces[j].name == "rim" || faces[j].name == "bottom").collect();
        let walls: Vec<usize> = (0..faces.len()).filter(|j| !ends.contains(j)).collect();
        let back: Vec<usize> = facing(ns, phi, 1.0).iter().map(|&q| walls[q]).collect();
        let fo0 = p.fo;
        let len0 = p.length;
        let rim = format!("rim_{k}");
        let at = |v: V3| R {
            x: if v[0].abs() < 1e-6 { rim.clone() } else { format!("{rim} + len_{k}") },
            y: lin(&[(1.0, &format!("cy_{k}")), (v[1] / fo0, &format!("fo_{k}"))]),
            z: lin(&[(1.0, &format!("cz_{k}")), (v[2] / fo0, &format!("fo_{k}"))]),
        };
        let _ = len0;
        // each strip's sketch frame: x down the tube, its normal inward
        struct Fr {
            xd: [f64; 3],
            yd: [f64; 3],

            off: String,
            body: String,
        }
        let mut frs: Vec<Option<Fr>> = (0..faces.len()).map(|_| None).collect();
        for &fi in &walls {
            let f = &faces[fi];
            let xd = w([1.0, 0.0, 0.0]);
            let mut yd = w(ph::cross(f.n, [1.0, 0.0, 0.0]));
            let mut nrm = cross(xd, yd);
            if dot3(nrm, w(f.n)) > 0.0 {
                yd = [-yd[0], -yd[1], -yd[2]];
                nrm = cross(xd, yd);
            }
            num += 1;
            let body = format!("{} wall #{num}", p.note);
            let off = dotw(&at(f.verts[0]), nrm);
            let sk = s.sketch_on([0.0, 0.0, 0.0], xd, yd, Some(&off), &format!("{body} sketch"));
            let pts: Vec<[String; 2]> = f.verts.iter().map(|&v| [dotw(&at(v), xd), dotw(&at(v), yd)]).collect();
            s.poly(&pts);
            s.finish();
            s.extrude(&sk, "ply", false, "new", &[], &body);
            frs[fi] = Some(Fr { xd, yd, off, body });
        }
        // the joints
        for e in ph::edges(&faces) {
            if !(walls.contains(&e.a) && walls.contains(&e.b)) {
                continue;
            }
            let dirv = ph::unit(ph::sub(e.q, e.p));
            let from_rim = e.p[0].abs() < 1e-6;
            let start = at(e.p);
            let first = e.a;
            let ncell = format!("ncell_{k}");
            let wexpr = format!("cell_{k}");
            let forced = notch && (e.a == back[0] || e.b == back[0]);
            let (kmin, kmax) = if from_rim {
                ("0".to_string(), format!("(ceil(rim_own/{wexpr}) - 1)"))
            } else {
                (format!("floor((len_{k} - rim_own)/{wexpr})"), format!("({ncell} - 1)"))
            };
            let sin_t = e.theta.to_radians().sin();
            let dc = format!("(ply*{})", n(sin_t));
            for pl in [e.a, e.b] {
                let fr = frs[pl].as_ref().unwrap();
                let s2 = (dotw(&start, fr.xd), dotw(&start, fr.yd));
                let wd = w(dirv);
                let d2 = (dot3(wd, fr.xd), dot3(wd, fr.yd));
                let cen = at(faces[pl].centre);
                // into the strip: perpendicular to the edge, toward its centre
                let c2 = (dot3(w(ph::sub(faces[pl].centre, e.p)), fr.xd), dot3(w(ph::sub(faces[pl].centre, e.p)), fr.yd));
                let _ = cen;
                let mut nin = (-d2.1, d2.0);
                if c2.0 * nin.0 + c2.1 * nin.1 < 0.0 {
                    nin = (-nin.0, -nin.1);
                }
                let strip = |t0: &str, t1: &str, depth: &str, away: &str| -> Vec<[String; 2]> {
                    let pt = |t: &str, dd: &str| {
                        let along = format!("({t})");
                        let across = format!("(({dd}) - ({away}))");
                        [
                            format!("{} + {}", s2.0, lin(&[(d2.0, &along), (nin.0, &across)])),
                            format!("{} + {}", s2.1, lin(&[(d2.1, &along), (nin.1, &across)])),
                        ]
                    };
                    vec![pt(t0, "-ply"), pt(t1, "-ply"), pt(t1, depth), pt(t0, depth)]
                };
                let target = fr.body.clone();
                let (xd, yd, off) = (fr.xd, fr.yd, fr.off.clone());
                let mut cutter = |s: &mut Script, pts: Vec<[String; 2]>, count: Option<(String, String)>| {
                    nfeat += 1;
                    let name = format!("cells {nfeat}");
                    let sk = s.sketch_on([0.0, 0.0, 0.0], xd, yd, Some(&off), &format!("{name} sketch"));
                    s.poly(&pts);
                    s.finish();
                    s.extrude(&sk, "ply", false, "cut", &[&target], &name);
                    if let Some((cnt, sp)) = count {
                        s.pattern(&name, wd, cnt, sp);
                    }
                };
                let k0 = if pl == first { 1 } else { 0 };
                let mut prog = |s: &mut Script, from: &str, cnt: &str| {
                    let away = format!("(1 - min({cnt}, 1))*1000 mm");
                    let t0 = format!("({from})*{wexpr}");
                    let t1 = format!("({from} + 1)*{wexpr}");
                    cutter(s, strip(&t0, &t1, &dc, &away), Some((format!("max({cnt}, 1)"), format!("2*{wexpr}"))));
                };
                if !forced {
                    prog(&mut s, &k0.to_string(), &format!("floor(({ncell} - {k0} + 1)/2)"));
                } else if pl == back[0] {
                    // the neighbour's cells, but none within rim_own of the rim
                    prog(&mut s, &k0.to_string(), &format!("max(0, floor(({kmin} - {k0} + 1)/2))"));
                    let ka = format!("({kmax} + 1 + mod({kmax} + 1 - {k0}, 2))");
                    prog(&mut s, &ka, &format!("max(0, floor(({ncell} - {ka} + 1)/2))"));
                } else {
                    // the back face's cells, and all of those near the rim
                    prog(&mut s, &k0.to_string(), &format!("floor(({ncell} - {k0} + 1)/2)"));
                    let t0 = format!("({kmin})*{wexpr}");
                    let t1 = format!("({kmax} + 1)*{wexpr}");
                    cutter(&mut s, strip(&t0, &t1, &dc, "0 mm"), None);
                }
            }
        }
        // the blowing edge, on the back face(s)
        let face_in = format!("fl_{k}*{}", n((PI / nf).tan()));
        let face_out = format!("fo_{k}*{}", n((PI / nf).tan()));
        if chamfer && !glued_back[i] {
            for &bf in &back {
                let f = &faces[bf];
                let fr = frs[bf].as_ref().unwrap();
                let xd = w([1.0, 0.0, 0.0]);
                let yd = w(f.n);
                let nrm = cross(xd, yd);
                // the face's middle, on its outside, at the rim
                let mid = R {
                    x: rim.clone(),
                    y: lin(&[(1.0, &format!("cy_{k}")), (f.n[1] / 2.0, &format!("fo_{k}"))]),
                    z: lin(&[(1.0, &format!("cz_{k}")), (f.n[2] / 2.0, &format!("fo_{k}"))]),
                };
                let (u0, v0) = (dotw(&mid, xd), dotw(&mid, yd));
                let off = format!("{} - ({face_out})/2 - 1 mm", dotw(&mid, nrm));
                nfeat += 1;
                let name = format!("bevel {nfeat}, sanded");
                let sk = s.sketch_on([0.0, 0.0, 0.0], xd, yd, Some(&off), &format!("{name} sketch"));
                s.poly(&[[u0.clone(), v0.clone()], [u0.clone(), format!("{v0} - chamfer_down")], [format!("{u0} + chamfer_down"), v0.clone()]]);
                s.finish();
                s.extrude(&sk, format!("{face_out} + 2 mm"), false, "cut", &[&fr.body], &name);
            }
        }
        if notch {
            let bf = back[0];
            let f = &faces[bf];
            let fr = frs[bf].as_ref().unwrap();
            let mid = R {
                x: rim.clone(),
                y: lin(&[(1.0, &format!("cy_{k}")), (f.n[1] / 2.0, &format!("fo_{k}"))]),
                z: lin(&[(1.0, &format!("cz_{k}")), (f.n[2] / 2.0, &format!("fo_{k}"))]),
            };
            let (u0, v0) = (dotw(&mid, fr.xd), dotw(&mid, fr.yd));
            let wn = format!("(notch_of*{face_in})");
            nfeat += 1;
            let name = format!("notch {k}");
            let sk = s.sketch_on([0.0, 0.0, 0.0], fr.xd, fr.yd, Some(&fr.off), &format!("{name} sketch"));
            s.poly(&[
                [format!("{u0} - 1 mm"), format!("{v0} - {wn}/2")],
                [format!("{u0} + notch_past"), format!("{v0} - {wn}/2")],
                [format!("{u0} + notch_past"), format!("{v0} + {wn}/2")],
                [format!("{u0} - 1 mm"), format!("{v0} + {wn}/2")],
            ]);
            s.finish();
            s.extrude(&sk, "ply", false, "cut", &[&fr.body], &name);
            let sk = s.sketch_on([0.0, 0.0, 0.0], fr.xd, fr.yd, Some(&fr.off), &format!("{name}, round sketch"));
            s.cmd(
                "sketch.circle.center",
                format!(
                    "{{\"center\":[{},{}],\"diameter\":{}}}",
                    lasermade_tools::solidcraft::q(&format!("{u0} + notch_past")),
                    lasermade_tools::solidcraft::q(&v0),
                    lasermade_tools::solidcraft::q(&wn)
                ),
            );
            s.finish();
            s.extrude(&sk, "ply", false, "cut", &[&fr.body], &format!("{name}, round"));
        }
        // the plugs
        let rp = format!("(fl_{k} + interf)/2/{}", n((PI / nf).cos()));
        for layer in 0..super::pp::PLUG_LAYERS {
            num += 1;
            let body = format!("plug, {} #{num}", p.note);
            let sk = s.sketch("XY", Some(&format!("-({rim} + air_{k} + {layer}*ply)")), &format!("{body} sketch"));
            let pts: Vec<[String; 2]> = (0..ns)
                .map(|q| {
                    let t = (phi + (q as f64 + 0.5) * 360.0 / nf).to_radians();
                    // world X = -y, Y = z
                    [lin(&[(-1.0, &format!("cy_{k}")), (-t.sin(), &rp)]), lin(&[(1.0, &format!("cz_{k}")), (-t.cos(), &rp)])]
                })
                .collect();
            s.poly(&pts);
            s.finish();
            s.extrude(&sk, "ply", true, "new", &[], &body);
        }
    }
    s.measure();
    s.text()
}
