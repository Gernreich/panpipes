#!/usr/bin/env python3
"""Cut files for the same low panpipes, E3 to E4, as separate tubes glued
side to side: octagonal in a row, hexagonal in a zigzag, or hexagonal in a row.

    python3 tube_panpipes.py --shape=octagon-row --no-write      # the checks
    python3 tube_panpipes.py --shape=hex-zigzag --out=DIR        # the sheets
    python3 tube_panpipes.py --shape=hex-row --edge=plain ...    # another edge
    python3 tube_panpipes.py --kerf=0.18 ...                     # for that kerf
    python3 tube_panpipes.py --page-to=turn-tubes.html           # the 3D view

EACH PIPE is a tube of N strips of 3mm ply, finger-jointed edge to edge by the
four-hole ocarinas' polyhedron.py joint, as the octagonal flute's tube is:
open at the rim, open at the bottom, a plug of PLUG_LAYERS polygons pressed up
from below to a blue line on its front. The acoustics are panpipes.py's: the
same notes, the same air columns for the same bore areas, AREA_LOW at E3
falling to AREA_HIGH at E4, the raft's. The tubes are press-fit and sealed;
NEIGHBOURS ARE GLUED, face to face, since separate tubes share no wall.

THE SHAPES, the y axis across the set, z from the player to the back:

  octagon-row  octagons in a line, a flat to the front for the lip, a flat
               to the back for the blowing edge, and flats either side, glued
               to the neighbours
  hex-zigzag   hexagons with a flat to the front and to the back, each pipe
               glued to the next on a diagonal face, so they alternate between
               a front row and a back row half a pipe behind it, as a siku's
               two rows split the scale; the back row stands RAISE taller at
               the rim, so its front face is clear of the front row for the lip
  hex-row      hexagons with flats either side, glued in a line; a corner
               faces the lip and the blowing edge is a V of two faces

THE BLOWING EDGE (--edge=) is the far side of the rim, as on the raft:

  chamfer  the default: a blue line engraved CHAMFER_DOWN below the rim on
           the outside of the back face (both back faces on hex-row); sand a
           45 degree bevel down to it
  plain    the rim as cut
  notch    a U cut into the back face's rim with a bevel line round it, the
           raft's quena notch, narrower to fit one face; not on hex-row,
           which has a corner there
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
for sib in ('square-flute', 'swept-curve', 'four-hole-ocarinas', 'lasermade-tools'):
    sys.path.insert(0, os.path.join(HERE, '..', sib))
import panpipes as PP                                          # noqa: E402
import square_flute as SF                                      # noqa: E402
import polyhedron as P                                         # noqa: E402
from four_hole_ocarinas import offset                          # noqa: E402
from square_flute import Part, pack                            # noqa: E402
from ribbon_bore import path                                   # noqa: E402

T, KERF = SF.T, SF.KERF
MIN_WEB = SF.MIN_WEB
BED, MARGIN = SF.BED, SF.MARGIN
BLUE, ORANGE, BLACK = SF.BLUE, SF.ORANGE, SF.BLACK
INTERF = SF.INTERF_ACROSS

AREA_LOW = PP.W_LOW * PP.DEPTH     # 440mm^2, the raft's lowest pipe
AREA_HIGH = PP.W_HIGH * PP.DEPTH   # 320mm^2, its highest
# sides, the first face's angle from the front, and the face each pipe is
# glued to the next across (alternating for the zigzag), in degrees
SHAPES = {
    'octagon-row': (8, 0.0, (90.0,)),
    'hex-zigzag': (6, 0.0, (120.0, 60.0)),
    'hex-row': (6, 30.0, (90.0,)),
}
EDGES = ('chamfer', 'plain', 'notch')
NOTCH_OF = 0.6                     # notch width, of the back face's inside width
RIM_OWN = 3 * P.FINGER             # the back face owns its joints this far down
LINE_IN = 0.5                      # an engraved line's least distance in from a face's edge
# The zigzag's back row stands this much taller at the rim. Level rims would put
# a back pipe's front face between two front pipes, half a pipe behind where the
# lip rests; raised, its front face stands clear above them for the lip.
# Measured by nobody yet: --raise= changes it, 0 levels the rims.
RAISE = 15.0


def normal(theta):
    """A face's outward normal, theta degrees round from the front."""
    t = math.radians(theta)
    return (0.0, math.sin(t), -math.cos(t))


