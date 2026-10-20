#!/usr/bin/env python3
"""Cut files for a low raft of panpipes, E3 to E4 in E natural minor: eight
closed pipes of rectangular bore in 3mm ply, each tuned by a press-fit plug.

    python3 panpipes.py --no-write                # the checks, write nothing
    python3 panpipes.py --out=DIR                 # the default build into DIR
    python3 panpipes.py --edge=notch --out=DIR    # another blowing edge
    python3 panpipes.py --notes=G3,A3,B3,D4 ...   # other pipes, low to high
    python3 panpipes.py --end=0.7 ...             # the open end's correction
    python3 panpipes.py --kerf=0.18 ...           # for a laser with that kerf
    python3 panpipes.py --page-to=turn.html       # the 3D view, every edge

THE RAFT is two plates, the front and the back, with a wall between every
pair of pipes and one at each end. An inner wall has FINGER wide tabs,
TAB_PITCH apart, through slots in both plates; the two end walls have the
square flute's finger joint, a finger every 2 x FINGER, in notches in the
plates' long edges, flush with the outside. The tops are
flush, the rim the player blows across; the bottoms step down with the pipes,
so the plates come out as a staircase.

EACH PIPE is a closed pipe, a quarter wave long. From the rim:

  air      L = C / 4f - END x a, where a is the radius of the circle with the
           bore's area: the column the plug's face stops
  plug     PLUG_LAYERS rectangles of ply, press-fit into the bore from the
           open bottom, up to the blue line on the front plate
  spare    SPARE of empty pipe below the plug, so it can come down and flatten
           the note as well as go up and sharpen it

The bore is DEPTH from front to back in every pipe, so the plates stay
plain, and narrows from W_LOW to W_HIGH across the raft, so the high pipes
are not fat for their length.

THE BLOWING EDGE is the back plate's top edge, the one the breath strikes
after crossing the bore. --edge= picks how it is made:

  chamfer  the default. The plate as cut, with a blue line engraved across its
           outside CHAMFER_DOWN below the rim: sand a 45 degree bevel down to
           it, leaving CHAMFER_LEFT of edge on the inside of the bore
  plain    the plate as cut, a 3mm square edge. It plays, breathier
  notch    a U notch cut into the rim over every pipe, half the pipe wide, as
           a quena's, with a blue line engraved round each one CHAMFER_DOWN
           out from it on the outside: file a 45 degree bevel down to the line,
           as a quena's notch is bevelled, leaving CHAMFER_LEFT of edge
  strip    the back plate stops STRIP_H short of the rim, and a strip of
           STRIP_T stock, cut from its own sheet, is glued along it with its
           inner face flush with the bore, so the edge is STRIP_T thick

The front plate's top, where the lower lip rests, is rounded with sandpaper
in every build; nothing here draws that.

The kerf, the finger joint, the sheet packing and the inks are the square
flute's, imported from square_flute.py, and the mitred offset is the
four-hole ocarinas': both repositories are expected beside this one, with
swept-curve for the path writer and lasermade-tools for the 3D view.
"""
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
for sib in ('square-flute', 'swept-curve', 'four-hole-ocarinas', 'lasermade-tools'):
    sys.path.insert(0, os.path.join(HERE, '..', sib))
import naf_model as NM                                         # noqa: E402
import square_flute as SF                                      # noqa: E402
from four_hole_ocarinas import offset                          # noqa: E402
from square_flute import Part, pack, finger_side, fingers_x    # noqa: E402
from ribbon_bore import path                                   # noqa: E402

T, KERF = SF.T, SF.KERF
FINGER, FINGER_END = SF.FINGER, SF.FINGER_END
MIN_WEB = SF.MIN_WEB
BED, MARGIN = SF.BED, SF.MARGIN
BLUE, ORANGE, BLACK = SF.BLUE, SF.ORANGE, SF.BLACK
C = NM.C * 1000.0                  # mm/s, air warm from the breath, as the flutes'
A4 = 440.0

