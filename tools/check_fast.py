#!/usr/bin/env python3
"""Does src/models.rs still give the same bits as the five Python models?

    python3 tools/check_fast.py          # after cargo build --release

Each model's sounding() runs its Rust port when the library is built
(python/tmm_fast.py), and checks it against its own Python once per process.
This makes the comparison many times over instead: every model, its own
design's notes and then the same with every hole moved and resized at
random, at other spans, and the cases that find no resonance and must raise
the same error. A mismatch anywhere fails, naming the model and the call.

It fails, too, when the library is not there to test: a gate that quietly
compared Python with itself would pass for ever. The last line is

    FAST MATCH: n soundings in 5 models, bit for bit

or FAST MISMATCH with a count, and the exit status is 1 on any failure.
"""
import os
import random
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
GIT = os.path.normpath(os.path.join(HERE, '..', '..'))
for repo in ('practice-chanters', 'ghb-chanter', 'square-flute', 'octagonal-flute'):
    sys.path.insert(0, os.path.join(GIT, repo))

import chanter_model as CM                                       # noqa: E402
import flute_model as FM                                         # noqa: E402
import ghb_chanter as G                                          # noqa: E402
import naf_model as NM                                           # noqa: E402
import standard_chanter as S                                     # noqa: E402
import tmm_fast                                                  # noqa: E402


def outcome(fn):
    try:
        return struct.pack('>d', fn())
    except ValueError as e:
        return str(e)


counts, bad = {}, []


def same(model, what, pure, rust):
    counts[model] = counts.get(model, 0) + 1
    a, b = outcome(pure), outcome(rust)
    if a != b:
        bad.append(f'{model} {what}: python {a!r}, rust {b!r}')


def jiggle(rnd, xs, d):
    return [x + rnd.uniform(-d, d) for x in xs]


def chanters(rnd):
    for n in (3, 4, 6, 8):
        b, L, pos, widths = CM.design(n)
        for trial in range(3):
            p = dict(pos) if not trial else {h: x + rnd.uniform(-4, 4) for h, x in pos.items()}
            w = dict(widths) if not trial else {h: x * rnd.uniform(0.7, 1.3) for h, x in widths.items()}
            for nt in CM.NOTES:
                for span in (1.3, 2.2):
                    near = CM.target(nt) * (rnd.uniform(0.9, 1.1) if trial else 1)
                    same('chanter_model', f'{n} sides {nt} span {span}',
                         lambda: CM._sounding(b, L, p, w, nt, near, span),
                         lambda: CM._sounding_rust(b, L, p, w, nt, near, span))
    b = S.Bore(S.REF_BORE)
    ref = [(h, S.REF_TOP - z, d, wall) for h, (z, d, wall) in S.REF_HOLES.items()]
    for trial in range(4):
        hs = ref if not trial else [(h, z + rnd.uniform(-3, 3), d * rnd.uniform(0.8, 1.2), wall)
                                    for h, z, d, wall in ref]
        for nt in S.M.NOTES:
            near = 255.0 * rnd.uniform(0.85, 2.1)
            same('standard_chanter', f'trial {trial} {nt}',
                 lambda: S._sounding(b, S.REF_TOP, hs, nt, near),
                 lambda: S._sounding_rust(b, S.REF_TOP, hs, nt, near))
    ref = G.ref_holes()
    for trial in range(4):
        hs = ref if not trial else [(h, z + rnd.uniform(-2, 2), d * rnd.uniform(0.85, 1.15), wall)
                                    for h, z, d, wall in ref]
        for nt in G.M.NOTES:
            near = 480.0 * rnd.uniform(0.85, 2.0)
            same('ghb_chanter', f'trial {trial} {nt}',
                 lambda: G._sounding(hs, nt, near), lambda: G._sounding_rust(hs, nt, near))


def flutes(rnd):
    for name, wide, deep, wall, _ in NM.VARIANTS:
        b = NM.Bore(wide, deep)
        with NM.window(wide or NM.WINDOW_W):
            L, holes, diams, _ = NM.design(b, wall)
            for trial in range(3):
                hs = list(holes) if not trial else jiggle(rnd, holes, 3)
                ds = list(diams) if not trial else [d * rnd.uniform(0.8, 1.2) for d in diams]
                for n_open, near in enumerate(NM.TARGETS):
                    near *= rnd.uniform(0.9, 1.1) if trial else 1
                    same('naf_model', f'{name} trial {trial} {n_open} open',
                         lambda: NM._sounding(b, L, hs, n_open, wall, near, ds),
                         lambda: NM._sounding_rust(b, L, hs, n_open, wall, near, ds))
    for flats in (17.0, 19.0, 21.0):
        b = FM.Bore(flats)
        L, holes, diams = FM.design(b)
        for trial in range(3):
            hs = list(holes) if not trial else jiggle(rnd, holes, 3)
            ds = list(diams) if not trial else [d * rnd.uniform(0.8, 1.2) for d in diams]
            for n_open, near in enumerate(FM.TARGETS):
                near *= rnd.uniform(0.9, 1.1) if trial else 1
                for span in (1.4, 1.1):
                    same('flute_model', f'{flats} flats trial {trial} {n_open} open span {span}',
                         lambda: FM._sounding(b, L, hs, n_open, near, ds, span),
                         lambda: FM._sounding_rust(b, L, hs, n_open, near, ds, span))


def main():
    if tmm_fast.lib is None:
        print('FAST MISMATCH: no library to test -- cargo build --release in bore-tmm '
              '(or BORE_TMM_PURE is set)')
        return 1
    rnd = random.Random(20261107)
    chanters(rnd)
    flutes(rnd)
    for line in bad[:10]:
        print('  ' + line)
    n = sum(counts.values())
    if len(counts) != 5 or min(counts.values()) < 20:
        print(f'FAST MISMATCH: compared too little: {counts}')
        return 1
    if bad:
        print(f'FAST MISMATCH: {len(bad)} of {n} soundings differ')
        return 1
    print(f'FAST MATCH: {n} soundings in {len(counts)} models, bit for bit '
          f'({", ".join(f"{k} {v}" for k, v in sorted(counts.items()))})')
    return 0


if __name__ == '__main__':
    sys.exit(main())
