#!/usr/bin/env python3
"""Each instrument's bore as its CUT FILES draw it, as bore-tmm JSON.

    python3 tools/read_cut_files.py          # writes bores/cut/ and bores/predict/
    python3 tools/read_cut_files.py --out=DIR    # DIR/cut/ and DIR/predict/ instead

bores/cut/ holds the instruments whose drawings must play their model's scale,
and the tuning gate judges them. bores/predict/ holds the ones whose pitch is
the question rather than the check: the trumpets, and the uilleann chanter,
whose holes are cut small to be opened by ear.

tools/export_models.py writes what each designing model asked for. This reads
what the generator drew: hole positions and cut sizes measured off the SVGs in
each repository's cut-files/, the kerf taken from the file's own description.
`bore-tmm check --tol=...` over bores/cut/ then says whether the parts as cut
play the scale, which is what can drift when a generator changes and its
model does not.

What a cut file cannot say is taken from the generator and named in each
reader: the bore's section (the parts are flat, and no one part carries it),
the ply, how far the foot is cut long to be trimmed, and the scale. Every
targets list is the model's, so a reader never decides what is in tune.

Each reader fails loudly when the drawing is not the shape it expects: the
wrong number of holes, a window it cannot find. It never guesses.
"""
import json
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import svgparts as S                                           # noqa: E402

GIT = os.path.normpath(os.path.join(HERE, '..', '..'))
ROOT = os.path.normpath(os.path.join(HERE, '..', 'bores'))
BLACK, ORANGE, BLUE = '#000000', '#ff8000', '#0000ff'


def use(repo):
    p = os.path.join(GIT, repo)
    if p not in sys.path:
        sys.path.insert(0, p)


def rel(path):
    return os.path.relpath(path, GIT)


def write(name, d, kind='cut'):
    out = os.path.join(ROOT, kind)
    os.makedirs(out, exist_ok=True)
    path = os.path.join(out, name + '.json')
    with open(path, 'w') as f:
        json.dump(d, f, indent=1)
        f.write('\n')
    print(f'{rel(path)}: {len(d.get("holes", []))} holes, {len(d["notes"])} notes')


class Unreadable(Exception):
    pass


def parts_of(shapes):
    """The black outlines that are parts, largest first."""
    return sorted((s for s in shapes if s.colour == BLACK and s.closed),
                  key=lambda s: -s.area)


def inside_part(part, shapes, colour):
    return [s for s in shapes if s.colour == colour and s is not part
            and part.inside(*s.centre)]


def along(part):
    """The part's long axis: 0 for x, 1 for y."""
    x0, y0, x1, y1 = part.bbox
    return 0 if x1 - x0 >= y1 - y0 else 1


# --- the octagonal flute -----------------------------------------------------------

def octagonal_flute():
    use('octagonal-flute')
    import flute_model as M
    f = os.path.join(GIT, 'octagonal-flute', 'cut-files', 'octagonal-flute-D4-sheet1-cut-files.svg')
    shapes, desc = S.shapes(f)
    kerf = S.kerf_of(desc)
    # the top wall carries the two engraved lines: the cork's face and the foot
    blues = [s for s in shapes if s.colour == BLUE]
    top = [p for p in parts_of(shapes) if all(p.inside(*b.centre) for b in blues)]
    if len(blues) != 2 or len(top) != 1:
        raise Unreadable(f'{rel(f)}: no single top wall carrying the two engraved lines')
    top = top[0]
    ax = along(top)
    cuts = inside_part(top, shapes, ORANGE)
    # the embouchure is the cut nearer the cork's line; the rest are finger holes
    cork = min(b.centre[ax] for b in blues)
    foot = max(b.centre[ax] for b in blues)
    cuts.sort(key=lambda s: s.centre[ax])
    emb, holes = cuts[0], cuts[1:]
    if len(holes) != len(M.HOLES):
        raise Unreadable(f'{rel(f)}: {len(holes)} finger holes on the top wall, not {len(M.HOLES)}')
    ec = emb.centre[ax]
    # foot-most first, as the model numbers them
    holes.sort(key=lambda s: -s.centre[ax])
    b = M.Bore()
    write('octagonal-flute', {
        'name': 'octagonal flute in D, as cut',
        'source': (f'{rel(f)}: embouchure, holes, cork and foot lines measured, kerf {kerf}; '
                   f'section, wall, lip cover and mouth from flute_model.py'),
        'temp_c': M.T_C,
        'drive': {'kind': 'jet', 'area_mm2': M.COVER * emb.cut_area(kerf),
                  'chimney_mm': M.CHIMNEY, 'mouth': M.MOUTH, 'stub_mm': ec - cork},
        'bore': [{'len_mm': foot - ec, 'd_mm': 2000 * b.a, 'loss_ratio': b.rh / b.a}],
        'end': 'unflanged',
        'holes': [{'name': M.HOLES[i], 'at_mm': h.centre[ax] - ec,
                   'd_mm': S.eq_d(h.cut_area(kerf)), 'chimney_mm': M.T}
                  for i, h in enumerate(holes)],
        'notes': [{'name': M.NOTES[n], 'hz': M.TARGETS[n], 'open': M.HOLES[:n], 'span': 1.4}
                  for n in range(7)],
    })


