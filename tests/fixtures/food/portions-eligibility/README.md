# Task B synthetic fixtures

`cases.json` maps the 13 B case groups to actual named Rust tests and independently supplied literal checks. The unchanged 55-case oracle is `../oracle-v1.json`; its SHA256 is pinned in the manifest. No expected value is regenerated from production functions.

The test target `food_portions_eligibility` builds native recipe and meal records from the accepted admission fixture and contract meal fixture, then binds synthetic evidence through the real checked admission constructors. Names/IDs in the oracle are symbolic test notation; concrete IDs and canonical decimal rationals are supplied by these local test builders. All evidence and food quantities are synthetic, not observations or recommendations.

The target additionally tests bounds, invalid direct tuple construction, exact override replacement, missing admission evidence, disabled shopping and meals, hard-union weakening, duplicate/foreign recipients and presets, applicability constraints, direct excluded ingredients, contradictory/unknown evidence, missing cold evidence, equipment phase ambiguity, conversion revisions/endpoints/conflicts/multiple components, no multi-hop inference, deterministic conflicts and incomplete-class/storage fresh-positive controls.

F09 verifies the common pure predicate for both caller labels; it is not UI integration. F05/F07 establish immutability and occasion-scoped requirements, not database guest-persistence acceptance. F40 covers conversion only; P08 pack selection/pricing is excluded. F10 fixed-selection retention and C/D/E scheduling/demand are excluded.

The exact frozen `validate_extra` signature has no `MealOverrides` input: recipient servings are checked against attendee default/meal-type preferences only. Unsupported meal-local extra quantities fail closed. Nonempty extra applicability requirements are additional evidence obligations, never a replacement for recipients' restrictions; real content-authoring interpretation still needs root confirmation.

Different nominal yield/discrete-batch yield has no separate admitted equivalence in this API and returns unsupported. On-the-move use with preparation equipment cannot establish per-phase availability and stays unresolved (or ineligible when a required appliance is absent). Conversion returns the frozen `Quantity`; consumers must preserve the supplied evidence ID, recipe reference and original/inverse endpoint operation in their own demand trace. Original recipe evidence is immutable.