def tangent(theta):
    t = math.radians(theta)
    return (0.0, math.cos(t), math.sin(t))


def design(notes, shape):
    n, phi, _ = SHAPES[shape]
    k = n * math.tan(math.pi / n) / 4                # area = k x flats^2
    fs = [PP.freq(x) for x in notes]
    if any(b <= a for a, b in zip(fs, fs[1:])):
        raise SystemExit('error: --notes= runs from the lowest pipe to the highest.')
    out = []
    for i, (nm, f) in enumerate(zip(notes, fs)):
        A = AREA_LOW + (AREA_HIGH - AREA_LOW) * (i / (len(notes) - 1) if len(notes) > 1 else 0)
        F = math.sqrt(A / k)
        a = math.sqrt(A / math.pi)
        air = PP.C / (4 * f) - PP.END * a
        rim = -RAISE if shape == 'hex-zigzag' and i % 2 else 0.0
        out.append(dict(note=nm, f=f, A=A, F=F, Fo=F + 2 * T, a=a, air=air, rim=rim,
                        length=air + PP.PLUG_LAYERS * T + PP.SPARE))
    return out


def centres(pipes, shape):
    """Each tube's axis, (y, z): each one placed against the last across the
    glued face, so the two faces lie in one plane and share a centre line."""
    _, _, glue = SHAPES[shape]
    cs = [(0.0, 0.0)]
    for i in range(1, len(pipes)):
        nv = normal(glue[(i - 1) % len(glue)])
        d = (pipes[i - 1]['Fo'] + pipes[i]['Fo']) / 2
        y, z = cs[-1]
        cs.append((y + d * nv[1], z + d * nv[2]))
    return cs


def tube(n, phi, Fo, length):
    """The tube's faces: the axis x, from the rim at 0 to the bottom."""
    R = Fo / 2 / math.cos(math.pi / n)
    ring = []
    for k in range(n):
        t = math.radians(phi + (k + 0.5) * 360 / n)
        ring.append((R * math.sin(t), -R * math.cos(t)))
    verts = [(x, y, z) for y, z in ring for x in (0.0, length)]
    planes = [('rim', (-1, 0, 0), 0.0), ('bottom', (1, 0, 0), length)]
    for k in range(n):
        planes.append((f'face {k}', normal(phi + k * 360 / n), Fo / 2))
    return P.solid(planes, verts)


def facing(n, phi, toward):
    """The faces whose normals point most nearly along `toward` (+1 back,
    -1 front): one for a flat, two for a corner."""
    zs = [normal(phi + k * 360 / n)[2] * toward for k in range(n)]
    best = max(zs)
    return [k for k, z in enumerate(zs) if z > best - 1e-6]


