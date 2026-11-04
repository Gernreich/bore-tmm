#!/usr/bin/env python3
"""Each designing model's own design, as bore-tmm JSON, with what that model
says every note plays (model_hz), so `bore-tmm check --compare` can show the
Rust engine and the Python models are the same physics.

    python3 tools/export_models.py            # writes bores/models/*.json

It imports the models from the sibling repositories, read-only:
square-flute/naf_model.py, octagonal-flute/flute_model.py,
practice-chanters/chanter_model.py and ghb-chanter/ghb_chanter.py.

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


def practice_chanter():
    use('practice-chanters')
    import chanter_model as M
    import practice_chanters as P
    for n in P.BUILT:
        b, L, pos, widths = M.design(n)
        a = 1000 * b.a
        write(f'practice-chanter-{n}', {
            'name': f'practice chanter, {n} sides, chanter_model.py design',
            'source': f'practice-chanters/chanter_model.py design({n})',
            'temp_c': M.T_C,
            'drive': {'kind': 'reed'},
            # the reed and staple as closed round bore, then the polygon
            'bore': [{'len_mm': M.REED, 'd_mm': 2 * a, 'loss_ratio': 1.0},
                     {'len_mm': L, 'd_mm': 2 * a, 'loss_ratio': 1000 * b.rh / a}],
            'end': 'flanged',
            'holes': [{'name': h, 'at_mm': M.REED + pos[h], 'd_mm': 2000 * b.hole_r(widths[h]),
                       'chimney_mm': b.wall} for h in M.HOLES],
            'notes': [{'name': nt, 'hz': M.target(nt), 'open': list(M.FINGERING[nt]),
                       'span': 1.3, 'model_hz': M.sounding(b, L, pos, widths, nt)}
                      for nt in M.NOTES],
        })


def ghb_chanter():
    use('ghb-chanter')
    import ghb_chanter as G
    M = G.M
    ref, got, holes, vent = G.solve()
    d0, slope = G.REF_CONE
    write('ghb-chanter', {
        'name': 'GHB chanter, ghb_chanter.py solve(): holes sized to sound the reference\'s notes',
        'source': 'ghb-chanter/ghb_chanter.py solve(); hz = the reference chanter in the same model',
        'temp_c': M.T_C,
        'drive': {'kind': 'reed', 'apex': True},
        # ghb_chanter.py takes the loss radius as the cone's diameter / 4, half
        # its radius, where the other models take the radius itself
        'bore': [{'len_mm': G.REF_SOLE, 'd1_mm': d0, 'd2_mm': d0 + slope * G.REF_SOLE,
                  'loss_ratio': 0.5}],
        'end': 'unflanged',
        'holes': [{'name': h, 'at_mm': z, 'd_mm': d, 'chimney_mm': t}
                  for h, z, d, t in holes.values()]
                 + [{'name': f'vent{i}', 'at_mm': G.REF_VENT[0], 'd_mm': vent,
                     'chimney_mm': M.PLATE, 'always_open': True} for i in (1, 2)],
        'notes': [{'name': nt, 'hz': ref[nt], 'open': list(M.FINGERING[nt]),
                   'span': 1.25, 'model_hz': got[nt]} for nt in M.NOTES],
    })


if __name__ == '__main__':
    octagonal_flute()
    square_flute()
    practice_chanter()
    ghb_chanter()
