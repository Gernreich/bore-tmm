#!/usr/bin/env python3
"""Each designing model's own design, as bore-tmm JSON, with what that model
says every note plays (model_hz), so `bore-tmm check --compare` can show the
Rust engine and the Python models are the same physics.

    python3 tools/export_models.py            # writes bores/models/*.json
    python3 tools/export_models.py --out=DIR    # writes them there instead

Every model is Rust since 2026-11-07, in the sibling repositories, and
prints its own designs, which this runs, built: `naf_model --bores` in
square-flute, `flute_model --bores` in octagonal-flute, practice-chanters'
chanter_bores and `ghb_chanter --bores`.

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
    for name, d in rust('octagonal-flute', 'flute_model', '--bores')['models'].items():
        write(name, d)


def square_flute():
    for name, d in rust('square-flute', 'naf_model', '--bores')['models'].items():
        write(name, d)


def rust(repo, exe, *args):
    """A model's or a generator's JSON on stdout, the program built."""
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
