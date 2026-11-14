#!/bin/zsh
# Bundles the KJV module from bible-nav-modules: the module, compressed and
# named with its version (<id>-<version>.db.gz) so the app can see it's
# already installed without unpacking it, plus its signed manifest
# (<id>-<version>.manifest.json and .manifest.sig), which the app checks
# before opening the module.
set -e
setopt null_glob  # Globs with no matches expand to nothing, for rm.
version=$(sed -n 's/^version: *//p' ../bible-nav-modules/assets/eng_kjv/config.yaml)
exports=../bible-nav-modules/exports
resources=./src-tauri/resources
rm -f $resources/eng_kjv.db.gz $resources/eng_kjv-*.db.gz $resources/eng_kjv-*.manifest.json $resources/eng_kjv-*.manifest.sig
gzip -n -c "$exports/eng_kjv-${version}.db" > "$resources/eng_kjv-${version}.db.gz"
cp "$exports/eng_kjv-${version}.manifest.json" "$exports/eng_kjv-${version}.manifest.sig" $resources/

# Key statements published in bible-nav-modules are built into the app
# (src-tauri/keys, listed in BUILT_IN_STATEMENTS in keys.rs).
statements=../bible-nav-modules/keys/statements
for statement in $statements/bible-nav-keys-*.json; do
  cp "$statement" "${statement:r}.sig" ./src-tauri/keys/
  print "Copied key statement ${statement:t}; list it in BUILT_IN_STATEMENTS in keys.rs"
done

# Cargo copies resources into target/<profile>/resources but never removes
# ones deleted here, and the app would try to install a stale bundled module
# at startup. Remove KJV files there that are no longer in resources.
for built in ./src-tauri/target/*/resources/eng_kjv*; do
  if [[ ! -e "$resources/${built:t}" ]]; then
    rm -f "$built"
    print "Removed stale build copy $built"
  fi
done