def build_tube(p, idx, shape, edge, c, num):
    n, phi, _ = SHAPES[shape]
    faces = tube(n, phi, p['Fo'], p['length'])
    ends = [i for i, f in enumerate(faces) if f.name in ('rim', 'bottom')]
    walls = [i for i in range(len(faces)) if i not in ends]
    back = [walls[k] for k in facing(n, phi, +1)]
    front = [walls[k] for k in facing(n, phi, -1)]

    def owner_at(v):
        return next(i for i in ends if abs(P.dot(v, faces[i].n) - faces[i].off) < 1e-6)

    def force(E, s0, s1, o):
        """The ends are open, so a face owns all of its ends; a notched back
        face owns both its edges near the rim, to keep its width round the U."""
        if E['a'] in ends or E['b'] in ends:
            return E['b'] if E['a'] in ends else E['a']
        if edge == 'notch' and back[0] in (E['a'], E['b']):
            x0 = min(E['P'][0] + s * (E['Q'][0] - E['P'][0]) / E['L'] for s in (s0, s1))
            if x0 < RIM_OWN:
                return back[0]
        return o
    es = P.edges(faces)
    cellmap = {id(E): P.cells(E, owner_at, force) for E in es}
    parts = []
    face_in = p['F'] * math.tan(math.pi / n)
    face_out = p['Fo'] * math.tan(math.pi / n)
    for i in walls:
        f = faces[i]
        O = P.mul(f.n, f.off)
        fr = P.frame(f, U=(1.0, 0.0, 0.0), O=O)
        o = P.outline(i, f, fr, es, cellmap)
        part = Part(f'{p["note"]} wall', next(num), o, '')
        part.strokes = []
        part.pipe = idx
        theta = math.degrees(math.atan2(f.n[1], -f.n[2]))

        def on_face(x, s):
            q = P.add(P.add((x, 0.0, 0.0), P.mul(f.n, f.off)), P.mul(tangent(theta), s))
            return P.to2d(fr, q)
        if i == front[0]:
            part.strokes.append([on_face(p['air'], -face_in / 2), on_face(p['air'], face_in / 2)])
            part.front = True
        if i in back and edge == 'chamfer':
            part.strokes.append([on_face(PP.CHAMFER_DOWN, -face_out / 2 + 1.0),
                                 on_face(PP.CHAMFER_DOWN, face_out / 2 - 1.0)])
        if i == back[0] and edge == 'notch':
            w = NOTCH_OF * face_in
            depth = w / 2 + PP.NOTCH_PAST
            vc = on_face(0.0, 0.0)[1]
            rim = [k for k, q in enumerate(o) if abs(q[0]) < 1e-6]
            # the rim is one run of the outline along u = 0; the U goes in it
            k = next(k for k in rim if (k + 1) % len(o) in rim)
            a, b = o[k], o[(k + 1) % len(o)]
            u_ = PP.half_round_notch(vc, w, depth)
            if a[1] < b[1]:
                u_ = list(reversed(u_))
            part.outline = o[:k + 1] + u_ + o[k + 1:]
            part.strokes.append(PP.notch_bevel(vc, w, depth))
            part.notch = (w, depth, face_in, face_out)
        part.edge_back = i in back
        Mw = P.matrix(fr)
        part.m = [[Mw[0][0], Mw[0][1], Mw[0][2], Mw[0][3] + p['rim']],
                  [Mw[1][0], Mw[1][1], Mw[1][2], Mw[1][3] + c[0]],
                  [Mw[2][0], Mw[2][1], Mw[2][2], Mw[2][3] + c[1]]]
        xs = [q[0] for q in part.outline]
        ys = [q[1] for q in part.outline]
        part.x0, part.y0 = min(xs), min(ys)
        part.w, part.h = max(xs) - part.x0, max(ys) - part.y0
        parts.append(part)
    Rp = (p['F'] + INTERF) / 2 / math.cos(math.pi / n)
    disc = []
    for k in range(n):
        t = math.radians(phi + (k + 0.5) * 360 / n)
        disc.append((Rp * math.sin(t), -Rp * math.cos(t)))
    for k in range(PP.PLUG_LAYERS):
        g = Part(f'plug, {p["note"]}', next(num), disc,
                 f'press-fit into the {p["note"]} pipe from below, up to its blue line')
        g.strokes = []
        g.plug = True
        g.pipe = idx
        g.m = [[0, 0, 1, p['rim'] + p['air'] + k * T], [1, 0, 0, c[0]], [0, 1, 0, c[1]]]
        parts.append(g)
    vol, polys = P.inner(faces)
    return parts, dict(faces=faces, ends=ends, end_poly=polys[ends[0]], face_in=face_in,
                       face_out=face_out, back=back, front=front)


def build(pipes, shape, edge):
    if edge == 'notch' and len(facing(*SHAPES[shape][:2], +1)) > 1:
        raise SystemExit(f'error: --shape={shape} has a corner at the back, and a notch '
                         f'cannot be cut into a corner. Its edges are chamfer and plain.')
    cs = centres(pipes, shape)
    num = iter(range(1, 9999))
    parts, infos = [], []
    for i, (p, c) in enumerate(zip(pipes, cs)):
        ps, info = build_tube(p, i, shape, edge, c, num)
        parts += ps
        infos.append(info)
    return parts, dict(centres=cs, tubes=infos)


def outsides(pipes, cs, shape):
    """Each tube's outside, its corners in (y, z)."""
    n, phi, _ = SHAPES[shape]
    out = []
    for p, c in zip(pipes, cs):
        R = p['Fo'] / 2 / math.cos(math.pi / n)
        out.append([(c[0] + R * math.sin(math.radians(phi + (k + 0.5) * 360 / n)),
                     c[1] - R * math.cos(math.radians(phi + (k + 0.5) * 360 / n)))
                    for k in range(n)])
    return out


def poly_area(pts):
    return abs(sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1)
                   in zip(pts, pts[1:] + pts[:1]))) / 2


