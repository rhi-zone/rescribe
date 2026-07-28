#!/usr/bin/env bash
# Download the JATS 1.3 Archiving and Interchange Tag Set RELAX NG schema.
#
# This is the derivation input for `crates/formats/jats-fmt/registry/`. The
# schema is NOT vendored in this repository, so nothing in a normal build needs
# it — only regenerating or verifying the committed registry document does:
#
#   scripts/jats/download-spec.sh
#   cargo run -p jats-fmt --features registry-derive --bin derive-registry -- \
#       --schema-dir spec/jats-1.3-archiving-rng --check
#
# License: the JATS DTD Suite states in every module header that it is in the
# public domain, and that the suite must not be modified or redistributed in
# modified form. Files land under spec/, which is gitignored — nothing fetched
# here is committed.
set -euo pipefail

SPEC_DIR="$(cd "$(dirname "$0")/../.." && pwd)/spec/jats-1.3-archiving-rng"
BASE="https://jats.nlm.nih.gov/archiving/1.3/rng"
DRIVER="JATS-archivearticle1-3.rng"

mkdir -p "$SPEC_DIR"

fetch() {
    local name="$1"
    if [[ -f "$SPEC_DIR/$name" ]]; then
        echo "Already exists: $name"
        return
    fi
    echo "Downloading: $name"
    curl -fsSL -o "$SPEC_DIR/$name" "$BASE/$name"
}

fetch "$DRIVER"

# Module set comes from the schema's own <include> graph, so this script never
# hard-codes a module inventory that could drift. The graph is walked
# transitively: the tag set embeds XHTML tables and MathML by reference, and
# those modules declare <table>/<tr>/<td> and the MathML vocabulary.
declare -A seen=()
queue=("$DRIVER")
while [[ ${#queue[@]} -gt 0 ]]; do
    current="${queue[0]}"
    queue=("${queue[@]:1}")
    [[ -n "${seen[$current]:-}" ]] && continue
    seen[$current]=1
    fetch "$current"
    while read -r module; do
        [[ -n "$module" ]] && queue+=("$module")
    done < <(grep -o '<include href="[^"]*"' "$SPEC_DIR/$current" 2>/dev/null \
        | sed 's/.*href="//;s/"//' | sort -u)
done

echo
echo "Schema in $SPEC_DIR"