# --- the square-bore flutes -----------------------------------------------------

def square_flutes():
    use('square-flute')
    import naf_model as M
    import square_flute as SF
    d = os.path.join(GIT, 'square-flute', 'cut-files')
    for name in sorted(os.listdir(d)):
        files = [x for x in os.listdir(os.path.join(d, name)) if x.endswith('.svg')]
        if len(files) != 1:
            raise Unreadable(f'square-flute/cut-files/{name}: {len(files)} sheets, expected 1')
        f = os.path.join(d, name, files[0])
        shapes, desc = S.shapes(f)
        kerf = S.kerf_of(desc)
        variant = name[:-5] if name.endswith('-flat') else name
        w, dd, wall, how = [(vw, vd, vwall, vhow) for vn, vw, vd, vwall, vhow in M.VARIANTS
                            if vn == variant][0]
        # the top plate: the part with the five finger holes and the window
        circles = [s for s in shapes if isinstance(s, S.Circle) and s.colour == ORANGE]
        tops = [p for p in parts_of(shapes)
                if sum(p.inside(*c.centre) for c in circles) == 5]
        # a doubled wall carries the holes twice, in the top and the doubler:
        # the top is the one that also has the window
        tops = [p for p in tops if any(isinstance(s, S.Shape) and not isinstance(s, S.Circle)
                                       for s in inside_part(p, shapes, ORANGE))]
        if len(tops) != 1:
            raise Unreadable(f'{rel(f)}: {len(tops)} parts look like the top plate')
        top = tops[0]
        ax = along(top)
        inner = inside_part(top, shapes, ORANGE)
        holes = [s for s in inner if isinstance(s, S.Circle)]
        slots = [s for s in inner if not isinstance(s, S.Circle)]
        if len(slots) != 2:
            raise Unreadable(f'{rel(f)}: {len(slots)} slots in the top plate, not the exit hole '
                             f'and the window')
        hx = sum(h.centre[ax] for h in holes) / len(holes)
        # the window is the slot nearer the finger holes; the foot is the far end
        win = min(slots, key=lambda s: abs(s.centre[ax] - hx))
        wc = win.centre[ax]
        lo, hi = top.bbox[ax], top.bbox[ax + 2]
        sign = 1 if hx > wc else -1
        end = hi if sign > 0 else lo
        drawn = sign * (end - wc)
        x0, y0, x1, y1 = win.bbox
        span = (x1 - x0, y1 - y0)
        wl, ww = span[ax] + kerf, span[1 - ax] + kerf
        holes.sort(key=lambda h: -sign * h.centre[ax])
        names = [f'h{i + 1}' for i in range(5)]
        b = M.Bore(w, dd)
        write('square-flute-' + name, {
            'name': f'square-bore flute {name}, as cut, foot trimmed {M.FOOT_SPARE:g}mm',
            'source': (f'{rel(f)}: window, holes and plate length measured, kerf {kerf}; '
                       f'section, wall and the foot\'s {M.FOOT_SPARE:g}mm to trim from '
                       f'naf_model.py'),
            'temp_c': M.T_C,
            'drive': {'kind': 'jet', 'area_mm2': ww * wl, 'chimney_mm': M.WINDOW_T,
                      'mouth': M.MOUTH, 'stub_mm': wl / 2},
            'bore': [{'len_mm': drawn - M.FOOT_SPARE, 'd_mm': 2000 * b.a,
                      'loss_ratio': b.rh / b.a}],
            'end': 'unflanged',
            'holes': [{'name': names[i], 'at_mm': sign * (h.centre[ax] - wc),
                       'd_mm': 2 * h.r + kerf, 'chimney_mm': wall}
                      for i, h in enumerate(holes)],
            'notes': [{'name': M.NOTES[n], 'hz': M.TARGETS[n], 'open': names[:n], 'span': 1.4}
                      for n in range(6)],
        })


