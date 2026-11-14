# Built-in key statements

Key statements published by the maintainer, copied here from
`bible-nav-modules/keys/statements` by `npm run copy-db`. Each is
`bible-nav-keys-<sequence>.json` with its `.sig` file.

Every statement here must also be listed in `BUILT_IN_STATEMENTS` in
`src/modules/keys.rs`, with `include_bytes!`, so each release carries every
statement known when it was built (a test fails until it is):

```rust
pub const BUILT_IN_STATEMENTS: &[BuiltInStatement] = &[BuiltInStatement {
    name: "bible-nav-keys-1.json",
    statement: include_bytes!("../../keys/bible-nav-keys-1.json"),
    signatures: include_bytes!("../../keys/bible-nav-keys-1.sig"),
}];
```

See "Key statements" in `bible-nav-modules/docs/integrity-spec.md`.