NOTES = ['E3', 'F#3', 'G3', 'A3', 'B3', 'C4', 'D4', 'E4']
DEPTH = 20.0                       # bore, front to back, every pipe
W_LOW, W_HIGH = 22.0, 16.0         # bore across the raft, lowest and highest pipe
END = 0.61                         # open-end correction, x the equivalent radius
PLUG_LAYERS = 3
INTERF = SF.INTERF_ACROSS          # the plug's press-fit, each way across
SPARE = 12.0                       # empty pipe below the plug, to flatten into
EDGES = ('chamfer', 'plain', 'notch', 'strip')
CHAMFER_LEFT = 1.0                 # edge left on the inside of the bore
CHAMFER_DOWN = T - CHAMFER_LEFT    # how far down the outside a 45 degree bevel runs
NOTCH_OF = 0.5                     # notch width, of the pipe's width
NOTCH_PAST = 1.0                   # notch depth past its half-round bottom
STRIP_H, STRIP_T = 5.0, 1.5
STEPS = 16                         # points on a half circle
# The inner walls' tabs, through slots in both plates. The end walls keep the
# square flute's fingers, a finger and a space each 2T, because those show;
# at that pitch an inner wall would carry 22 tabs a side, and 44 tabs pressed
# home at once is a wall that will not go in straight.
TAB_PITCH = 40.0
OVER_CLEAR = 4.0                   # wall either side of a tab below a shorter pipe

NAMES = {'C': 0, 'D': 2, 'E': 4, 'F': 5, 'G': 7, 'A': 9, 'B': 11}


def freq(note):
    """'F#3' -> Hz, equal tempered from A4."""
    try:
        k = NAMES[note[0].upper()]
        rest = note[1:]
        while rest and rest[0] in '#b':
            k += 1 if rest[0] == '#' else -1
            rest = rest[1:]
        midi = 12 * (int(rest) + 1) + k
    except (KeyError, ValueError, IndexError):
        raise SystemExit(f'error: {note} is not a note, e.g. E3 or F#3.')
    return A4 * 2 ** ((midi - 69) / 12)


def design(notes):
    """Each pipe's bore width, air column and cut length, low to high."""
    fs = [freq(n) for n in notes]
    if any(b <= a for a, b in zip(fs, fs[1:])):
        raise SystemExit('error: --notes= runs from the lowest pipe to the highest.')
    n = len(notes)
    pipes = []
    for i, (nm, f) in enumerate(zip(notes, fs)):
        w = W_LOW + (W_HIGH - W_LOW) * (i / (n - 1) if n > 1 else 0)
        a = math.sqrt(w * DEPTH / math.pi)
        air = C / (4 * f) - END * a
        pipes.append(dict(note=nm, f=f, w=w, a=a, air=air,
                          length=air + PLUG_LAYERS * T + SPARE))
    return pipes


def layout(pipes):
    """Where each wall stands across the raft, and how long it is: wall j is
    between pipes j-1 and j, as long as the longer of them."""
    ys, y = [], 0.0
    for p in pipes:
        ys.append(y)
        y += T + p['w']
    ys.append(y)
    lens = [pipes[0]['length']] + [p['length'] for p in pipes]
    return ys, lens, y + T