# --- the chanters -------------------------------------------------------------------
#
# A chanter's cut file cannot give its bore: the plate chanter's 131 plates are
# packed on the sheet by size, not in stack order, and a strip's holes do not say
# which end is the reed's. So the chanters are read from the PARTS the generator
# draws its sheet from -- each part's place down the body, each hole's cut -- which
# the repository's .repro gate holds equal to the committed SVG. The generators are
# Rust since 2026-11-07 and read their own parts: practice-chanters'
# chanter_bores and `ghb_chanter --bores` print each chanter's bore as this
# writes it (bores.rs and bores() there say how); this runs them, built.

def rust(repo, exe, *args):
    """A chanter generator's JSON on stdout, the generator built."""
    import subprocess
    path = os.path.join(GIT, repo, 'target', 'release', exe)
    if not os.path.exists(path):
        raise Unreadable(f'{rel(path)} is not built: cargo build --release in {repo}')
    r = subprocess.run([path, *args], capture_output=True, text=True)
    if r.returncode:
        raise Unreadable(f'{rel(path)} {" ".join(args)}: {(r.stdout + r.stderr).strip()}')
    return json.loads(r.stdout)


def practice_chanters():
    for name, d in rust('practice-chanters', 'chanter_bores')['cut'].items():
        write(name, d)


def ghb_chanters():
    for name, d in rust('ghb-chanter', 'ghb_chanter', '--bores')['cut'].items():
        write(name, d)


# --- the trumpets -------------------------------------------------------------------
#
# A trumpet here is three parts: the mouthpiece and the bell, shared by every
# 10mm bore, each a stack of rings whose apertures the cut file gives in
# assembly order from ring 0 at the bore; and the bore between them, a 10mm
# square channel whose centreline length its generator reports. The model
# takes every ring as a short cylinder of its aperture's area, exactly the
# staircase that is cut, and the lips as a reed at the rim: the instrument
# sounds near the peaks of the impedance they see.
#
# NOT modelled: the bore's turns. A 90-degree lattice corner grows the
# section by up to 41% for a block, and the spiral's facets meet at 45 degrees;
# both are taken as straight tube of the centreline's length. Nor the bell's
# square port on the spiral. These bores are in bores/predict/, not bores/cut/,
# so the tuning gate does not judge them: what they play is the question,
# and one measured note is not yet a scale.

TRUMPET = os.path.join(GIT, 'trumpet-elbows-allowed')
SQUARE10 = {'d_mm': 2 * math.sqrt(100 / math.pi), 'loss_ratio': math.sqrt(math.pi / 4)}


def apertures(svg):
    """Each ring's aperture, round-equivalent diameter, in the file's order."""
    shapes, desc = S.shapes(svg)
    m = re.search(r'([\d.]+)mm of rise', desc)
    rise = float(m.group(1)) if m else 3.0
    return [S.eq_d(s.area) for s in shapes if s.colour == ORANGE], rise


