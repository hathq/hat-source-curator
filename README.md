# hat-source-curator

Turn an explicitly selected document into source-linked structural input for downstream interpretation.

## What you can do

- Decompose supported source records without assigning meaning.
- Bind explicit declared input to the exact source revision.

## Current scope

Sensitive input fields are rejected. The consuming language implementation interprets meaning; installation and input binding do not grant an external effect.

Package distribution is not activated by this documentation. Use the checked-in source and the declared dependency versions; published availability must be verified separately.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. Use the configured private registry when a dependency is not distributed publicly. Run from this repository:

```sh
cargo test --locked
```

## Documentation and source

[Usage guide](docs/getting-started.md)

[Schemas](schemas) · [Implementation and public interfaces](src) · [Verification cases](tests) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)
