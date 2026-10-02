# Source Curator HAT

Independent external source adapter, version 0.10.0. Hatter does not link this
crate. It neither creates semantic definitions nor grants execution authority.

`hat-source-decompose` turns an explicitly selected source record into neutral
StructuralInput. sem-lang, not source names, labels or paths, interprets meaning.

Owner-declared input is a separate private `OwnerInput`: a bounded Zixcel
InputDeclaration plus the exact source revision, source document and explicit
field-reference bindings. `input-contract` prints only the public declaration;
`input-bind` validates exact identity and values, applies declared values to a
transient copy, and returns StructuralInput through the same decomposer. Original
source bytes and fixed values are not modified. Sensitive fields are unsupported
by this adapter and rejected, not stored as source content.

`hat-source-input PRIVATE_PLAN ABSOLUTE_SOCKET` serves this pure binding over a
bounded local Crowsi connection. The socket parent must be private (0700).
`hat-source-decompose input-seal` seals a private plan before starting the service.
The service returns only its public declaration, source revision and fresh
generation from `describe`; `bind` requires that exact generation and contract.
It has one in-flight request, a four-second exchange deadline, and removes only
its own socket on Ctrl-C, including during an incomplete request.

After installing the signed HAT package, Hatter's `interaction` command accepts
`sourceRevision`, then an explicit `sourceRegister` with its control revision,
provider id, installed repository/digest, private endpoint and exact Role ref.
Hatter never launches the service or imports its source code. Registration is
pure input configuration, not an execution Grant. A replaced Role, provider
generation or package requires explicit registration again; no automatic rebind.
`sourceDescribe` projects the declaration and `sourceSubmit` binds values through
this service before Hatter's existing Structural Apply / sem-lang admission.
`sourceDeactivate` withdraws the registration. Browser drafts cannot change the
private plan, fixed references or bindings.

The separate `review-source` worker accepts source/subject references and emits
review summary/result references. Input binding does not execute that effect.
No arbitrary executable or JSON-editor fallback is provided. Product acceptance
must be established by installed-artifact browser tests, not CLI tests alone.