def mouthpiece(layout):
    """The mouthpiece's rings, lip first. The shipped sheet is the trumpet
    layout; the legacy layout, the one on the three-turn trumpet, is drawn by
    the same generator into a temporary file."""
    if layout == 'trumpet':
        f = os.path.join(TRUMPET, 'parts', 'mouthpiece', 'cut-files',
                         'mouthpiece-bore10-trumpet-parts-cut-files.svg')
        aps, rise = apertures(f)
        src = rel(f)
    else:
        import subprocess
        import tempfile
        with tempfile.TemporaryDirectory() as t:
            f = os.path.join(t, 'legacy.svg')
            subprocess.run([sys.executable, os.path.join(TRUMPET, 'parts', 'mouthpiece',
                                                         'mouthpiece-round.py'),
                            '--layout=legacy', f], check=True, capture_output=True)
            aps, rise = apertures(f)
        src = 'mouthpiece-round.py --layout=legacy'
    return [{'len_mm': rise, 'd_mm': d} for d in reversed(aps)], src


def bell():
    f = os.path.join(TRUMPET, 'parts', 'bell', 'cut-files',
                     'bell-round10-153mm-17rings-x3-rim86-cut-files.svg')
    aps, rise = apertures(f)
    return [{'len_mm': rise, 'd_mm': d} for d in aps], rel(f)


def centreline(walk):
    import subprocess
    out = subprocess.run([sys.executable, os.path.join(TRUMPET, 'tools', 'bore_split.py'), walk,
                          '--bore=10', '--straight=30', '--no-write'],
                         check=True, capture_output=True, text=True).stdout
    m = re.search(r'\(([\d.]+)mm of centreline\)', out)
    if not m:
        raise Unreadable('bore_split.py did not report a centreline length')
    return float(m.group(1))


def trumpets():
    b, bsrc = bell()
    walk = open(os.path.join(TRUMPET, 'tools', 'walks', 'coil-3t.txt')).readline().strip()
    L = centreline(walk)
    for layout, tag, why in (('legacy', 'built', 'as built: the 24-ring legacy mouthpiece'),
                             ('trumpet', 'designed', 'as designed: the shipped 30-ring mouthpiece')):
        mp, msrc = mouthpiece(layout)
        write(f'three-turn-{tag}', {
            'name': f'three-turn trumpet, {why}',
            'source': (f'{msrc}; {L:g}mm of 10mm square bore, bore_split.py on '
                       f'tools/walks/coil-3t.txt with --straight=30; {bsrc}'),
            'drive': {'kind': 'reed'},
            'bore': mp + [dict(len_mm=L, **SQUARE10)] + b,
            'end': 'flanged',
            'targets': 'the one note measured off the built instrument (three-turn/README.md)',
            'notes': [{'name': 'F4 measured', 'hz': 349.2, 'span': 1.15}] if tag == 'built' else [],
        }, kind='predict')
    mp, msrc = mouthpiece('trumpet')
    write('swept-curve-spiral', {
        'name': 'swept-curve spiral, 1000mm, with the shipped mouthpiece and the bell',
        'source': (f'{msrc} (which mouthpiece it carries is not recorded: assumed the shipped '
                   f'one); 1000mm of 10mm square ribbon bore, swept-curve/README.md; {bsrc}'),
        'drive': {'kind': 'reed'},
        'bore': mp + [dict(len_mm=1000.0, **SQUARE10)] + b,
        'end': 'flanged',
        'notes': [],
    }, kind='predict')


# --- the uilleann chanter --------------------------------------------------------------
#
# Fischer's square cone under a round throat, his eight holes where the lid and
# back strips cut them. Nothing in uilleann_chanter.rs is modelled, and his
# holes are DRILLED SMALL, to be opened toward his final sizes by ear, so the
# chanter as cut should play flat; the second file has his final sizes. Not
# modelled either: his four dishes, the bore enlargements under the lid, each
# of which raises the notes above it.
#
# The reed and its staple are the uncalibrated part, as on every chanter here:
# a closed length of the throat's bore above the top, fitted (bore-tmm fit) so
# that the chanter as cut sounds low D with every hole closed. A piper sets
# that by how far the reed goes in.
#
# Fingering: the "closed" uilleann fingering, each note opening its own hole
# and keeping every other one shut; D' is the thumb's hole on the back.