def checks(pipes, parts, info, shape, edge, sheets):
    out = []

    def note(ok, what, detail):
        out.append((ok, what, detail))
    n, phi, glue = SHAPES[shape]
    tubes = info['tubes']
    fronts = [p for p in parts if getattr(p, 'front', False)]
    marks = sorted((p.pipe, p.strokes[0][0][0]) for p in fronts)
    note(len(marks) == len(pipes) and all(abs(u - pipes[i]['air']) < 1e-6 for i, u in marks),
         'every blue line on a front is its plug\'s face',
         f'{len(marks)} lines, {min(u for _, u in marks):.1f} to {max(u for _, u in marks):.1f}mm from the rim')
    worst = 0.0
    for t, p in zip(tubes, pipes):
        sec = [(q[1], q[2]) for q in t['end_poly']]
        a = poly_area(sorted(sec, key=lambda q: math.atan2(q[1], q[0])))
        worst = max(worst, abs(a / p['A'] - 1))
    note(worst < 1e-6, 'every bore is the area the pipe is tuned for',
         f'{pipes[0]["A"]:.0f} to {pipes[-1]["A"]:.0f}mm^2, {pipes[0]["F"]:.2f} to '
         f'{pipes[-1]["F"]:.2f}mm across the flats')
    plugs = [p for p in parts if getattr(p, 'plug', False)]
    sizes = []
    for g in plugs:
        F = 2 * max(abs(P.dot((0, x, y), normal(phi + k * 360 / n))) for x, y in g.outline
                    for k in range(1))
        sizes.append(F - pipes[g.pipe]['F'])
    note(len(plugs) == PP.PLUG_LAYERS * len(pipes) and all(abs(s - INTERF) < 1e-6 for s in sizes),
         'every plug is a press-fit in its bore', f'{len(plugs)} layers, {INTERF:g}mm over across the flats')
    room = min(p['length'] - p['air'] - PP.PLUG_LAYERS * T for p in pipes)
    note(abs(room - PP.SPARE) < 1e-9, 'every plug can come down as well as go up',
         f'{room:g}mm of pipe below each plug')
    # glued faces: coplanar, and how much of each they share
    cs = info['centres']
    share, plane = math.inf, 0.0
    for i in range(1, len(pipes)):
        nv = normal(glue[(i - 1) % len(glue)])
        a = cs[i - 1][0] * nv[1] + cs[i - 1][1] * nv[2] + pipes[i - 1]['Fo'] / 2
        b = cs[i][0] * nv[1] + cs[i][1] * nv[2] - pipes[i]['Fo'] / 2
        plane = max(plane, abs(a - b))
        share = min(share, min(tubes[i - 1]['face_out'], tubes[i]['face_out'])
                    - 2 * P.clear(180 - 360 / n))
    note(plane < 1e-6 and share >= 6.0, 'every glued face lies flat on its neighbour\'s',
         f'{len(pipes) - 1} glue joints, at least {share:.1f}mm wide of solid face')
    rings = outsides(pipes, cs, shape)

    def gap(A, B):
        """Separating-axis distance between two convex rings: > 0 apart,
        0 touching, < 0 overlapping."""
        best = -math.inf
        for ring in (A, B):
            for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
                L = math.hypot(x1 - x0, y1 - y0)
                nx, ny = (y1 - y0) / L, -(x1 - x0) / L
                pa = [x * nx + y * ny for x, y in A]
                pb = [x * nx + y * ny for x, y in B]
                best = max(best, min(pb) - max(pa), min(pa) - max(pb))
        return best
    worst = min(gap(rings[i], rings[j]) for i in range(len(rings))
                for j in range(i + 1, len(rings)))
    note(worst > -1e-6, 'no two tubes overlap', f'{len(rings) * (len(rings) - 1) // 2} pairs, '
         + ('neighbours touching, none overlapping' if worst > -1e-6
            else f'two overlap by {-worst:.2f}mm'))
    web = min(t['face_out'] for t in tubes) - 2 * P.clear(180 - 360 / n)
    note(web >= 2 * MIN_WEB, 'a face keeps ply between its two joints',
         f'{web:.2f}mm where both neighbours own the edge, at {180 - 360 / n:g} degrees')
    if edge == 'notch':
        nts = [p.notch for p in parts if hasattr(p, 'notch')]
        # the notch is a cut, so MIN_WEB of ply either side of it inside; the
        # line is an engrave, and only has to land on the face, LINE_IN in
        ok = all(w + 2 * MIN_WEB <= fi and w + 2 * PP.CHAMFER_DOWN <= fo - 2 * LINE_IN
                 and d + PP.CHAMFER_DOWN + MIN_WEB <= RIM_OWN for w, d, fi, fo in nts)
        note(len(nts) == len(pipes) and ok, 'every notch and its bevel line fit the back face',
             f'{max(w for w, *_ in nts):.1f}mm wide, {max(d for _, d, *_ in nts):.1f}mm deep, '
             f'the line {max(w for w, *_ in nts) + 2 * PP.CHAMFER_DOWN:.1f}mm across')
    if edge == 'chamfer':
        lines = sum(len(p.strokes) - (1 if getattr(p, 'front', False) else 0) for p in parts
                    if getattr(p, 'edge_back', False))
        note(lines == len(pipes) * len(tubes[0]['back']), 'every back face has its bevel line',
             f'{lines} lines, {PP.CHAMFER_DOWN:g}mm below the rim')
    ratio = min(p['air'] / (2 * p['a']) for p in pipes)
    note(ratio >= 8, 'no pipe is fat for its length', f'{ratio:.1f} diameters at the least')
    longest = max(max(p.w, p.h) for p in parts)
    note(longest + KERF + 2 * MARGIN <= BED[0], 'the longest part fits the bed',
         f'{longest + KERF:.1f}mm with the kerf, against {BED[0] - 2 * MARGIN:g}')
    note(all(s['w'] <= BED[0] and s['h'] <= BED[1] for s in sheets), 'every sheet fits the bed',
         f'{len(sheets)} sheet(s), largest {max(s["w"] for s in sheets):.0f} x '
         f'{max(s["h"] for s in sheets):.0f}mm')
    placed = sum(len(s['parts']) for s in sheets)
    note(placed == len(parts), 'every part is on a sheet', f'{placed} of {len(parts)}')
    return out


