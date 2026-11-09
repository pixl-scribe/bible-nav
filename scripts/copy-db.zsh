#!/bin/zsh
# Bundles the KJV module from bible-nav-modules, named with its version
# (<id>-<version>.db.gz), so the app can see it's already installed without
# unpacking it. The app checks the name matches the module's own metadata.
set -e
version=$(sed -n 's/^version: *//p' ../bible-nav-modules/assets/eng_kjv/config.yaml)
rm -f ./src-tauri/resources/eng_kjv.db.gz ./src-tauri/resources/eng_kjv-*.db.gz
gzip -c ../bible-nav-modules/exports/eng_kjv.db > "./src-tauri/resources/eng_kjv-${version}.db.gz"