def tabs_x(L):
    """Where an inner wall's tabs go along L: FINGER wide, TAB_PITCH apart,
    centred, FINGER_END clear of each end, as the end walls' fingers are."""
    n = int((L - 2 * FINGER_END - FINGER) // TAB_PITCH) + 1
    span = (n - 1) * TAB_PITCH
    return [L / 2 - span / 2 + i * TAB_PITCH for i in range(n)]


def overhang_x(x0, x1):
    """Tabs along the stretch of an inner wall below its shorter pipe, x0 to
    x1: there the plate's staircase edge runs beside the wall, so they are
    fingers in notches in that edge, not tabs in slots. One in the middle,
    or more at TAB_PITCH if the stretch is long; none if it is too short to
    keep MIN_WEB of wall either side."""
    L = x1 - x0
    if L < FINGER + 2 * OVER_CLEAR:
        return []
    n = int((L - 2 * OVER_CLEAR - FINGER) // TAB_PITCH) + 1
    span = (n - 1) * TAB_PITCH
    return [x0 + L / 2 - span / 2 + i * TAB_PITCH for i in range(n)]


def joints(pipes, lens):
    """Each wall's tab positions: the end walls' fingers; an inner wall's
    tabs in slots down its shorter pipe, and in notches below it."""
    last = len(lens) - 1
    out = []
    for j, L in enumerate(lens):
        if j in (0, last):
            out.append(fingers_x(L))
        else:
            short = pipes[j]['length']
            out.append(tabs_x(short) + overhang_x(short, L))
    return out


def half_round_notch(yc, w, depth):
    """A U into the rim (x = 0) centred at yc: straight sides, a half-round
    bottom, `depth` deep at its centre. Listed from the +y side to the -y
    side, as the outline meets it walking down the rim toward -y."""
    r = w / 2
    s = depth - r
    pts = [(0.0, yc + r)]
    for k in range(STEPS + 1):
        t = math.pi * k / STEPS
        pts.append((s + r * math.sin(t), yc + r * math.cos(t)))
    pts.append((0.0, yc - r))
    return pts


def notch_bevel(yc, w, depth):
    """The bevel's line round a notch: the U's own outline moved CHAMFER_DOWN
    out into the plate, from the rim on one side round to the rim on the other,
    an open line to engrave."""
    r = w / 2 + CHAMFER_DOWN
    s = depth - w / 2
    pts = [(0.0, yc + r)]
    for k in range(STEPS + 1):
        t = math.pi * k / STEPS
        pts.append((s + r * math.sin(t), yc + r * math.cos(t)))
    pts.append((0.0, yc - r))
    return pts


def plate(name, num, pipes, ys, lens, Y, edge, back):
    """A plate as seen from its outside face, x down the pipes from the rim,
    y across the raft from the lowest pipe. Slots for the inner walls, the
    end walls' fingers notched into its two long edges."""
    n = len(pipes)
    xs = joints(pipes, lens)
    top_x = STRIP_H if (back and edge == 'strip') else 0.0
    # the edge at y = 0: wall 0, notched for its fingers, rim to bottom
    o = [(top_x, 0.0)]
    for x in xs[0]:
        o += [(x - FINGER / 2, 0.0), (x - FINGER / 2, T), (x + FINGER / 2, T), (x + FINGER / 2, 0.0)]
    o.append((lens[0], 0.0))
    # the staircase, down the pipes' bottoms
    for j in range(1, n):
        o.append((lens[j], ys[j] + T))
        for x in sorted((x for x in xs[j] if x > pipes[j]['length']), reverse=True):
            o += [(x + FINGER / 2, ys[j] + T), (x + FINGER / 2, ys[j]),
                  (x - FINGER / 2, ys[j]), (x - FINGER / 2, ys[j] + T)]
        o.append((pipes[j]['length'], ys[j] + T))
    # the edge at y = Y: the last wall, bottom to rim
    o.append((lens[n], Y))
    for x in reversed(xs[n]):
        o += [(x + FINGER / 2, Y), (x + FINGER / 2, Y - T), (x - FINGER / 2, Y - T), (x - FINGER / 2, Y)]
    o.append((top_x, Y))
    # the rim, walked back toward y = 0
    if back and edge == 'notch':
        for i in reversed(range(n)):
            yc = ys[i] + T + pipes[i]['w'] / 2
            w = NOTCH_OF * pipes[i]['w']
            o += half_round_notch(yc, w, w / 2 + NOTCH_PAST)
    o = [q for k, q in enumerate(o) if k == 0 or q != o[k - 1]]
    p = Part(name, num, o, '')
    p.strokes = []
    for j in range(1, n):
        for x in (x for x in xs[j] if x < pipes[j]['length']):
            p.cuts.append(('poly', SF.rect(x - FINGER / 2, ys[j], x + FINGER / 2, ys[j] + T)))
    if back and edge == 'chamfer':
        p.strokes.append([(CHAMFER_DOWN, 0.0), (CHAMFER_DOWN, Y)])
    if back and edge == 'notch':
        for i in range(n):
            yc = ys[i] + T + pipes[i]['w'] / 2
            w = NOTCH_OF * pipes[i]['w']
            p.strokes.append(notch_bevel(yc, w, w / 2 + NOTCH_PAST))
    if not back:
        for i, pp in enumerate(pipes):
            p.strokes.append([(pp['air'], ys[i] + T), (pp['air'], ys[i + 1])])
    return p


def mirror(p, Y):
    """The part flipped across the raft, y -> Y - y: the front plate, drawn as
    seen from outside, is the back plate's mirror."""
    q = Part(p.name, p.num, [(x, Y - y) for x, y in reversed(p.outline)], p.note)
    q.cuts = [('poly', [(x, Y - y) for x, y in reversed(c[1])]) for c in p.cuts]
    q.strokes = [[(x, Y - y) for x, y in s] for s in p.strokes]
    return q


def build(pipes, edge):
    """Every part, in its flat coordinates, with p.m taking it from (u, v,
    up-from-the-sheet) into the raft: x down the pipes from the rim, y across
    from the lowest pipe, z from the front plate's inside face to the back."""
    ys, lens, Y = layout(pipes)
    n = len(pipes)
    num = iter(range(1, 999))
    parts = []
    front = mirror(plate('front plate', next(num), pipes, ys, lens, Y, edge, False), Y)
    front.note = ('engraved face out, toward the player; the blue lines are the plugs\' '
                  'faces; round its top edge where the lip rests')
    front.m = [[1, 0, 0, 0], [0, -1, 0, Y], [0, 0, -1, 0]]
    parts.append(front)
    back = plate('back plate', next(num), pipes, ys, lens, Y, edge, True)
    back.note = {'chamfer': 'engraved face out; sand the rim to a 45 degree bevel down to the blue line',
                 'plain': 'its top edge is the blowing edge, as cut',
                 'notch': 'engraved face out; file each notch to a 45 degree bevel down to its blue line',
                 'strip': f'stops {STRIP_H:g}mm short of the rim; the strip glues along that edge'}[edge]
    back.m = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, DEPTH]]
    parts.append(back)
    for j in range(n + 1):
        w = finger_side(lens[j], DEPTH, joints(pipes, lens)[j], next(num))
        w.name = 'end wall' if j in (0, n) else 'wall'
        w.note = 'tabs through the slots in both plates'
        w.strokes = []
        # u down the pipe, v front to back, up-from-the-sheet across the raft
        w.m = [[1, 0, 0, 0], [0, 0, -1, ys[j] + T], [0, 1, 0, 0]]
        parts.append(w)
    for i, pp in enumerate(pipes):
        for k in range(PLUG_LAYERS):
            g = Part(f'plug, {pp["note"]}', next(num),
                     SF.rect(0, 0, pp['w'] + INTERF, DEPTH + INTERF),
                     f'press-fit into the {pp["note"]} pipe from below, up to its blue line')
            g.strokes = []
            g.m = [[0, 0, 1, pp['air'] + k * T], [1, 0, 0, ys[i] + T - INTERF / 2],
                   [0, 1, 0, -INTERF / 2]]
            g.plug = True
            parts.append(g)
    strips = []
    if edge == 'strip':
        s = Part('edge strip', next(num), SF.rect(0, 0, Y, STRIP_H),
                 f'{STRIP_T:g}mm stock; glued along the back plate\'s cut-down top, '
                 f'inner face flush with the bore')
        s.strokes = []
        s.m = [[0, 1, 0, 0], [-1, 0, 0, Y], [0, 0, STRIP_T / T, DEPTH]]
        strips.append(s)
    return parts, strips, dict(ys=ys, lens=lens, Y=Y, xs=joints(pipes, lens))


