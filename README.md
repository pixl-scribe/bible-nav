# Bible Nav

Bible Nav is a cross-platform Bible desktop application built to be
- :compass: easy to use and navigate
- :rocket: modern
- :racing_car: high performant
- :footprints: with a small footprint

## Planned Features

- [x] Themes
- [x] Module builder project using [USX](https://ubsicap.github.io/usx/index.html) as the source text format
- [ ] Bundled with KJV fully searchable with [FTS5 and porter tokenizer](https://www.sqlite.org/fts5.html#porter_tokenizer)
- [ ] Interact with strong's numbering
- [ ] Add translator notes
- [ ] Module import
- [ ] Add Geneva Bible (1599) translation module for download
- [ ] Add Greek Textus Receptus with Morphology
- [ ] Add Masoretic Hebrew with Morphology
- [ ] Support Bible Dictionary modules
- [ ] Support Lexicon modules
- [ ] USB stick ghost bible (future project for the church under persecution)

## Dev Steps

Requires a stable [Rust toolchain](https://rustup.rs/).

```
cargo run                    # runs app in dev mode
cargo test                   # runs unit tests
cargo clippy -- -D warnings  # lints the Rust code
cargo fmt                    # formats the Rust code
```

## Build Steps

```
cargo build --release
```

The compiled executable can be found at `target/release/bible-nav`

## License

Bible Nav is freely given ("freely ye have received, freely give", Matthew 10:8).
It is licensed under [MIT No Attribution (MIT-0)](https://choosealicense.com/licenses/mit-0/),
so you can copy, translate, modify, and distribute it without restriction
and without needing to ask permission. See [LICENSE](LICENSE).

In the spirit of [Let's copy, church](https://copy.church/dedicate/), this
project is offered for the free use of the church.
