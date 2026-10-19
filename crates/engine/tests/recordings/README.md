# Recordings

Committed messages of the decider protocol (`crates/engine/src/decider/`), shared by the
engine's tests (`../recordings.rs`), the Python tests (`python/tests/`) and, later, Step 07's
replay decider. Three kinds of file:

- `sample_turn0.recording.json` -- a **recording**: the sample setting's turn-0 request and
  the reply the arithmetic rule (`python/policyspace/arithmetic.py`) gives it. Pretty JSON
  with a trailing newline. It is written half by each side: the engine's test writes the
  `request` half, the Python test writes the `reply` half. The engine's test checks that the
  request equals what the engine builds today and that the reply validates against it.
- `exactness.request.json` -- a **request** the engine writes (compact, one line and a
  trailing newline) holding the extreme and awkward numbers of the format (the largest turn,
  the largest and smallest floats, numbers that Python's own JSON writer lays out
  differently). Python reads it and must write it back byte for byte.
- `*.request.json` (the other six) -- **hand-written requests**, pretty JSON, edited by hand:
  one business each, for the cases the sample doesn't reach (`negative_margin`,
  `unpriced_input`, `cash_short`, `revival`, `no_market`, `seller`). The engine's test only
  checks that they parse; the Python tests assert what the arithmetic rule does with them.

## Regenerating

When the sample setting, the protocol or the rule changes, the sample recording and the
exactness request go stale and their tests fail. Rewrite them in this order:

```
set POLICYSPACE_UPDATE_RECORDINGS=1      (PowerShell: $env:POLICYSPACE_UPDATE_RECORDINGS = "1")
cargo test -p policyspace-engine --test recordings -- --test-threads=1     # rewrites the engine's halves
.venv/Scripts/python -m pytest python/tests            # rewrites the rule's reply
cargo test -p policyspace-engine --test recordings -- --test-threads=1     # confirms
```

Unset the variable afterwards; without it every test only checks.