def seg_dist(q, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    L2 = dx * dx + dy * dy
    t = 0 if L2 == 0 else max(0, min(1, ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / L2))
    return math.hypot(q[0] - a[0] - t * dx, q[1] - a[1] - t * dy)


def checks(pipes, parts, strips, info, edge, sheets):
    out = []

    def note(ok, what, detail):
        out.append((ok, what, detail))
    ys, lens, Y = info['ys'], info['lens'], info['Y']
    n = len(pipes)
    front = next(p for p in parts if p.name == 'front plate')
    marks = sorted(st[0][0] for st in front.strokes)
    note(len(marks) == n and all(abs(m - p['air']) < 1e-9
                                 for m, p in zip(marks, sorted(pipes, key=lambda q: q['air']))),
         'every blue line on the front is its plug\'s face',
         f'{len(marks)} lines, {marks[0]:.1f} to {marks[-1]:.1f}mm from the rim')
    bores = [ys[i + 1] - ys[i] - T for i in range(n)]
    note(all(abs(b - p['w']) < 1e-9 for b, p in zip(bores, pipes)),
         'the walls stand a bore apart', f'{min(bores):.1f} to {max(bores):.1f}mm across, '
         f'{DEPTH:g}mm deep')
    plugs = [p for p in parts if getattr(p, 'plug', False)]
    note(len(plugs) == PLUG_LAYERS * n and all(
        abs(g.w - pp['w'] - INTERF) < 1e-9 and abs(g.h - DEPTH - INTERF) < 1e-9
        for g, pp in zip(plugs, [pp for pp in pipes for _ in range(PLUG_LAYERS)])),
        'every plug is a press-fit in its bore', f'{len(plugs)} layers, {INTERF:g}mm over each way')
    room = min(p['length'] - p['air'] - PLUG_LAYERS * T for p in pipes)
    note(abs(room - SPARE) < 1e-9, 'every plug can come down as well as go up',
         f'{room:g}mm of pipe below each plug')
    lens_ok = all(lens[j] >= max(p['length'] for p in pipes[max(0, j - 1):j + 1]) - 1e-9
                  for j in range(n + 1))
    note(lens_ok, 'every pipe is walled to its bottom', f'{n + 1} walls')
    first = min(min(xs) for xs in info['xs'] if xs)
    note(first - FINGER / 2 >= STRIP_H + MIN_WEB, 'the rim is clear of every joint',
         f'the first finger {first - FINGER / 2:.1f}mm down, against {STRIP_H:g}mm of strip')
    worst, where = math.inf, ''
    for pl in (p for p in parts if p.name.endswith('plate')):
        ring = pl.outline + pl.outline[:1]
        for c in pl.cuts:
            for q in c[1]:
                inside = sum(1 for a_, b_ in zip(ring, ring[1:])
                             if (a_[1] > q[1]) != (b_[1] > q[1]) and
                             q[0] < a_[0] + (q[1] - a_[1]) * (b_[0] - a_[0]) / (b_[1] - a_[1])) % 2
                d = min(seg_dist(q, a_, b_) for a_, b_ in zip(ring, ring[1:]))
                if not inside:
                    d = -d
                if d < worst:
                    worst, where = d, pl.name
    note(worst >= MIN_WEB, 'every slot keeps ply between it and the plate\'s edge',
         f'{worst:.2f}mm at the least, on the {where}, against {MIN_WEB:g}mm')
    for j in range(1, n):
        short = pipes[j]['length']
        if any(x - FINGER / 2 - MIN_WEB < short < x + FINGER / 2 + MIN_WEB for x in info['xs'][j]):
            note(False, 'no tab straddles a pipe\'s bottom', f'wall {j}')
    if edge == 'notch':
        widest = max(NOTCH_OF * p['w'] for p in pipes)
        deepest = max(NOTCH_OF * p['w'] / 2 + NOTCH_PAST for p in pipes)
        note(all(NOTCH_OF < 1 for p in pipes) and deepest + MIN_WEB <= first - FINGER / 2,
             'every notch stays within its pipe and clear of the joints',
             f'{widest:.1f}mm wide and {deepest:.1f}mm deep at the most')
        back = next(q for q in parts if q.name == 'back plate')
        lines = [st for st in back.strokes]
        reach = max(max(x for x, y in st) for st in lines)
        spread = max(max(y for x, y in st) - min(y for x, y in st) for st in lines)
        over = sum(1 for st, pp in zip(lines, pipes)
                   if max(y for x, y in st) - min(y for x, y in st) > pp['w'])
        note(len(lines) == n and over == 0 and reach + MIN_WEB <= first - FINGER / 2,
             'every notch has its bevel line, clear of the joints',
             f'{len(lines)} lines, {CHAMFER_DOWN:g}mm out from each notch, reaching '
             f'{reach:.1f}mm down and {spread:.1f}mm across, each within its pipe')
    if edge == 'chamfer':
        note(0 < CHAMFER_DOWN < T, 'the bevel leaves an edge inside the bore',
             f'{CHAMFER_LEFT:g}mm left of {T:g}mm, {CHAMFER_DOWN:g}mm down the outside')
    ratio = min(p['air'] / (2 * p['a']) for p in pipes)
    note(ratio >= 8, 'no pipe is fat for its length', f'{ratio:.1f} diameters at the least')
    longest = max(max(p.w, p.h) for p in parts + strips)
    note(longest + KERF + 2 * MARGIN <= BED[0], 'the longest part fits the bed',
         f'{longest + KERF:.1f}mm with the kerf, against {BED[0] - 2 * MARGIN:g}')
    note(all(s['w'] <= BED[0] and s['h'] <= BED[1] for s in sheets), 'every sheet fits the bed',
         f'{len(sheets)} sheet(s), largest {max(s["w"] for s in sheets):.0f} x '
         f'{max(s["h"] for s in sheets):.0f}mm')
    placed = sum(len(s['parts']) for s in sheets)
    note(placed == len(parts) + len(strips), 'every part is on a sheet',
         f'{placed} of {len(parts) + len(strips)}')
    return out


def svg_sheet(k, n, s, what, edge, stock):
    orange, black, blue = [], [], []
    for p, sx, sy in s['parts']:
        dx, dy = sx - p.x0 + KERF / 2, sy - p.y0 + KERF / 2
        mv = lambda pts: [(x + dx, y + dy) for x, y in pts]
        black.append(path(mv(offset(p.outline, KERF / 2))))
        for c in p.cuts:
            orange.append(path(mv(offset(c[1], -KERF / 2))))
        for st in p.strokes:
            blue.append(path(mv(st), close=False))
    eng = {'chamfer': 'the plugs\' places on the front plate and the bevel\'s line on the back',
           'strip': 'the plugs\' places on the front plate',
           'plain': 'the plugs\' places on the front plate',
           'notch': 'the plugs\' places on the front plate and the bevel\'s line round every notch on the back'}[edge]
    desc = (f'1 user unit = 1mm. Panpipes, E3 to E4, {edge} edge, sheet {k} of {n}: {what}. '
            f'{stock:g}mm ply, drawn for a {KERF:g}mm kerf. Blue {BLUE} engraves {eng}, '
            f'on the plates\' outside faces, which lie face up. Then orange {ORANGE} cuts '
            f'the slots, and black {BLACK} frees the parts.')
    g = lambda col, items: (
        f'  <g fill="none" stroke="{col}" stroke-width="0.2">\n'
        + ''.join(f'    <path d="{i}"/>\n' for i in items) + '  </g>\n') if items else ''
    return (f'<?xml version="1.0" encoding="utf-8"?>\n'
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{s["w"]:.2f}mm" '
            f'height="{s["h"]:.2f}mm" viewBox="0 0 {s["w"]:.2f} {s["h"]:.2f}">\n'
            f'<title>Panpipes E3 to E4, {edge} edge, sheet {k} of {n}</title>\n'
            f'<desc>{desc}</desc>\n'
            + g(BLUE, blue) + g(ORANGE, orange) + g(BLACK, black) + '</svg>\n')


def page(path_out, notes):
    """The 3D view: every edge as a design of its own, drawn from the same
    parts the sheets are, plugs at their blue lines."""
    import turn3d as t
    # the viewer's up is +z and it looks along +y: the rim at the top, the pipes
    # hanging below it, the front plate toward the camera
    up = [[0, -1, 0, 0], [0, 0, 1, 0], [-1, 0, 0, 0], [0, 0, 0, 1]]
    designs = {}
    for edge in EDGES:
        pipes = design(notes)
        parts, strips, info = build(pipes, edge)
        Y = info['Y']
        out = []
        for p in parts + strips:
            prt = {'o': p.outline, 'h': [c[1] for c in p.cuts], 'e': p.strokes}
            m = [row[:] for row in p.m] + [[0, 0, 0, 1]]
            # the explode slider: plates off front and back, plugs down, strip up
            ex = ((0, 0, -1.4) if p.name == 'front plate' else (0, 0, 1.4) if p.name == 'back plate'
                  else (1.2, 0, 0) if getattr(p, 'plug', False) else (-1, 0, 1.6)
                  if p.name == 'edge strip' else (0, 0, 0))
            wood = 'dark' if getattr(p, 'plug', False) or p.name == 'edge strip' else 'ply'
            ex = (-ex[1], ex[2], -ex[0])
            out.append(t.placed(prt, t.chain(up, m), ex, wood))
        designs[f'{edge} edge'] = {'parts': out, 'ex': 30, 'view': [-0.6, -0.3, 4.2]}
    t.write(path_out, 'Panpipes in 3D', designs,
            'Drag to turn; right-drag, shift-drag or move two fingers to slide the view; '
            'scroll or pinch to zoom in on whatever is under the pointer; double-click a part '
            'to turn round it, and Reset view to go back. Draw Explode to take it apart. Every part is '
            'the cut files’ own outline: the front plate toward you, the back plate behind '
            'with its blowing edge along the top, the walls between the pipes, and the plugs, '
            'darker, at their blue lines. The buttons switch between the four blowing edges.',
            'panpipes.py', spin=True)


def main():
    global KERF
    out, write, edge, notes, page_to = None, True, 'chamfer', NOTES, None
    for x in sys.argv[1:]:
        if x == '--no-write':
            write = False
        elif x.startswith('--out='):
            out = x.split('=', 1)[1]
        elif x.startswith('--edge='):
            edge = x.split('=', 1)[1]
            if edge not in EDGES:
                raise SystemExit(f'error: --edge={edge}: the edges are {", ".join(EDGES)}.')
        elif x.startswith('--notes='):
            notes = [s for s in x.split('=', 1)[1].split(',') if s]
        elif x.startswith('--end='):
            global END
            END = float(x.split('=', 1)[1])
        elif x.startswith('--kerf='):
            KERF = SF.set_kerf(x)
        elif x.startswith('--page-to='):
            page_to = x.split('=', 1)[1]
        else:
            raise SystemExit(f'error: {x} is not a flag this generator reads. It takes '
                             f'--out=, --edge=, --notes=, --end=, --kerf=, --page-to= and --no-write.')
    if page_to:
        page(page_to, notes)
        print(f'  wrote {os.path.basename(page_to)}')
        return 0
    if write and not out:
        raise SystemExit('error: --out=DIR, or --no-write to run the checks only.')
    pipes = design(notes)
    parts, strips, info = build(pipes, edge)
    sheets = pack(parts)
    s_sheets = pack(strips) if strips else []
    res = checks(pipes, parts, strips, info, edge, sheets + s_sheets)
    print(f'panpipes, {notes[0]} to {notes[-1]}, {edge} edge: {len(pipes)} closed pipes, '
          f'{DEPTH:g}mm deep, the raft {info["Y"]:.1f}mm across')
    for p in pipes:
        print(f'  {p["note"]:4} {p["f"]:7.2f}Hz  {p["w"]:5.2f} x {DEPTH:g}mm  plug face '
              f'{p["air"]:6.1f}mm from the rim, pipe {p["length"]:6.1f}mm')
    bad = 0
    for ok, what, detail in res:
        print(f'  {"pass" if ok else "FAIL"}  {what:56} {detail}')
        bad += not ok
    if bad:
        print(f'\n  {bad} check(s) failed. Nothing written.')
        return 1
    if write:
        os.makedirs(out, exist_ok=True)
        stem = f'panpipes-{notes[0].replace("#", "s")}-{edge}'
        runs = [(sheets, 'sheet', T)] + ([(s_sheets, f'{STRIP_T:g}mm-sheet', STRIP_T)] if strips else [])
        for group, label, stock in runs:
            for k, s in enumerate(group, 1):
                what = ', '.join(sorted({p.name.split(',')[0] for p, _, _ in s['parts']}))
                f = os.path.join(out, f'{stem}-{label}{k}-cut-files.svg')
                open(f, 'w').write(svg_sheet(k, len(group), s, what, edge, stock))
                print(f'  wrote {os.path.basename(f)}  {s["w"]:.0f} x {s["h"]:.0f}mm, '
                      f'{len(s["parts"])} parts')
    return 0


if __name__ == '__main__':
    sys.exit(main())
