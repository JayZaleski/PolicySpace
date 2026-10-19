# Step 04: factory-mechanics

## Summary
This Step gives factories their physics: what happens when a business runs a factory at some fraction of its capacity, how a factory wears each turn, and what it costs to maintain one. The three operations are pure functions over one business's factories, their technologies, the business's stockpile and the business's instructions, each with its own tests. Nothing in the turn calls them yet: the fractions a business asks for come from its decider (Steps 05 to 07), and Step 08 wires the functions into the turn. The rules follow what Step 01 recorded as the meaning of a technology's numbers; this Step settles the cases Step 01 and Step 03 left open: what an exhausted factory is, the order of the operations within a turn, how a business's factories share its goods, and how rounding is kept from changing a factory's life.

## Behavior

### The instructions
Each turn a business gives, for each of its factories, two numbers between 0 and 1 inclusive:

- **A run fraction**: how much of the factory's capacity to use this turn. It is a fraction of capacity, not an amount of output, so it is the same kind of number as the maintenance fraction and the condition, it is trivially bounded, and it keeps its meaning if the factory's technology or capacity changes later. The decider sees the capacity in its view (Step 05), so turning a fraction into units is one multiplication on its side.
- **A maintenance fraction**: how much of the technology's full capital bundle to replenish this turn.

The functions assume the numbers are in range; Step 05's validation makes sure they arrive that way.