UILLEANN_NOTES = [('D4', 0, None), ('Eb4', 1, 'Eb'), ('E4', 2, 'E'), ('F#4', 4, 'F#'),
                  ('G4', 5, 'G'), ('A4', 7, 'A'), ('B4', 9, 'B'), ('C#5', 11, 'C#'),
                  ("D5", 12, "D'")]
BORE_TMM = os.path.normpath(os.path.join(HERE, '..', 'target', 'release', 'bore-tmm'))


def uilleann():
    import subprocess
    import tempfile
    u = rust('uilleann-chanter', 'uilleann_chanter', '--bores')
    x0, sole, found = u['x0'], u['sole'], u['found']
    ref = u['ref']
    if len(found) != len(ref):
        raise Unreadable(f'uilleann_chanter.rs: {len(found)} holes cut, not {len(ref)}')
    side = [s * 2 / math.sqrt(math.pi) for s in u['side']]     # the square's round equivalent
    sq = math.sqrt(math.pi / 4)
    d4 = 440 * 2 ** (-7 / 12)
    for tag, sizes in (('as-cut', [d for _, d, _ in found]),
                       ('final', [h[3] * u['inch'] for h in ref])):
        ins = {
            'name': f'uilleann chanter, Fischer\'s square bore, holes {tag.replace("-", " ")}',
            'source': ('uilleann-chanter/uilleann_chanter.rs build(): hole places, '
                       + ('their drilled sizes' if tag == 'as-cut' else 'Fischer\'s final sizes')
                       + ', strip depths; the throat and the square cone from its constants; '
                       'the reed allowance fitted to low D'),
            'drive': {'kind': 'reed'},
            'bore': [{'len_mm': 30.0, 'd_mm': u['throat']},
                     {'len_mm': x0, 'd_mm': u['throat']},
                     {'len_mm': sole - x0, 'd1_mm': side[0], 'd2_mm': side[1], 'loss_ratio': sq}],
            'end': 'unflanged',
            'holes': [{'name': h[0], 'at_mm': 30.0 + z, 'd_mm': d, 'chimney_mm': c}
                      for h, (z, _, c), d in zip(ref, found, sizes)],
            'notes': [{'name': nm, 'hz': d4 * 2 ** (k / 12), 'open': [o] if o else [], 'span': 1.3}
                      for nm, k, o in UILLEANN_NOTES],
        }
        if tag == 'as-cut':
            with tempfile.TemporaryDirectory() as t:
                f = os.path.join(t, 'u.json')
                json.dump(ins, open(f, 'w'))
                r = subprocess.run([BORE_TMM, 'fit', f, '--set=bore.0.len_mm', '--range=0,250',
                                    '--heard=D4=%r' % d4, f'--write={f}'],
                                   capture_output=True, text=True)
                if r.returncode:
                    raise Unreadable(f'bore-tmm fit for the uilleann reed: {r.stdout}{r.stderr}')
                reed = round(json.load(open(f))['bore'][0]['len_mm'], 2)
        ins['bore'][0]['len_mm'] = reed
        ins['source'] += f' ({reed}mm, on the holes as cut)'
        write(f'uilleann-chanter-{tag}', ins, kind='predict')


READERS = [octagonal_flute, square_flutes, practice_chanters, ghb_chanters, trumpets, uilleann]

if __name__ == '__main__':
    for x in sys.argv[1:]:
        if x.startswith('--out='):
            ROOT = os.path.abspath(x.split('=', 1)[1])
        else:
            raise SystemExit(f'error: {x} is not a flag this reads. It takes --out=DIR.')
    bad = 0
    for r in READERS:
        try:
            r()
        except Unreadable as e:
            bad += 1
            print(f'UNREADABLE {e}')
    sys.exit(1 if bad else 0)
