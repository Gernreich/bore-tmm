# bore-tmm

[← All projects](https://gernreich.github.io)

What a described bore plays. Describe a wind instrument's bore from its driven end down: its lengths of cylinder and cone, the tone holes along it, how it ends and what drives it. bore-tmm then gives each fingering's note, how far that note is from where it should be, and every register the fingering has.

It is a transfer-matrix model, the same physics as the four Python models in the LaserMadeMusic repositories, gathered in one place:

- [naf_model.py](https://github.com/Gernreich/square-flute/blob/main/naf_model.py), in square-flute
- [flute_model.py](https://github.com/Gernreich/octagonal-flute/blob/main/flute_model.py), in octagonal-flute
- [chanter_model.py](https://github.com/Gernreich/practice-chanters/blob/main/chanter_model.py), in practice-chanters
- [ghb_chanter.py](https://github.com/Gernreich/ghb-chanter/blob/main/ghb_chanter.py), in ghb-chanter

Each of those models designs its instrument by solving where the holes go. bore-tmm does not design anything. It answers the question a cut file has to answer: given this bore, what does it play?

```
cargo build --release
./target/release/bore-tmm check --compare bores/models/*.json   # against the designing models
./target/release/bore-tmm check --tol=5 BORE.json                # fail any note > 5 cents out
./target/release/bore-tmm registers BORE.json                    # each fingering's resonances
./target/release/bore-tmm curve BORE.json D4 --to=2000           # |Z| at the drive, CSV
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

Positions are in mm down the bore from its driven end. A cone segment takes `d1_mm` and `d2_mm` in place of `d_mm`. A hole with `always_open` is a vent. A note can carry `span` (how far either side of `hz` to look) and `model_hz` (what the designing model says it plays).

## Agreement with the Python models

`tools/export_models.py` writes each model's own design to `bores/models/`, with that model's prediction for every note. `check --compare` then reads:

```
MODEL MATCH: 49 notes, worst 0.00000 cents from the designing model
```

That covers seven bores: the octagonal flute, the four square-flute builds, the octagonal practice chanter and the GHB chanter. bore-tmm checks all seven in milliseconds, where the Python models took seconds.

`cargo test` checks the engine against textbook results: a reed on a cylinder is a quarter wave, a reed on a whole cone is a half wave, and a cone with no taper is a cylinder.

**One difference between the models.** The GHB chanter's model takes the cone's loss radius as its diameter / 4, half its radius. The other three take the radius itself. The GHB export keeps `loss_ratio: 0.5` so the two engines match. Set to 1, every note moves 3 to 6 cents flat. Because the GHB model only compares the strip chanter with the reference in the same model, the effect on its hole sizes should be smaller than that, but it has not been measured.

## Not yet

- **Bores read from the cut files.** The bores here are the models'. That is what each generator was asked for, not what it drew. The real check is to read the bore back from each generator's own geometry, so a drift between the model and the cut file fails a gate.
- **A gate** in `lasermade-tools/all-gates.sh`: `check --tol` over every instrument's bore, ending in `TUNING OK`.
- **Uilleann chanter and the trumpets**, which have no model yet. The uilleann chanter needs its tapered square bore. The trumpets need a lip drive, a mouthpiece, the bell's flare as cone segments, and a decision on whether bends matter at these bore sizes.
- **Measurement**: a speaker, a microphone and a capillary to measure a built bore's impedance, so the uncalibrated parts (the flute's window and the reed's correction) can be fitted to something real. `curve` already writes the curve to compare against.

## Files

- `Cargo.toml`, `Cargo.lock`: the crate and its pinned dependencies (serde, serde_json).
- `src/lib.rs`: the engine. `src/main.rs`: the command line.
- `tests/physics.rs`: the textbook checks.
- `tools/export_models.py`: writes the designing models' bores.
- `bores/models/`: those bores, one file each: `octagonal-flute.json`, `square-flute-box-square-3.json`, `square-flute-box-square-6.json`, `square-flute-stack-rect-3.json`, `square-flute-stack-rect-6.json`, `practice-chanter-8.json` and `ghb-chanter.json`.
- `.doc-audit-ignore`: the sibling repositories' files this page names.
- `LICENSE`: CC0.
