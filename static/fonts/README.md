# Fonts bundled with Bible Nav

Bible Nav carries its own fonts for the original languages, so Hebrew and
Greek look the same on every computer, including a live system with few
fonts installed. Both are SIL International's own web-font files, unchanged:
under the SIL Open Font License, a converted or subset font would count as a
Modified Version and could not keep its name.

| Font | File | From | SHA-256 |
|---|---|---|---|
| Ezra SIL 2.51 (Biblical Hebrew, with points and cantillation) | `ezra-sil/SILEOT.woff` | `EzraSIL-2.51-web.zip`, software.sil.org/ezra (zip SHA-256 `7c19544c173c91e6ac47f605dae2cfa7e61e428abdafe27cf3f225fec4406357`) | `c5b6a195d770b98a576969e550f4395283086ef63769118d4ff5d0a3d28afd2e` |
| Gentium Book 7.000, Regular (polytonic Greek) | `gentium-book/GentiumBook-Regular.woff2` | `GentiumBook-7.000.zip`, software.sil.org/gentium (zip SHA-256 `fa4e35bcea62dd68befabf4bb7c2765aacd2691f51ec8ae008f5f913ef49f419`) | `302f8139d9e15b4ef719241c5d633565b06247787b11db4b650fccc0910633ea` |

Licences: Ezra SIL is under the SIL Open Font License 1.1, with its Hebrew
layout code under the MIT/X11 License (`ezra-sil/Licenses.txt`); Gentium Book
is under the SIL Open Font License 1.1 (`gentium-book/OFL.txt`). Each
folder keeps the font's FONTLOG.

`src/app.css` gives each font a `unicode-range`, so it draws only Hebrew or
Greek, and everything else keeps the app's usual font.

Checked 2026-11-18 against every Hebrew and Greek character in the sources:
Gentium Book has all 193 Greek characters used by Strong's Greek and the
Textus Receptus (eBible grctr); Ezra SIL has 85 of the 87 Hebrew characters
used by Strong's Hebrew, the Westminster Leningrad Codex (eBible hboWLC) and
Sefaria's BDB. The two it lacks, WIDE HE and WIDE TAV (U+FB23, U+FB28), occur
once, in BDB entry BDB05416, and fall back to a system font.
