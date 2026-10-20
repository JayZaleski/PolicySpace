# Bugs

Code that doesn't do what its Step's `design.md` says. Status: `open` → `specified` (`fix.md` written) → `built` (fix approved at Code Review) → `fixed` (committed); or `wont-fix` with the reason. See Bug fixes in `CLAUDE.md`.

## Bug 01: arithmetic-rule-bids-disagree-with-fractions
- Status: open
- Step: stage-03 step-06-business-script
- Found: during Code Review of stage-03 step-08-business-trade
- Description: The arithmetic rule bids labor 6.25 for the sample's toolmaker but asks for a maintenance fraction a hair under 1/24 (0.04166666666666663), so the engine uses about 2.2e-16 less labor than was bought. The speck perishes every turn and prints on the turn line (`perished 0.0000000000000002220446049250313 unskilled_labor, ...`). Step 06's design has the rule bid exactly what its plan needs. Fix in the rule (bids computed from the same fractions it sends, or fractions snapped to the engine's slice arithmetic), never by rounding in the engine.
