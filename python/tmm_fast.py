"""bore-tmm's Rust port of the design models' sounding(), for those models.

The two Python flute models, square-flute/naf_model.py and
octagonal-flute/flute_model.py, each spend nearly all their time in sounding(): a scan and a bisection over the bore's
admittance, a few thousand times a design. src/models.rs is that loop in
Rust, the same arithmetic in the same order, so it returns the same float,
about a hundred times sooner. (The three chanter models were Python too until 2026-11-07; they are Rust
now and call src/models.rs directly.) Each model finds this file in a bore-tmm
checkout beside its own repository:

    sys.path.append(<repository>/../bore-tmm/python)
    import tmm_fast

and calls run() with both of its paths. Without the library -- bore-tmm not
checked out, or `cargo build --release` not run in it, or BORE_TMM_PURE set
-- `lib` is None and every model runs its own Python, as it always did.

THE FIRST CALL IN A PROCESS RUNS BOTH. run() compares the Rust answer with
the model's own Python, bit for bit, once per model per process, and if they
differ it says so on stderr and uses the Python from then on. So a model
edited without its port, or a port built from older source, costs speed and
never a wrong cut file. tools/check_fast.py makes the same comparison over
many more calls, as a gate.
"""
import ctypes
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(HERE, '..', 'target', 'release', 'libbore_tmm.dylib')

D, I64, SZ, U8P, DP = (ctypes.c_double, ctypes.c_int64, ctypes.c_size_t,
                       ctypes.POINTER(ctypes.c_uint8), ctypes.POINTER(ctypes.c_double))

lib = None
if not os.environ.get('BORE_TMM_PURE') and os.path.exists(LIB):
    try:
        lib = ctypes.CDLL(LIB)
    except OSError:
        lib = None
if lib is not None:
    lib.tmm_jet.argtypes = [D] * 5 + [SZ, U8P, DP, DP, U8P] + [D] * 7 + [I64, I64]
    lib.tmm_jet.restype = D


def doubles(xs):
    return (ctypes.c_double * max(1, len(xs)))(*[float(x) for x in xs])


def flags(xs):
    return (ctypes.c_uint8 * max(1, len(xs)))(*[1 if x else 0 for x in xs])


def jet(c, rho, rh, area, a, kind, length, r, opened, t, stub, neck_area, neck_a, leff,
        near, span, nscan, bis):
    """The jet kernel: naf_model and flute_model. kind: 0 tube, 1 hole."""
    return lib.tmm_jet(c, rho, rh, area, a, len(kind), flags(kind), doubles(length),
                       doubles(r), flags(opened), t, stub, neck_area, neck_a, leff, near,
                       span, nscan, bis)


def bits(x):
    return struct.pack('>d', x)


_checked = {}


def _outcome(fn):
    """What fn did, for comparing: its float to the bit, or the error it raised."""
    try:
        x = fn()
        return (bits(x), repr(x))
    except ValueError as e:
        return ('raises', f'ValueError({str(e)!r})')


def run(model, fast, pure):
    """fast(), once this model's port has matched pure() in this process;
    pure() if there is no library or the two ever differed."""
    if lib is None or _checked.get(model) is False:
        return pure()
    if model not in _checked:
        got, want = _outcome(fast), _outcome(pure)
        _checked[model] = got == want
        if got != want:
            sys.stderr.write(f'tmm_fast: the Rust port of {model}.sounding() gave {got[1]}, '
                             f'its Python {want[1]}; using the Python. Rebuild bore-tmm '
                             f'(cargo build --release) or port the change.\n')
            return pure()
    return fast()