### Running
A factory that holds at least one lifetime-slice of capital (a condition of at least one over its technology's lifetime) runs at its technology's capacity times the run fraction. Condition does not slow a factory: it produces at full capacity until its capital is exhausted, as the vision says. A factory holding less than a slice does not run at all (see Wear). The inputs for that output, from the technology's fixed-proportion recipe, leave the business's stockpile, soonest-perishing groups first, as every removal does. A technology with no inputs runs on nothing.

The output is **delivered to the business's stockpile only after every one of the business's factories has run**, and it enters as goods added in the current turn, perishing by the good's own rule. So this turn's output cannot feed another of the business's factories this turn. The reason is the rule below: a business's factories run as if at the same moment. A consequence to note, not police: a good that perishes after one turn is useless as factory output, because it is gone before anyone could sell or use it. That is a setting author's mistake, like a technology that can't pay for itself.

### No order among a business's factories
Nothing a business sees may depend on which of its factories has the lower ID. Its factories run as if simultaneously. Two rules carry this:

- Output is delivered after all have run (above).
- **A shortage is shared, not first-come-first-served.** When the business's factories together need more of a good than the stockpile holds, every factory that needs that good is cut by the same proportion: what is held divided by what was asked, for that good. A factory that needs several goods is cut by the worst of its shortages, so its actual run fraction is the one it asked for times the smallest of those proportions over its inputs. This is one pass. Cutting a factory for lack of one good can leave some of another good unused that a different factory could have taken, and the engine does not go back to redistribute it; the leftover stays in the stockpile. The rule is deliberately this simple because of the principle below.

Maintenance (below) shares a shortage across the business's factories the same way.

### Wear
Every turn, whether or not it ran, a factory loses a slice of condition equal to one over its technology's lifetime, stopping at zero. A factory holding less than one slice **does not run**; that is what "shut down" means. Every turn of output costs exactly one slice, so a business cannot keep a factory running on a sliver of capital: if a factory ran whenever its condition was above zero, a business could hold it a hair above zero, run it at full capacity, let wear take the hair (the rest of the slice forgiven because wear stops at zero) and replace the hair for next to nothing. The price of the rule is that a partial slice (half a slice left, say) produces nothing and is lost to wear. Shutdown is a fact derived from the condition and the technology's lifetime, not a separate state, and it is not final: see maintenance.

**Rounding never changes a factory's life.** A factory that starts full and is never maintained runs exactly its lifetime in turns, no more and no fewer. Condition is a decimal number and repeated subtraction of a slice need not land on exactly zero in computer arithmetic, so the spec chooses the arithmetic that guarantees this rule; a speck of leftover condition must not buy a factory an extra turn of output.

### Maintenance
Maintenance is construction run for a fraction of its time, as Step 01 recorded. Replenishing a fraction f of the capital bundle costs f of the capital bundle plus f of the construction bundle, both taken from the business's stockpile, soonest-perishing groups first. The goods spent on the capital bundle become the factory's capital; the construction bundle's goods are consumed. The fraction a business asks for is reduced, in this order:

1. to at most one over the technology's construction turns, the most that can be done in one turn;
2. to at most what brings the factory to full condition;
3. to what the goods on hand cover, shared proportionally with the business's other factories as under running.

The factory's condition rises by the fraction actually done. A shut-down factory can be maintained like any other, so an exhausted factory comes back the way construction builds one, slice by slice, and runs again once it holds a full slice. A technology whose construction takes longer than its lifetime can't get there: wear removes more each turn than maintenance can add back, so such a factory, once exhausted, stays exhausted until Stage 07 can scrap or rebuild it (Step 01 allowed this case: it wears out faster than it can be maintained). This keeps one number as the factory's whole state and matches the vision's "maintenance mirrors construction". The price of it is that a business can leave a factory at zero indefinitely as a free option on reviving it; that is accepted.

### Order within a turn
For each business: **run, then wear, then maintain.** Running takes its inputs from the stockpile before maintenance takes its goods. This order is a rule every business knows, not an accident of IDs, and it is chosen so that the numbers mean what Step 01 says:

- Running before wear makes a full, never-maintained factory run exactly its lifetime in turns. Wearing first would give one turn fewer.
- Maintaining after wear lets a business that replenishes one lifetime-slice each turn hold a factory at full condition. Maintaining first would be capped at full and then lose a slice, so steady maintenance could never keep a factory full.

Step 08 places this sequence in the turn; this Step designs and tests the functions for it.

### The safety net principle
The cuts for missing goods are a backstop that keeps the world consistent whatever a decider sends. They are not the normal path. A business is not supposed to send orders its goods can't cover: the arithmetic rule (Step 06) asks only for what it can supply. So the sharing rules stay simple, and the functions make a cut visible rather than absorbing it silently.

### What is recorded
For each factory, each turn, the functions report: the run fraction asked for and the one actually run; the inputs used and the output made; the slice of condition lost to wear; the maintenance fraction asked for and the one done, with the goods it cost; and the condition before and after. "Asked for" beside "done" is what shows a cut order. Step 08 keeps this in the business's per-turn record and Step 09 shows it in the dev UI.

## Player-facing Changes
None. Nothing in the turn, the report, the setting format or the dev UI changes. The functions are observable only through their tests until Step 08.

## Interactions
- **Technologies** (Step 01): the recipe, capacity, capital and construction bundles, construction turns and lifetime, with the meanings Step 01 recorded. This Step implements them.
- **Factories** (Step 03): the condition is the state these functions read and change. Step 03 placed a factory at a condition above zero and left zero to this Step; the engine type already accepts zero.
- **Businesses** (Step 02): the stockpile inputs and maintenance goods come from and output goes to. Nothing here changes a business's cash: at the dummy market's fixed prices, buying inputs is Step 08's trade round, and these functions only use goods already in the stockpile.
- **Stockpiles** (Stage 01 Step 06, Stage 02 Step 02): removal soonest-perishing first; output added as goods of the current turn.
- **Stone-Geary demand** (Stage 02 Step 05): the pattern of a pure function with its own tests, wired into the turn by a later Step.
- **Later Steps:** Step 05 defines the instructions as part of the action vocabulary, with these bounds. Step 06's rule asks only for what it can cover. Step 08 calls the functions in the turn, in the order above, after the trade round, and records the results. Step 09 shows the record. Stage 07 builds, scraps and retools factories using the same bundles.

## Out of Scope
- Wiring into the turn, the per-business record in the World, and the turn report: Step 08.
- Choosing the fractions, and any cash limit on them: Steps 05 to 08.
- Redistributing goods left unused after a shared cut.
- Construction of new factories, scrapping, retooling: Stage 07.
- Capacity falling with condition, or any effect of condition other than running or not.
- Any check that a setting's technologies or factories make economic sense.
- Showing any of this in the dev UI: Step 09.

## Acceptance Criteria
- A factory asked to run at a fraction makes capacity times that fraction of its output, its inputs leave the stockpile in recipe proportions, soonest-perishing first, and the output appears in the stockpile only after every factory of the business has run, perishing by its good's rule from the current turn. A technology with no inputs runs on nothing.
- With two factories of one business sharing a scarce input, both are cut by the same proportion whatever their IDs; a factory short of two goods is cut by the worse shortage; the leftover of a good freed by that cut stays in the stockpile. Running the same case with the factory IDs swapped gives the same results per factory.
- Every factory loses one lifetime-slice of condition per turn, run or not, stopping at zero. A factory holding less than one lifetime-slice makes nothing whatever it is asked; one holding one and a half slices runs once and makes nothing the turn after. A factory that starts full and is never maintained runs on exactly as many turns as its lifetime, including for lifetimes whose slice isn't exact in decimal arithmetic (for example 3 and 24), and makes nothing on the turn after.
- Maintenance of a fraction costs that fraction of both bundles from the stockpile and raises the condition by it; the fraction asked for is reduced first to one construction-turn's slice, then to what fills the factory, then to what the goods on hand cover, shared proportionally across the business's factories; a factory at zero can be maintained and runs again once it holds a full slice. A business replenishing one lifetime-slice each turn, in the order run, wear, maintain, holds its factory at full condition indefinitely.
- Each factory's record shows the fractions asked for and done for running and maintenance, the inputs used, the output made, the wear slice, the goods maintenance cost, and the condition before and after; a cut order is visible as asked above done.
- Every existing test passes unchanged: no setting, report, turn or dev UI text changes.
