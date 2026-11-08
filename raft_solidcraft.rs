//! The raft as a parametric SolidCraft design (2026-11-08): every rule the
//! generator follows, as expressions, so changing one number rebuilds the
//! rest. Type another frequency into `f_1` ... `f_8` and that pipe lengthens or
//! shortens, its plug moves to its new air column, and the walls, the
//! plates' staircase and every tab and slot follow; change `ply` and the
//! fingers, the slots and the plugs follow that.
//!
//! The frame is the 3D view's and the STEP files': X across the raft (the
//! lowest pipe at X = 0, the raft running to -X), Y from the front plate's
//! inside face to the back, Z up, the rim at Z = 0 and the pipes hanging
//! below it. In the generator's raft frame (x down the pipes, y across, z
//! front to back) that is X = -y, Y = z, Z = -x, and the expressions below are
//! written in the raft frame and turned by `rbox`.
//!
//! The blowing edge is the build's: the chamfer is drawn sanded, as the blue
//! line asks; the notch's U is cut, its filed bevel left out; the strip is
//! glued on. The front plate's lip rounding and the engraved lines are not
//! drawn.

use super::{Pipe, EDGES, PLUG_LAYERS};
use lasermade_tools::solidcraft::{n, Script};

/// A box given by its ranges in the raft frame, turned into the world.
fn rbox(s: &mut Script, name: &str, x: (&str, &str), y: (&str, &str), z: (&str, &str), op: &str, targets: &[&str]) {
    let (x0, x1) = x;
    let (y0, y1) = y;
    let (z0, z1) = z;
    s.bx(
        name,
        [format!("-({y1})"), format!("{z0}"), format!("-({x1})")],
        [format!("({y1}) - ({y0})"), format!("({z1}) - ({z0})"), format!("({x1}) - ({x0})")],
        op,
        targets,
    );
}

/// Semitones from A4 of a note name, for the default frequencies.
fn semis(note: &str) -> i64 {
    let f = super::freq(note);
    (12.0 * (f / super::A4).log2()).round() as i64
}

