# Calendar checkpoint 1 fixtures

Owner: bounded Codex fixture worker under the P09 calendar implementation owner.
Asana reference only: Nutritionist Agent: menu-picker feature (low priority),
1216779462407190. Root retains updates, integration and acceptance.

`expectations.md` records independent literal expectations before each fixture
builder. The first direct-reuse oracle was frozen before subject implementation
with SHA256 `a2f76e53176eb35925bb9c0f542349ff14176f18a081a9288d90b0c59e0cd1b8`.
Later sections were appended before their respective builders or subject tests.
The oracle deliberately distinguishes these materialised cases from the original
renderer-only C01/C03 example.

`native.rs` reuses the unchanged `food/batches/f11_fixture.rs` synthetic seed,
normalises inherited December review/source/confirmation metadata to 1 October
with exact recipe-reference rebinding, then constructs complete saved/current
native inputs. It exposes:

- `normal()`, implemented in `normal.rs`: the full reviewed C01 layout, with
  two diners, Diner 2's 100 g tomato extra, 40 g oats / 100 ml milk prep inside
  the dinner session and a separate breakfast finishing session. Complete known
  product-use bindings yield no current warnings or blockers. Exact full
  synthetic text and counts are frozen in `normal-expectations.md`.
- `direct_reuse(false)` and `direct_reuse(true)`: exact earlier source phase and
  current cold/reheated phase; both current requirements, four original demand
  lines, 400 g source yield and two 200 g allocations.
- `remix()`: source plus two remixes; six demand lines and only each remix's new
  ingredient demand. Explicit hob/microwave resource evidence accompanies the
  complete potato instructions.
- `standalone()`: one genuinely admitted synthetic storage-supported Sunday
  AdvancePrep session for Monday's source meal.
- `with_extra()`: one Cooking session, base and extra phase identities, exact
  100 g tomato extra with its recipient.
- `linked_prior()`: complete prior/subject histories and current inputs, with
  the new week's prep attached to the previous week's actual retained Cooking
  session during its unattended oven phase.

Every successful builder derives through A/B/C/D, reconstructs the complete
schedule, verifies the exact saved approval/order, and asserts a nonblocking E
continuation before returning. `finish` and `refresh_recipe` are public test
helpers for bounded negative and overflow cases. They are not source-admission
APIs. All source meaning, scaled prose and freshness applicability are explicit
synthetic assumptions; no production recipe has been admitted.

Five fixture-only tests cover six positive variants. Before the metadata
chronology correction, the last Cargo fixture run
was `cargo test -p mstrchf-calendar --test event_sources full_ --locked --offline
-j 1`: **5 passed, 0 failed**. Evidence is retained in the main project's
`loop/001/calendar-implementation/fixture-premise-06.txt`. Intermediate compiler
and premise failures remain in `fixture-premise-*` / `fixture-premise-rustc-*`.
During concurrent calendar editing, an isolated test harness also ran against
the unchanged already-built core/contracts libraries; that was a fixture test,
not a replacement for the final integrated calendar checks. Those results
belong to the earlier source bytes. At the metadata-correction handoff, the
revised source had not been compiled, formatted or tested during containment.
Later authorised validation is recorded separately below.

The later C01 commission used root's restored direct Rust 1.94 toolchain route.
`cargo test -p mstrchf-calendar --test event_sources full_normal --locked --offline
-j 1` passed **1 test, 0 failures** against the metadata-normalised source,
including the full native admission/derivation/history/continuation chain and
literal quantity/task assertions. Evidence is main
`loop/001/calendar-implementation/normal-fixture-premise-03.txt`; earlier
`normal-fixture-premise-01/02.txt` retain the nullable-name compiler correction
and the oats computed-capability correction. This was native premise validation;
the parent separately owns normal calendar-body/event assertions.

The C01 command called
`C:\Users\Gregg\.rustup\toolchains\1.94.0-x86_64-pc-windows-msvc\bin\cargo.exe`
directly, with `RUSTC` and `RUSTDOC` pinned to sibling executables,
`RUSTUP_TOOLCHAIN=1.94.0-x86_64-pc-windows-msvc` and `RUSTUP_AUTO_INSTALL=0`.
It did not invoke the mutable stable proxy.

One corrected assertion is material: core preserves source ingredient lineage
in reuse's use graph so date limits still apply. That lineage is **not** new
shopping demand. Tests instead assert unchanged literal demand quantities and
an empty reuse-session ingredient list.

Products are deliberately unbound in the non-C01 cases, so unresolved date
evidence is a retained warning premise, not a claim that food remains fresh. Store-query
completeness, source evidence truth, recipe/presentation admission, observed
cooking, checkout and calendar import remain outside these pure fixtures.
