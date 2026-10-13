# hat-source-curator

Turn an explicitly selected document into source-linked structural input for downstream interpretation.

## What you can do

- Decompose supported source records without assigning meaning.
- Bind explicit declared input to the exact source revision.

## Current scope

Sensitive input fields are rejected. The consuming language implementation interprets meaning; installation and input binding do not grant an external effect.

Package distribution is not activated by this documentation. Use the checked-in source and the declared dependency versions; published availability must be verified separately.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. The generic JSON structure crate resolves from an exact public Zixcel Git revision; other dependencies resolve from crates.io. No private language package, private registry or adjacent checkout is required. Run from this repository:

```sh
cargo test --locked
```

## Documentation and source

[Usage guide](docs/getting-started.md)

[Schemas](schemas) · [Implementation and public interfaces](src) · [Verification cases](tests) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)

## Source structure boundary

Generic JSON decomposition is owned by `zixcel-json-structure` in `zixcel/document-processing`. This role binds source revisions and declared input, preserving source metadata. The public input contains no Sem-lang CompileContext, semantic budget or authority. Language interpretation must add its own explicit context in the language owner. Arrays preserve order instead of being treated as unordered records. The current source interface replaces the private-language-coupled form; no compatibility decoder is retained.
