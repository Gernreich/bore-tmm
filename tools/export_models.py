#!/usr/bin/env python3
"""Each designing model's own design, as bore-tmm JSON, with what that model
says every note plays (model_hz), so `bore-tmm check --compare` can show the
Rust engine and the Python models are the same physics.

    python3 tools/export_models.py            # writes bores/models/*.json
    python3 tools/export_models.py --out=DIR    # writes them there instead

It imports the flute models from the sibling repositories, read-only:
square-flute/naf_model.py and octagonal-flute/flute_model.py. The chanters'
models are Rust since 2026-11-07: practice-chanters' chanter_bores and
`ghb_chanter --bores` print their designs, which this runs, built.

These are the MODELS' bores, not the cut files'. A model's bore is what the
generator was asked for; a cut file's is what it drew. Reading the bore back
out of a generator's own geometry is the next step, and the one that would
catch the two drifting apart.
"""
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
GIT = os.path.normpath(os.path.join(HERE, '..', '..'))
OUT = os.path.normpath(os.path.join(HERE, '..', 'bores', 'models'))


def use(repo):
    sys.path.insert(0, os.path.join(GIT, repo))


def write(name, d):
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name + '.json')
    with open(path, 'w') as f:
        json.dump(d, f, indent=1)
        f.write('\n')
    print(f'{path}: {len(d["notes"])} notes')


def octagonal_flute():
    use('octagonal-flute')
    import flute_model as M
    b = M.Bore()
    L, holes, diams = M.design(b)
    write('octagonal-flute', {
        'name': 'octagonal flute in D, flute_model.py design',
        'source': 'octagonal-flute/flute_model.py design()',
        'temp_c': M.T_C,
        'drive': {'kind': 'jet', 'area_mm2': M.COVER * M.emb_area(),
                  'chimney_mm': M.CHIMNEY, 'mouth': M.MOUTH, 'stub_mm': M.CORK},
        'bore': [{'len_mm': L, 'd_mm': 2000 * b.a, 'loss_ratio': b.rh / b.a}],
        'end': 'unflanged',
        'holes': [{'name': M.HOLES[i], 'at_mm': holes[i], 'd_mm': diams[i],
                   'chimney_mm': M.T} for i in range(6)],
        'notes': [{'name': M.NOTES[n], 'hz': M.TARGETS[n], 'open': M.HOLES[:n],
                   'span': 1.4,
                   'model_hz': M.sounding(b, L, holes, n, M.TARGETS[n], diams)}
                  for n in range(7)],
    })


def square_flute():
    use('square-flute')
    import naf_model as M
    for name, w, d, wall, how in M.VARIANTS:
        if w is None:
            continue
        b = M.Bore(w, d)
        with M.window(w):
            L, holes, diams, _ = M.design(b, wall, even=True)
            model = [M.sounding(b, L, holes, n, wall, M.TARGETS[n], diams) for n in range(6)]
        names = [f'h{i + 1}' for i in range(5)]
        write('square-flute-' + name, {
            'name': f'square-bore flute {name}, naf_model.py design (laid out for the hands)',
            'source': 'square-flute/naf_model.py design(even=True)',
            'temp_c': M.T_C,
            'drive': {'kind': 'jet', 'area_mm2': w * M.WINDOW_L, 'chimney_mm': M.WINDOW_T,
                      'mouth': M.MOUTH, 'stub_mm': M.WINDOW_L / 2},
            'bore': [{'len_mm': L, 'd_mm': 2000 * b.a, 'loss_ratio': b.rh / b.a}],
            'end': 'unflanged',
            'holes': [{'name': names[i], 'at_mm': holes[i], 'd_mm': diams[i],
                       'chimney_mm': wall} for i in range(5)],
            'notes': [{'name': M.NOTES[n], 'hz': M.TARGETS[n], 'open': names[:n],
                       'span': 1.4, 'model_hz': model[n]} for n in range(6)],
        })


def rust(repo, exe, *args):
    """A chanter generator's JSON on stdout, the generator built."""
    import subprocess
    path = os.path.join(GIT, repo, 'target', 'release', exe)
    if not os.path.exists(path):
        raise SystemExit(f'error: {path} is not built: cargo build --release in {repo}')
    r = subprocess.run([path, *args], capture_output=True, text=True, check=True)
    return json.loads(r.stdout)


def practice_chanter():
    # NOT the chanter practice_chanters builds: chanter_model.rs's own design
    # (practice-chanters/bores.rs says why it is kept)
    for name, d in rust('practice-chanters', 'chanter_bores')['models'].items():
        write(name, d)


def ghb_chanter():
    for name, d in rust('ghb-chanter', 'ghb_chanter', '--bores')['models'].items():
        write(name, d)


if __name__ == '__main__':
    for x in sys.argv[1:]:
        if x.startswith('--out='):
            OUT = os.path.abspath(x.split('=', 1)[1])
        else:
            raise SystemExit(f'error: {x} is not a flag this reads. It takes --out=DIR.')
    octagonal_flute()
    square_flute()
    practice_chanter()
    ghb_chanter()
