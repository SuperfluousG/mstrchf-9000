# Synthetic metadata correction and toolchain custody

Root commissioned this source-only correction after identifying the inherited
December metadata gap and containing an external shared-toolchain change.
It edits only the new calendar fixture files. Existing F11/native-v4 sources and
all compiler/test outputs remain unchanged.

Pre-change SHA256:

| File | SHA256 |
| --- | --- |
| native.rs | `468c4c7797b0b24f686fe362465904637b0986d44aa0d020ca6feadb781a2e92` |
| expectations.md | `1cb40b3aa2854f2131627f6f454fb9278eb773d3dd06dd2f5f81d7ecaf671f6e` |
| README.md | `310c7149da1b7e8b9abb70b1d94cf8eea8353a4a61a8d05dd40c0f8cf119a13d` |
| main calendar-implementation/fixture-premise-06.txt | `d75123edf4fb88fd35dbc55e567c9254a19515c5dfbe4baa922a291dc7f3aee4` |

The inherited F11 seed has December review, source-retrieval, restriction and
privacy-confirmation metadata. Its earlier successful native checks established
structural acceptance, not a truthful October chronology. The new private
`seed()` wrapper changes only recognised December metadata keys (`retrieved_at`,
`confirmed_at`, `at`) to `2026-10-01T00:00:00Z`. It does not rewrite planned dates,
meal times, production anchors, storage clocks, quantities or instruction text.
It rehashes each exact source recipe followed by its dependent recipes and
rebinds plans, presets, stamps, method/resource/class evidence and phase keys.
Every later variant continues through the existing `finish` validation chain.
This remains explicit synthetic construction, never production admission.

Compiler custody observed before containment:

- The fixture worker invoked the PATH command `rustc`; read-only resolution
  identifies `C:\Users\Gregg\.cargo\bin\rustc.exe` (the rustup proxy).
- `rustc --version` actually returned
  `rustc 1.99.0 (b940084d7 2026-09-28)` before the isolated harness runs.
- The isolated harness linked the then-built
  `libmstrchf_core-7bbfc91f91faebd6.rlib`,
  `libmstrchf_contracts-6d44d53182f85f61.rlib` and
  `libserde_json-934e83ffc6c29bd9.rlib` from this checkout's target/debug/deps.
  No direct underlying rustc binary path was captured during those runs.
- Root reported that shared stable had changed from 1.94 to 1.99 elsewhere.
  The fixture worker did not request a toolchain update or install a compiler.

No compiler, formatter or test command has run for this metadata correction.
All preceding green claims are restricted to their preceding source bytes and
observed compiler route. Root must rerun the revised fixture/calendar checks
through the authorised contained toolchain before assigning a new verdict.
