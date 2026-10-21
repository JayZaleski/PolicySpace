# Stage 03: factories — Consolidation

## Context
Files `build` must read before starting:

- `CLAUDE.md` (the loop, safe points, and that `build` never commits).
- `crates/engine/src/factories.rs`, `crates/engine/src/resources.rs`, `crates/engine/src/businesses.rs` (the per-hex index pattern C01 copies).
- `crates/engine/src/setting/mod.rs` (`load_setting`, the order the readers `finish` in), `crates/engine/src/setting/resources.rs`, `crates/engine/src/setting/factories.rs`.
- `crates/engine/src/decider/host.rs`, `crates/engine/src/decider/batch.rs`, `crates/engine/src/decider/decision.rs`.
- `crates/engine/src/setting/error.rs`, `crates/engine/src/setting/reader.rs`, `crates/engine/src/setting/goods.rs`, `crates/engine/src/setting/technologies.rs`, `crates/engine/src/setting/businesses.rs`, `crates/engine/src/setting/map.rs`.
- `crates/engine/src/lib.rs`, `crates/engine/src/world.rs`.
- The integration tests that pin the loader's messages: `crates/engine/tests/goods.rs`, `technologies.rs`, `businesses.rs`, `factories.rs`, `resources.rs`; and the decider tests `crates/engine/tests/decider.rs`, `decider_host.rs`, `recordings.rs`.
- `tools/check.py` (the one check command: `.venv/Scripts/python tools/check.py`).

Behavior must not change anywhere: every existing test keeps its expectations, including every error message text, every turn line, and every recording. The build is one `build` in the main tree (no Packages): the plan is small and the recommendations share files.

## Recommendations

### C01: Per-owner indexes for factories and deposits
- **Status:** applied
- **Summary:** `Factories::owned_by` and `Deposits::owned_by` filter the whole list for every business, every turn (`World::operate_business`, `decider::view::build`) and every dev-UI frame (`text::businesses`). Give both lists a `by_owner` index built once at load, like the `by_hex` index they already have. Same results, in the same (ID) order; no caller changes.
- **Changes:**
  - `crates/engine/src/factories.rs`: add `by_owner: IdVec<BusinessKind, Vec<FactoryId>>` to `Factories`. `from_factories(factories: Vec<Factory>, hex_count: usize, business_count: usize)` fills it in ID order and asserts `factory.owner.index() < business_count` with the message `"{owner:?} is not from a list of {business_count} businesses"` (same style as the hex assertion). `owned_by(&self, business: BusinessId) -> impl Iterator<Item = FactoryId> + '_` returns `self.by_owner.get(business).iter().copied()`; document that it panics if `business` isn't from the businesses the list was built with (and on `Factories::default()`, like `on_hex`). Update the module doc's mention of the index.
  - `crates/engine/src/resources.rs`: the same for `Deposits`: `by_owner: IdVec<BusinessKind, Vec<DepositId>>`, `from_deposits(deposits, hex_count, business_count)` with the same assertion, `owned_by` from the index. A deposit with `owner: None` is in no owner's list.
  - `crates/engine/src/setting/resources.rs`: `DepositsReader::finish(self, hex_count: usize, business_count: usize)` passes both through.
  - `crates/engine/src/setting/factories.rs`: `FactoriesReader::finish(self, hex_count: usize, business_count: usize)` likewise.
  - `crates/engine/src/setting/mod.rs` (`load_setting`): finish the businesses before the deposits and factories, and pass `businesses.len()` to both `finish` calls. Nothing else in the order changes (`markets.finish()` still first, then `map.finish()`).
  - Callers of `owned_by` (`world.rs`, `decider/view.rs`, `crates/dev-ui/src/text/businesses.rs`) are unchanged.
- **Tasks:**
  1. Add the index and the `business_count` parameter to `Factories`; rewrite `owned_by`; update the unit tests in `factories.rs`: `sample_factories()` builds with 3 businesses (so the existing `owned(b[2])` is-empty assertion still holds), add a test that `from_factories` panics with the new message for an owner beyond `business_count`, and a test that `owned_by` on `Factories::default()` panics.
  2. The same for `Deposits` in `resources.rs`: `sample_deposits()` passes `0` businesses (no deposit is owned); the `owned_by` test passes 3; add the two panic tests.
  3. Thread `business_count` through the two readers' `finish` and reorder `load_setting`.
  4. `cargo build --workspace` compiles with no other caller changes.
