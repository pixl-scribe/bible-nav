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

```
npm run tauri dev    # runs app in dev mode
npm run lint         # lints the typescript
npm run pseudolocale # regenerate the locales/en-XA.json pseudo locale file for i18n testing
npm run copy-db      # copy and compress default kjv sqlite DB to resources folder
```

## Build Steps

```
NO_STRIP=true npm run tauri build
```

The compiled executable can be found in `src-tauri/target/release`

## Run Steps