def svg_sheet(k, n, s, shape, edge):
    orange, black, blue = [], [], []
    for p, sx, sy in s['parts']:
        dx, dy = sx - p.x0 + KERF / 2, sy - p.y0 + KERF / 2
        mv = lambda pts: [(x + dx, y + dy) for x, y in pts]
        black.append(path(mv(offset(p.outline, KERF / 2))))
        for st in p.strokes:
            blue.append(path(mv(st), close=False))
    eng = {'chamfer': 'the plugs\' places on the front faces and the bevel\'s line on the back',
           'plain': 'the plugs\' places on the front faces',
           'notch': 'the plugs\' places on the front faces and the bevel\'s line round every notch'}[edge]
    desc = (f'1 user unit = 1mm. Panpipes, E3 to E4, {shape}, {edge} edge, sheet {k} of {n}. '
            f'3mm ply, drawn for a {KERF:g}mm kerf. Blue {BLUE} engraves {eng}, on the faces\' '
            f'outsides, which lie face up. Then black {BLACK} frees the parts.')
    g = lambda col, items: (
        f'  <g fill="none" stroke="{col}" stroke-width="0.2">\n'
        + ''.join(f'    <path d="{i}"/>\n' for i in items) + '  </g>\n') if items else ''
    return (f'<?xml version="1.0" encoding="utf-8"?>\n'
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{s["w"]:.2f}mm" '
            f'height="{s["h"]:.2f}mm" viewBox="0 0 {s["w"]:.2f} {s["h"]:.2f}">\n'
            f'<title>Panpipes E3 to E4, {shape}, {edge} edge, sheet {k} of {n}</title>\n'
            f'<desc>{desc}</desc>\n'
            + g(BLUE, blue) + g(ORANGE, orange) + g(BLACK, black) + '</svg>\n')


def page(path_out, notes):
    """The 3D view: each shape, chamfer edge, plugs at their lines."""
    import turn3d as t
    up = [[0, -1, 0, 0], [0, 0, 1, 0], [-1, 0, 0, 0], [0, 0, 0, 1]]
    designs = {}
    for shape in SHAPES:
        pipes = design(notes, shape)
        parts, info = build(pipes, shape, 'chamfer')
        out = []
        for p in parts:
            prt = {'o': p.outline, 'h': [], 'e': p.strokes}
            m = [row[:] for row in p.m] + [[0, 0, 0, 1]]
            c = info['centres'][p.pipe]
            if getattr(p, 'plug', False):
                ex = (1.2, 0, 0)
            else:
                # each face out from its own tube's axis
                cx = (m[1][3] - c[0], m[2][3] - c[1])
                L = math.hypot(*cx) or 1
                ex = (0, 0.9 * cx[0] / L, 0.9 * cx[1] / L)
            ex = (-ex[1], ex[2], -ex[0])
            out.append(t.placed(prt, t.chain(up, m), ex,
                                'dark' if getattr(p, 'plug', False) else 'ply'))
        designs[shape] = {'parts': out, 'ex': 30, 'view': [-0.6, -0.3, 4.2]}
    t.write(path_out, 'Tube panpipes in 3D', designs,
            'Drag to turn, scroll or pinch to zoom; draw Explode to take it apart. Every part is '
            'the cut files’ own outline: each pipe a tube of finger-jointed strips, glued to '
            'its neighbours, with its plug, darker, at the blue line on its front. The buttons '
            'switch between the octagonal row, the hexagonal zigzag and the hexagonal row, each '
            'with the chamfer edge’s line along the back of the rim.',
            'tube_panpipes.py', spin=True)


