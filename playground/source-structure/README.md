# Source structure playground

```sh
cargo run --locked --bin hat-source-decompose < playground/source-structure/input.json
cargo test --locked --test structure --test input
```

The synthetic document is decomposed by the exact public Zixcel JSON structure crate. Keys, arrays and source locators remain source representation; no source label becomes a semantic type or permission. Owner input tests additionally preserve fixed values and the original source, reject stale revisions/undeclared or sensitive inputs, and omit private paths from public declarations.

Language interpretation is downstream: its owner supplies its own context and translates supported source shapes. This public role has no private language dependency or compatibility decoder for the old context-bearing request. The graph/contribution adapters remain explicitly source-owner formats; they are not a language compiler.
