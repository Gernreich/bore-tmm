# bore-tmm

[← All projects](https://gernreich.github.io)

What a described bore plays. Describe a wind instrument's bore from its driven end down: its lengths of cylinder and cone, the tone holes along it, how it ends and what drives it. bore-tmm then gives each fingering's note, how far that note is from where it should be, and every register the fingering has.

It is a transfer-matrix model in Rust. The instrument repositories each carry their own copy of the same physics, in Python, to design their instruments:

- [naf_model.py](https://github.com/Gernreich/square-flute/blob/main/naf_model.py), in square-flute
- [flute_model.py](https://github.com/Gernreich/octagonal-flute/blob/main/flute_model.py), in octagonal-flute
- [chanter_model.py](https://github.com/Gernreich/practice-chanters/blob/main/chanter_model.py) and [standard_chanter.py](https://github.com/Gernreich/practice-chanters/blob/main/standard_chanter.py), in practice-chanters
- [ghb_chanter.py](https://github.com/Gernreich/ghb-chanter/blob/main/ghb_chanter.py), in ghb-chanter

bore-tmm does not design anything. It answers the question a cut file has to answer: given this bore, what does it play? It reads each instrument's bore back out of what its generator draws, checks the tuning in a gate, and has the tools to measure a built bore and fit the model's uncalibrated parts to it.

```
cargo build --release
B=./target/release/bore-tmm
$B check --tol=5 bores/cut/*.json                 # every drawn instrument in tune?
$B check --compare bores/models/*.json            # the engine against the Python models
$B registers bores/predict/three-turn-built.json  # a bore's resonances
$B curve BORE.json NOTE --to=2000                 # |Z| at the drive, as CSV
$B fit BORE.json --set=drive.mouth --range=0.5,1.5 --heard="D4=291.2,A4=437"
$B sweep sweep.wav                                # for the measuring rig
$B compare BORE.json NOTE recording.wav --ref=cavity.wav --ref-bore=bores/rig/reference-cavity.json
```

## The physics

- **Bore**: a lossy tube with Benade's wall loss, 3.0e-5 √f / r per metre, taken on a loss radius. Each segment gives the loss radius as `loss_ratio` times its equivalent radius. That ratio is 1 for a round bore and √(π / (n tan(π/n))) for a regular n-gon.
- **Segments**: plane waves in a cylinder, spherical waves in a cone. A segment is described by equivalent diameters, which give the round bore of the same area.
- **Holes**: Keefe's (1990) tone hole, open or closed. Its chimney is the wall it goes through, and it is described by the round hole of its area, whatever its shape.
- **End**: `unflanged` (0.25 (ka)², 0.6133 ka), `flanged` (0.5 (ka)², 0.8216 ka) or `closed`.
- **Drive**:
  - `reed`: a cane reed, or lips. The bore sounds where Im(Y) rises through zero. With `apex`, the first cone carries on above the top to its apex, closed there, which stands in for a conical bore's missing tip.
  - `jet`: a flute. The window or embouchure is a neck flanged at both ends, in series with the bore, with the stub to the plug or cork in parallel. The bore sounds where the reactance rises through zero.

## The bore file

```json
{
 "name": "...", "temp_c": 25,
 "drive": {"kind": "jet", "area_mm2": 75.6, "chimney_mm": 6, "mouth": 1, "stub_mm": 20},
 "bore": [{"len_mm": 511.9, "d_mm": 19.5, "loss_ratio": 0.974}],
 "end": "unflanged",
 "holes": [{"name": "R3", "at_mm": 404.0, "d_mm": 5.52, "chimney_mm": 3}],
 "notes": [{"name": "E4", "hz": 329.63, "open": ["R3"]}]
}
```

Positions are in mm down the bore from its driven end. A cone segment takes `d1_mm` and `d2_mm` in place of `d_mm`. A hole with `always_open` is a vent. A note can carry `span` (how far either side of `hz` to look), `model_hz` (what the designing model says it plays), and `tol_cents` with `tol_why`, its own tolerance and the reason for it. `targets` says what the notes' `hz` are when they are not the scale itself.

## What the drawings play

`tools/read_cut_files.py` reads every instrument's bore from what its generator draws, and writes them to `bores/cut/`:

- **The flutes are read off their SVGs.** The octagonal flute's top wall carries the embouchure, the six holes, and two engraved lines for the cork's face and the foot. The square flutes' top plate carries the window and five holes, and the foot is the plate's end less the 15mm cut long to trim. Each hole's size is its cut shape plus the kerf the file says it was drawn for.
- **The chanters are read off the parts their sheets are drawn from**, because a sheet cannot give a chanter's bore. The plate chanter's 131 plates are packed by size, not in stack order, and a strip's holes do not say which end is the reed's. The repositories' `.repro` gates hold those parts equal to the committed SVGs.

What no part carries comes from the generator, and each reader says so in the file's `source`: the bore's section, the ply, the reed allowance, and the scale.

`check --tol=5` over all fourteen reads:

```
TUNING OK: 106 notes in 14 files, every one within 5 cents or its own stated tolerance
```

Every note is within 0.3 cents of its model except one. The 8-sided plate chanter's low G is 6.7 cents sharp, which `practice_chanters.py` accepts on purpose: the bore is the nearest whole number of plates, and low G has no hole to tune it. Its file says so, with a 10-cent tolerance.

## Agreement with the Python models

`tools/export_models.py` writes each model's own design to `bores/models/`, with that model's prediction for every note. `check --compare` then reads:

```
MODEL MATCH: 49 notes, worst 0.00000 cents from the designing model
```

**What the comparison found:**

- **The plate chanter is no longer `chanter_model.py`'s design.** Since 2026-10-23 `practice_chanters.py` has solved it in `standard_chanter.py`'s model, against a measured reference chanter. The export of `chanter_model.py`'s own 8-sided design is kept, renamed `chanter-model-8-sided`, because `strip_chanter.py` still uses that model. `bores/cut/` has the chanter that is actually cut.
- **The GHB chanter's loss radius is half the others'.** `ghb_chanter.py` takes the cone's diameter / 4, half its radius; the other models take the radius. The GHB files keep `loss_ratio: 0.5` so the engines match. Set to 1, every note moves 3 to 6 cents flat. Its model only compares the strip chanter with the reference in the same model, so the effect on the hole sizes should be smaller than that, but it has not been measured.

## Predictions

`bores/predict/` holds the bores whose pitch is the open question rather than the check, so the tuning gate does not judge them.

**The three-turn trumpet** is the 24-ring legacy mouthpiece, 1096mm of 10mm square bore (the length `bore_split.py` reports for its walk), and the 17-ring bell. Every ring is a short cylinder of its aperture's area, read off the cut files. Its one measured note is F4, 349.2 Hz, and with nothing calibrated the model has a resonance at **350.1 Hz, 4.6 cents away**:

```
three-turn trumpet, as built: no holes; the bore's resonances, 40 to 1500Hz
     71.0Hz  C#2 +43c
    212.3Hz  G#3 +38c
    350.1Hz  F4 +4c
    479.5Hz  A#4 +49c
    595.1Hz  D5 +23c
    705.4Hz  F5 +17c
```

The three-turn writeup says F4 "lands on no simple mode" of the tube and puts that down to the bell and the mouthpiece. The model agrees. It is one note, so this is evidence, not proof.

The model does not include the bore's 16 turns, where a lattice corner grows the section by up to 41% for one 16mm block. Taken as +21% (the mean) the note moves −0.2 cents; taken as the full +41%, −11.7 cents. The measured note falls between the two. With the shipped 30-ring mouthpiece the same bore would sound at 352.1 Hz.

**The swept-curve spiral** has been blown, but its note is not reported. It is 1000mm of the same bore with the same bell. Which mouthpiece it carries is not recorded, so the shipped one is assumed. The model's resonances are 77.4, 231.4, **380.2** (F#4 +47c), 510.7, 611.7 and 724.2 Hz. One note measured off it would test this prediction.

**The uilleann chanter** is read from `uilleann_chanter.py`'s parts: Fischer's square cone under a round throat, and his eight holes at their drilled sizes (`-as-cut`) and his final sizes (`-final`). The reed allowance is fitted so that low D sounds with every hole closed. **The model does not give it a playable scale.** With a cylinder reed the lowest notes come out sharp and the top ones flat, and D5 is 300 cents out. With the cone carried to its apex every hole plays about a minor third sharp. The suspects, in order:

1. the fingering, taken as each note opening only its own hole;
2. the reed stand-in, for a cone this narrow at the top;
3. Fischer's four bore dishes, which are not modelled.

So no notes are claimed for it until a measured chanter settles which.

## Measuring a bore

The parts the models cannot get from a drawing are the reed's allowance and the flute's window correction. Both are guessed, and each moves every note. Two measurements pin them down.

**Played notes, and `fit`.** Play the instrument, read a tuner, and fit the one uncalibrated number to what was heard:

```
$B fit bores/cut/octagonal-flute.json --set=drive.mouth --range=0.5,1.5 \
    --heard="D4=289.9,E4=325.1,G4=387.3" --write=mine.json
```

The fit is least squares in cents over the notes given, scanned and then narrowed. It prints each note's residue, the part one number cannot explain, and it fails if the answer sits at the end of `--range`. `--set` takes any number in the file: `bore.0.len_mm` for a reed allowance, `holes.3.d_mm` for one hole.

**The bore alone, with a rig.** A played note mixes the bore with the player. The rig measures the bore with no player:

- **Source**: a small loudspeaker sealed in a box, which feeds the bore's input through a **capillary**. Use a blunted 0.5mm-bore needle about 50mm long: about 6×10⁸ Pa·s/m³ of flow resistance, seven times the trumpet's highest impedance peak and far above any flute's or chanter's. Through it the flow barely depends on the bore, so the pressure at the input is the bore's impedance times a constant.
- **Microphone**: an electret capsule set flush in the wall at the input plane, into any audio interface.
- **Reference**: a closed cavity 19mm across and 20mm deep, `bores/rig/reference-cavity.json`, measured the same way. Its first resonance is near 8.6kHz, so across the band its impedance is smooth and known. Dividing by it removes the loudspeaker, the microphone and the room.

```
$B sweep sweep.wav --to=3000                 # play this, record the microphone
$B analyse cavity.wav --to=3000              # the reference, for a look
$B compare bores/cut/octagonal-flute.json E4 flute-E4.wav --to=3000 \
    --ref=cavity.wav --ref-bore=bores/rig/reference-cavity.json
```

`compare` lists each model resonance beside the measured one, in cents. `simulate` writes the recording the rig would make of a model. On that, with the loudspeaker deliberately coloured (a 150Hz high-pass and a 6dB bump at 900Hz) and noise added, the reference recovers every resonance of the octagonal flute's E4 fingering within 5 cents, and `cargo test` checks it does. **The rig itself has not been built.** Only the analysis is tested, and only on simulated recordings.

## The gate

`lasermade-tools/all-gates.sh` runs lane T, four verdicts:

- `cargo test`: the textbook checks and the rig's analysis.
- **The bores read now are the bores committed** in `bores/cut/` and `bores/predict/`. A generator that moves a hole shows up as a diff even before it goes out of tune.
- **Every drawn bore is in tune**, `TUNING OK`, within 5 cents or its stated tolerance. Moving one octagonal-flute hole by 3mm fails it at −7.9 cents.
- **The engine agrees with every designing model**, `MODEL MATCH`, to 0.01 cents.

Everything is read into a temporary directory. When a generator changes on purpose, rerun `python3 tools/read_cut_files.py` and commit the bores with the change.

## Files

- `Cargo.toml`, `Cargo.lock`: the crate and its pinned dependencies (serde, serde_json).
- `src/lib.rs`: the engine. `src/measure.rs`: the sweep, WAV in and out, the FFT and the peak analysis. `src/main.rs`: the command line.
- `tests/physics.rs`: the textbook checks. `tests/measure.rs`: the rig's analysis through a coloured loudspeaker, and WAV round trips.
- `tools/read_cut_files.py`: the bores as drawn. `tools/svgparts.py`: the shapes in a cut file. `tools/export_models.py`: the designing models' own bores.
- `bores/cut/`, `bores/predict/`, `bores/models/`: what those write. `bores/rig/reference-cavity.json`: the rig's reference.
- `.doc-audit-ignore`, `.doc-audit-generated`: the sibling repositories' files this page names, and the generated folders.
- `index.html`: this page, built from `README.md` by lasermade-tools' md2html.py.
- `LICENSE`: CC0.

## Credits

Written by Michael Gernreich with [Claude](https://www.anthropic.com/claude) (Anthropic's Claude Opus 5.5, in Claude Code), which wrote the engine, the readers and this page; every commit it made carries a `Co-Authored-By: Claude` line.