pub fn script(pipes: &[Pipe], edge: &str, end: f64) -> String {
    assert!(EDGES.contains(&edge));
    let np = pipes.len();
    let mut s = Script::new();
    // the sheet, the joints
    s.len("ply", format!("{} mm", n(super::T)), "sheet thickness: the plates, the walls, each plug layer, and every tab's depth");
    s.len("finger", "2*ply", "a finger's width, and the space between two on the end walls");
    s.len("finger_end", "2*finger", "the end walls' fingers stay this far from each end");
    s.len("tab_pitch", format!("{} mm", n(super::TAB_PITCH)), "an inner wall's tabs, this far apart");
    s.len("over_clear", format!("{} mm", n(super::OVER_CLEAR)), "wall either side of a tab below a shorter pipe");
    s.len("interf", format!("{} mm", n(super::INTERF)), "each plug's press-fit, over the bore each way");
    // the air
    s.num("temp_c", n(square_flute::T_C), "the air's temperature, warm from the breath, degrees C");
    s.num("c_mm_s", "331300*sqrt(1 + temp_c/273.15)", "speed of sound, mm/s");
    s.num("a4", n(super::A4), "concert pitch, Hz");
    s.num("end", n(end), "the open end's correction, times the bore's equivalent radius");
    // the bores
    s.len("depth", format!("{} mm", n(super::DEPTH)), "every bore, front to back");
    s.len("w_low", format!("{} mm", n(super::W_LOW)), "the lowest pipe's bore, across the raft");
    s.len("w_high", format!("{} mm", n(super::W_HIGH)), "the highest pipe's; those between step evenly");
    s.len("spare", format!("{} mm", n(super::SPARE)), "empty pipe below each plug, so it can come down and flatten the note");
    for (i, p) in pipes.iter().enumerate() {
        let k = i + 1;
        let t = if np > 1 { i as f64 / (np - 1) as f64 } else { 0.0 };
        let sm = semis(&p.note);
        s.num(&format!("f_{k}"), format!("a4*pow(2, ({sm})/12)"), &format!("pipe {k}'s note, Hz: {} by default; type a frequency to retune it. Keep them rising", p.note));
        s.len(&format!("w_{k}"), format!("w_low + (w_high - w_low)*{}", n(t)), &format!("pipe {k}'s bore across"));
        s.len(&format!("air_{k}"), format!("c_mm_s/(4*f_{k})*1 mm - end*sqrt((w_{k}/(1 mm))*(depth/(1 mm))/pi)*1 mm"), &format!("read only: pipe {k}'s air column, rim to plug face"));
        s.len(&format!("len_{k}"), format!("air_{k} + {}*ply + spare", PLUG_LAYERS), &format!("read only: pipe {k}'s length"));
    }
    // where the walls stand: wall j between pipes j and j+1 (1-based pipes),
    // wall 0 and wall np the ends
    s.len("y_0", "0 mm", "wall 0's outer face");
    for j in 1..=np {
        s.len(&format!("y_{j}"), format!("y_{} + ply + w_{j}", j - 1), &format!("read only: where wall {j} starts across the raft"));
    }
    s.len("raft_w", format!("y_{np} + ply"), "read only: the raft across");
    s.len(
        "bed_room",
        format!("{} mm - len_1", n(square_flute::BED.0 - 2.0 * square_flute::MARGIN - square_flute::kerf())),
        "read only: what the bed leaves past the longest pipe (the plates are as long); below 0 it does not fit the laser",
    );
    // wall j's length: the longer of its pipes, the lower one
    let wl = |j: usize| if j == 0 { "len_1".to_string() } else { format!("len_{j}") };
    // the plates: a staircase of one box per pipe
    let plates = [("front plate #1", "-ply", "0 mm"), ("back plate #2", "depth", "depth + ply")];
    for (pn, z0, z1) in plates {
        rbox(&mut s, pn, ("0 mm", "len_1"), ("0 mm", "y_1 + ply"), (z0, z1), "new", &[]);
        for k in 2..=np {
            let (ya, yb) = (format!("y_{} + ply", k - 1), format!("y_{k} + ply"));
            rbox(&mut s, &format!("{pn}, pipe {k}"), ("0 mm", &format!("len_{k}")), (&ya, &yb), (z0, z1), "join", &[pn]);
        }
    }
    let both = ["front plate #1", "back plate #2"];
    let mut num = 2usize;
    // the walls, their tabs, and the tabs' slots and notches in the plates
    for j in 0..=np {
        num += 1;
        let name = format!("{} #{num}", if j == 0 || j == np { "end wall" } else { "wall" });
        let (ya, yb) = (format!("y_{j}"), format!("y_{j} + ply"));
        rbox(&mut s, &name, ("0 mm", &wl(j)), (&ya, &yb), ("0 mm", "depth"), "new", &[]);
        // (count, first x, pitch, lift): lift moves a run that should not be
        // there clear of everything, since a pattern cannot be empty
        let mut runs: Vec<(String, String, String, String)> = vec![];
        if j == 0 || j == np {
            let l = wl(j);
            s.num(&format!("n_end_{j}"), format!("floor(({l} - 2*finger_end - finger)/(2*finger)) + 1"), &format!("read only: wall {j}'s fingers each edge"));
            runs.push((format!("n_end_{j}"), format!("{l}/2 - (n_end_{j} - 1)*finger"), "2*finger".into(), "0 mm".into()));
        } else {
            let short = format!("len_{}", j + 1);
            let long = format!("len_{j}");
            s.num(&format!("n_tab_{j}"), format!("floor(({short} - 2*finger_end - finger)/tab_pitch) + 1"), &format!("read only: wall {j}'s tabs through the slots, each edge"));
            runs.push((format!("n_tab_{j}"), format!("{short}/2 - (n_tab_{j} - 1)*tab_pitch/2"), "tab_pitch".into(), "0 mm".into()));
            let ov = format!("(({long}) - ({short}))");
            s.num(&format!("n_low_{j}"), format!("max(1, floor(({ov} - 2*over_clear - finger)/tab_pitch) + 1)"), &format!("read only: wall {j}'s tabs below pipe {}, in notches in the staircase", j + 1));
            s.num(&format!("has_low_{j}"), format!("min(1, max(0, floor({ov}/(finger + 2*over_clear))))"), &format!("read only: 1 if wall {j} reaches far enough below pipe {} for a tab", j + 1));
            runs.push((
                format!("n_low_{j}"),
                format!("{short} + {ov}/2 - (n_low_{j} - 1)*tab_pitch/2"),
                "tab_pitch".into(),
                format!("(1 - has_low_{j})*10000 mm"),
            ));
        }
        for (r, (cnt, first, pitch, lift)) in runs.iter().enumerate() {
            let xa = format!("{first} - finger/2 + {lift}");
            let xb = format!("{first} + finger/2 + {lift}");
            let tab = format!("{name} tabs {}", r + 1);
            rbox(&mut s, &tab, (&xa, &xb), (&ya, &yb), ("-ply", "depth + ply"), "join", &[&name]);
            s.pattern(&tab, [0.0, 0.0, -1.0], cnt, pitch);
            let slot = format!("{name} slots {}", r + 1);
            rbox(&mut s, &slot, (&xa, &xb), (&ya, &yb), ("-ply", "depth + ply"), "cut", &both);
            s.pattern(&slot, [0.0, 0.0, -1.0], cnt, pitch);
        }
    }
    // the plugs, at each pipe's air column
    for k in 1..=np {
        for layer in 0..PLUG_LAYERS {
            num += 1;
            let xa = format!("air_{k} + {layer}*ply");
            let xb = format!("air_{k} + {}*ply", layer + 1);
            let (ya, yb) = (format!("y_{} + ply - interf/2", k - 1), format!("y_{} + ply + w_{k} + interf/2", k - 1));
            rbox(&mut s, &format!("plug, {} #{num}", pipes[k - 1].note), (&xa, &xb), (&ya, &yb), ("-interf/2", "depth + interf/2"), "new", &[]);
        }
    }
    // the blowing edge
    let back = "back plate #2";
    match edge {
        "chamfer" => {
            s.len("chamfer_left", format!("{} mm", n(super::CHAMFER_LEFT)), "the edge left on the inside of the bore, when the rim is sanded to 45 degrees");
            s.len("chamfer_down", "ply - chamfer_left", "read only: how far down the outside the bevel runs, the blue line");
            // in the plane across the plate (world Y, Z), swept across the raft
            let sk = s.sketch_on([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], None, "bevel sketch");
            s.poly(&[["depth + chamfer_left", "0 mm"], ["depth + ply", "0 mm"], ["depth + ply", "-chamfer_down"]]);
            s.finish();
            s.extrude(&sk, "raft_w", true, "cut", &[back], "bevel, sanded");
        }
        "notch" => {
            s.num("notch_of", n(super::NOTCH_OF), "each notch's width, of its pipe's bore");
            s.len("notch_past", format!("{} mm", n(super::NOTCH_PAST)), "each notch's depth past its half-round bottom");
            for k in 1..=np {
                let yc = format!("y_{} + ply + w_{k}/2", k - 1);
                let r = format!("notch_of*w_{k}/2");
                let (ya, yb) = (format!("{yc} - {r}"), format!("{yc} + {r}"));
                rbox(&mut s, &format!("notch {k}"), ("0 mm", "notch_past"), (&ya, &yb), ("depth", "depth + ply"), "cut", &[back]);
                s.cylinder(&format!("notch {k}, round"), [format!("-({yc})"), "depth".into(), "-notch_past".into()], Some([0.0, 1.0, 0.0]), &format!("notch_of*w_{k}"), "ply", "cut", &[back]);
            }
        }
        "strip" => {
            s.len("strip_h", format!("{} mm", n(super::STRIP_H)), "the back plate stops this short of the rim; the strip fills it");
            s.len("strip_t", format!("{} mm", n(super::STRIP_T)), "the strip's stock: the blowing edge's thickness");
            rbox(&mut s, "back plate, cut down", ("-1 mm", "strip_h"), ("0 mm", "raft_w"), ("depth", "depth + ply"), "cut", &[back]);
            num += 1;
            rbox(&mut s, &format!("edge strip #{num}"), ("0 mm", "strip_h"), ("0 mm", "raft_w"), ("depth", "depth + strip_t"), "new", &[]);
        }
        _ => {}
    }
    s.measure();
    s.text()
}