def main():
    global KERF
    out, write, shape, edge, notes, page_to = None, True, 'octagon-row', 'chamfer', PP.NOTES, None
    for x in sys.argv[1:]:
        if x == '--no-write':
            write = False
        elif x.startswith('--out='):
            out = x.split('=', 1)[1]
        elif x.startswith('--shape='):
            shape = x.split('=', 1)[1]
            if shape not in SHAPES:
                raise SystemExit(f'error: --shape={shape}: the shapes are {", ".join(SHAPES)}.')
        elif x.startswith('--edge='):
            edge = x.split('=', 1)[1]
            if edge not in EDGES:
                raise SystemExit(f'error: --edge={edge}: the edges are {", ".join(EDGES)}.')
        elif x.startswith('--notes='):
            notes = [s for s in x.split('=', 1)[1].split(',') if s]
        elif x.startswith('--raise='):
            global RAISE
            RAISE = float(x.split('=', 1)[1])
        elif x.startswith('--end='):
            PP.END = float(x.split('=', 1)[1])
        elif x.startswith('--kerf='):
            KERF = SF.set_kerf(x)
        elif x.startswith('--page-to='):
            page_to = x.split('=', 1)[1]
        else:
            raise SystemExit(f'error: {x} is not a flag this generator reads. It takes --out=, '
                             f'--shape=, --edge=, --notes=, --raise=, --end=, --kerf=, --page-to= and --no-write.')
    if page_to:
        page(page_to, notes)
        print(f'  wrote {os.path.basename(page_to)}')
        return 0
    if write and not out:
        raise SystemExit('error: --out=DIR, or --no-write to run the checks only.')
    pipes = design(notes, shape)
    parts, info = build(pipes, shape, edge)
    sheets = pack(parts)
    res = checks(pipes, parts, info, shape, edge, sheets)
    pts = [q for r in outsides(pipes, info['centres'], shape) for q in r]
    width = max(q[0] for q in pts) - min(q[0] for q in pts)
    depth = max(q[1] for q in pts) - min(q[1] for q in pts)
    print(f'panpipes, {notes[0]} to {notes[-1]}, {shape}, {edge} edge: {len(pipes)} tubes of '
          f'{SHAPES[shape][0]} strips, the set {width:.1f}mm across, {depth:.1f}mm deep'
          + (f'; the back row {RAISE:g}mm taller at the rim' if shape == 'hex-zigzag' else ''))
    for p in pipes:
        print(f'  {p["note"]:4} {p["f"]:7.2f}Hz  {p["F"]:5.2f}mm across the flats  plug face '
              f'{p["air"]:6.1f}mm from the rim, tube {p["length"]:6.1f}mm')
    bad = 0
    for ok, what, detail in res:
        print(f'  {"pass" if ok else "FAIL"}  {what:56} {detail}')
        bad += not ok
    if bad:
        print(f'\n  {bad} check(s) failed. Nothing written.')
        return 1
    if write:
        os.makedirs(out, exist_ok=True)
        stem = f'panpipes-{notes[0].replace("#", "s")}-{shape}-{edge}'
        for k, s in enumerate(sheets, 1):
            f = os.path.join(out, f'{stem}-sheet{k}-cut-files.svg')
            open(f, 'w').write(svg_sheet(k, len(sheets), s, shape, edge))
            print(f'  wrote {os.path.basename(f)}  {s["w"]:.0f} x {s["h"]:.0f}mm, '
                  f'{len(s["parts"])} parts')
    return 0


if __name__ == '__main__':
    sys.exit(main())