- **Tests:** `cargo test --workspace` unchanged except the unit tests above. `crates/engine/tests/businesses.rs` (`owned`, deposit owners `[0, 0, 1, 1, none]`), `crates/engine/tests/factories.rs` (`owned_by` per business in ID order), `crates/engine/tests/decider.rs` (the sample views' factories and deposits) and the dev-UI business-section tests must pass unchanged: they are the behavior check.

### C02: Decision-point lookups by name
- **Status:** applied
- **Summary:** `DeciderHost::decision_point` walks every business once per decider to find its businesses, and `Reply::validate` searches the view list for every decision and the decision list for every view; the host then searches the decisions again for each refused answer. All are quadratic in the number of businesses. Group once with maps. Same answers, same error order, same report.
- **Changes:**
  - `crates/engine/src/decider/host.rs` (`decision_point`): before the decider loop, build `BTreeMap<&str, Vec<BusinessId>>` from one pass over `businesses.iter()` (pushing in ID order, so each decider's list stays in business ID order); in the loop take each decider's list with `remove(name.as_str()).unwrap_or_default()`. Deciders are still taken in name order (the `self.deciders` `BTreeMap`). In the `Ok(validated)` branch, take the refused decision from the `Answer` (see below) instead of searching `reply.decisions`.
  - `crates/engine/src/decider/batch.rs`: change `Answer::Refused(DecisionErrors)` to `Answer::Refused { decision: &'a Decision, errors: DecisionErrors }` and document it. In `Reply::validate`: collect the request's business names into a `BTreeSet<&str>` for the unknown-business check, and the decisions into a `BTreeMap<&str, &Decision>` while walking `self.decisions` in reply order (an `insert` that returns `Some` is a `DuplicateBusiness`; a name not in the set is `UnknownBusiness`, checked first as now, so the problem order is unchanged). Build each view's `Answer` from `map.get(view.business.as_str())`.
  - Tests: `batch.rs` unit test `an_invalid_decision_is_refused_without_refusing_the_reply` matches the new `Refused { decision, errors }` shape and also asserts `decision.business == "a"`. Check `crates/engine/tests/decider.rs` and `recordings.rs` still compile (they match `Answer::Valid`/`Answer::Nothing` only).
- **Tasks:**
  1. Change `Answer::Refused` and `Reply::validate` in `batch.rs`; fix its unit tests.
  2. Group businesses by decider once in `host.rs`; use the answer's decision in the refused branch.
  3. Run the decider tests.
- **Tests:** `cargo test -p policyspace-engine` (unit tests in `batch.rs`, `host.rs`; integration tests `decider.rs`, `decider_host.rs`, `recordings.rs`, `business_trade.rs`, `external_decider.rs`). `decider_host.rs`'s `each_decider_receives_only_its_businesses_in_id_order_and_an_empty_request_otherwise` and `a_refused_decision_idles_only_its_business` are the behavior check and must pass unchanged.

### C03: One generic error-list type
- **Status:** applied
- **Summary:** `LoadErrors` (`setting/error.rs`), `DecisionErrors` (`decider/decision.rs`) and `ReplyErrors` (`decider/batch.rs`) are three identical newtype wrappers: a never-empty `Vec`, `iter`, `len`, `is_empty`, `IntoIterator for &`, and a `Display` that writes one error per line. Replace them with one generic `ErrorList<E>` behind the three existing names as type aliases, so the public names and every caller stay the same.
- **Changes:**
  - New `crates/engine/src/error_list.rs`: `#[derive(Debug, Clone, PartialEq, Eq)] pub struct ErrorList<E>(Vec<E>);` with `pub(crate) fn new(errors: Vec<E>) -> Self` (asserts `!errors.is_empty()`, message `"an error list holds at least one error"`), `pub fn iter(&self) -> std::slice::Iter<'_, E>`, `pub fn len(&self) -> usize`, `pub fn is_empty(&self) -> bool` (always false; keep it so `len`/`is_empty` lint stays satisfied), `impl<'a, E> IntoIterator for &'a ErrorList<E>`, `impl<E: fmt::Display> fmt::Display` (errors separated by `\n`, no trailing newline, exactly as the three impls do today), `impl<E: fmt::Debug + fmt::Display> std::error::Error`. Under `#[cfg(test)]`, `pub(crate) fn lines(&self) -> Vec<String>` (each error's `Display`), replacing `LoadErrors::lines`. Module doc explains the three uses. Unit tests: `new` panics on empty; `Display` joins with newlines; `iter`/`len`/`into_iter` agree.
  - `crates/engine/src/lib.rs`: `pub mod error_list; pub use error_list::ErrorList;`.
  - `crates/engine/src/setting/error.rs`: `pub type LoadErrors = ErrorList<LoadError>;` with the current doc comment ("Every problem found in one failed load, in sorted order. Never empty."); delete the struct, its impls and the test-only `lines`; `ErrorSink::into_result` builds `ErrorList::new(self.errors)`; `into_lines` keeps calling `errors.lines()`.
  - `crates/engine/src/decider/decision.rs`: `pub type DecisionErrors = ErrorList<DecisionError>;` with its doc ("in document order. Never empty."); delete the impls; `validate` returns `Err(ErrorList::new(problems))`; the `error_displays` test builds `ErrorList::new(vec![...])`.
  - `crates/engine/src/decider/batch.rs`: `pub type ReplyErrors = ErrorList<ReplyError>;` with its doc; delete the impls; `validate` returns `Err(ErrorList::new(problems))`.
  - `crates/engine/src/setting/mod.rs` re-exports `LoadErrors` from `error` as now (the alias); `lib.rs` re-exports stay as they are.
- **Tasks:**
  1. Write `error_list.rs` with its tests; declare and re-export it in `lib.rs`.
  2. Replace the three wrappers with aliases and `ErrorList::new` at the three construction sites; move `lines`.
  3. `cargo build --workspace`; the dev UI (`text::error_lines` uses `iter` and `len`) compiles unchanged.
- **Tests:** `cargo test --workspace`. The `Display` texts asserted in `decision.rs` (`error_displays`), `batch.rs` (`every_reply_problem_is_reported`), `host.rs` (`failure_and_report_displays`), `crates/engine/tests/setting_loader.rs` and the dev-UI `error_lines` tests pin the format and must pass unchanged.

### C04: One "whole number of at least 1" reader
- **Status:** applied
- **Summary:** The loader checks "an integer, at least 1, within range" in four places with four code shapes: `perishes_after` (`setting/goods.rs`), `construction_turns`/`lifetime` (`turns` in `setting/technologies.rs`), deposit counts (`read_deposits` in `setting/businesses.rs`) and `count` (`setting/map.rs`). One helper on `EntryReader` does the check and the two messages; the four call sites shrink to a line each. Every message text stays byte for byte.
- **Changes:**
  - `crates/engine/src/setting/reader.rs`: add `pub fn at_least_one(&mut self, prefix: &str, value: Located<i64>, max: Option<u32>) -> Option<u32>`: if `value.value < 1`, report `"{prefix} must be at least 1"` at `value.at` and return None; if it doesn't fit a `u32`, or `max` is `Some(m)` and it exceeds `m`, report `"{prefix} is out of range"` followed by `" (the most is {m})"` only when `max` is `Some`, and return None; otherwise `Some(n)`. Doc it next to `positive_quantity`, which it mirrors. Unit test in `reader.rs`'s test module: 1, a large valid value, 0, -3, `u32::MAX + 1` with `None`, and a value above `max` with `Some`, checking the exact messages.
  - `crates/engine/src/setting/goods.rs`: `perishes_after` becomes `reader.optional_integer("perishes_after")` then `reader.at_least_one("key 'perishes_after' in good", value, None)` → `Perishes::After(NonZeroU32::new(n).expect("at least 1"))`, `Perishes::Never` when absent or invalid (as now).
  - `crates/engine/src/setting/technologies.rs`: `turns(reader, key)` becomes `required_integer(key)` then `at_least_one(&format!("key '{key}' in {label}"), v, None)`.
  - `crates/engine/src/setting/businesses.rs`: `read_deposits` maps each value through `at_least_one(&format!("key '{}' in deposits of {label}", key.value), v, None)`.
  - `crates/engine/src/setting/map.rs`: `count` keeps its absent/wrong-type handling and calls `at_least_one(&format!("key 'count' in {label}"), count, Some(MAX_COUNT))`.
- **Tasks:**
  1. Add `at_least_one` and its unit test.
  2. Rewrite the four call sites; keep each function's existing unit tests (`businesses.rs` `deposit_counts_are_whole_numbers_of_at_least_one_that_fit_a_u32`, `resources.rs`'s `count` tests) passing as they are.
- **Tests:** `cargo test -p policyspace-engine`: the pinned messages in `tests/goods.rs` (`perishes_after ... must be at least 1` / `is out of range`), `tests/technologies.rs` (`construction_turns`, `lifetime`), `tests/businesses.rs` (`in deposits of business must be at least 1` / `is out of range`), `tests/resources.rs` and `tests/factories.rs` (`key 'count' ... must be at least 1` / `is out of range (the most is 1000000)`) are the behavior check.

### C05: Stale comments and a leftover
- **Status:** applied
- **Summary:** Comments written during Stages 02 and 03 still point at the wrong Stage or promise work that is done, and `World::new` initialises a field twice. Comment-only, plus one two-line cleanup; no behavior change.
- **Changes:**
  - `crates/engine/src/market.rs` line 7: "the seam Stage 05 replaces with the live market" → "Stage 04". Line 170: "Crate-private: `World::market` builds it, and later Steps build it inside the World." → "Crate-private: `World::market` and the World's trade round build it."
  - `crates/engine/src/trade.rs` lines 23–24: "Stage 05 replaces the inside of the round" → "Stage 04".
  - `crates/engine/src/world.rs` line 17: "the seam Stage 05 replaces" → "Stage 04".
  - `crates/engine/src/growth.rs` line 11: "Stage 05 revisits" → "Stage 04".
  - `crates/engine/src/stockpile.rs` lines 3–5: "each hex has a stockpile of unowned goods and each POP has its own (see the [`world`](crate::world) module); later Stages give industries theirs." → "each hex has a stockpile of unowned goods, and each POP and each business has its own (see the [`world`](crate::world) module)."
  - `crates/engine/src/technologies.rs` lines 21–22: "these numbers, as pure functions; Step 08 of this Stage wires it into the turn." → "these numbers, as pure functions, and the World runs them for each business every turn."
  - `crates/engine/src/businesses.rs` lines 2–3: "a decider name (the name of the decider that will control it; Step 07 resolves it, this Step only stores it)" → "a decider name (the name of the decider that controls it, resolved against the host when a world is created; see [`decider::host`](crate::decider::host))".
  - `crates/engine/src/decider/batch.rs` lines 43–44: "the recording Step 06's Python tests read and Step 07's replay decider answers from." → "the recording the Python tests read and the replay decider answers from." Line 145: "A decision that failed validation; what happens next is Step 07's." → "A decision that failed validation; the host idles its business for the turn."
  - `crates/engine/src/setting/mod.rs` line 7: "later Steps build the world state from it" → "the World is built from it"; line 67: "# Adding a section type (for later Steps)" → "(for later Stages)".
  - `crates/engine/tests/recordings/README.md` lines 3–4: "and, later, Step 07's replay decider" → "and the replay decider (`crates/engine/src/decider/replay.rs`)".
  - `crates/engine/src/world.rs` `World::new` (lines 287–291): take `let business_count = start.businesses.len();` before the struct literal and initialise `business_answers: IdVec::from_vec(vec![BusinessAnswer::Nothing; business_count])` there; delete the later `world.business_answers = ...` assignment.
- **Tasks:**
  1. Apply each text change above (search for the quoted text; line numbers are as of the safe point).
  2. Apply the `World::new` change.
  3. `cargo doc -p policyspace-engine --no-deps` builds without warnings (the new intra-doc link resolves).
- **Tests:** `cargo test --workspace` unchanged; `tests/world.rs`, `decider_host.rs` and `business_trade.rs` cover `World::new`.

## Done When
Every approved recommendation is applied, `.venv/Scripts/python tools/check.py` passes (format, clippy, every Rust test, every Python test), and no test expectation, error message, turn line or recording changed.
