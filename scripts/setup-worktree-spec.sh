#!/usr/bin/env bash
# Give a fresh git worktree access to spec/ (mac/linux).
#
# spec/ holds large, gitignored reference material (ECMA-376 schema zips and
# their extracted .rnc/.xsd files, ODF schemas, JATS RNG, etc. -- see
# .gitignore's "/spec/*" rule and docs/ooxml/SPEC.md). `git worktree add`
# only checks out tracked files, so a fresh worktree's spec/ has just the
# small set of tracked exceptions (spec/*.yaml, spec/fixtures/,
# spec/odf/README.md, spec/odf/odf-1.2.rnc) -- everything else (the
# extracted ECMA-376 RNC/XSD trees that ooxml-wml/-sml/-dml/-pml's build.rs
# and ooxml-codegen's tests read) is simply missing, and fails with a bare
# "file not found" that reads like an unrelated flake rather than a
# missing-setup-step.
#
# spec/ is never written to by any build script or test (only read via
# read_to_string/PathBuf joins), so it's safe to share the main checkout's
# copy across every worktree via symlinks -- no per-worktree copy or
# re-download needed. Because spec/ already has some real, tracked content
# checked out in every worktree (the yaml/fixtures/odf files above), this
# links in only the individual files/directories that are missing, rather
# than replacing spec/ itself with one symlink.
#
# Run once per worktree, from inside that worktree:
#   scripts/setup-worktree-spec.sh
#
# Safe to re-run (idempotent): already-linked or already-tracked entries are
# left alone. If the main checkout has no spec/ yet, see
# scripts/ooxml/download-spec.sh and spec/odf/README.md to populate it there
# first.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

git_common_dir=$(git rev-parse --git-common-dir)
common_dir_abs=$(cd "$git_common_dir" && pwd -P)
main_worktree="$(dirname "$common_dir_abs")"
spec_source="$main_worktree/spec"

if [[ "$(pwd -P)" == "$main_worktree" ]]; then
  echo "This is the main checkout, not a linked worktree -- nothing to do."
  exit 0
fi

if [[ ! -d "$spec_source" ]]; then
  cat <<EOF
Main checkout has no spec/ dir to link to ($spec_source doesn't exist).
Populate it there first, then re-run this script:
  scripts/ooxml/download-spec.sh
  cd $spec_source && unzip -o '*.zip'
See spec/odf/README.md for the ODF schema (partly non-redistributable,
generated locally with trang) and docs/ooxml/SPEC.md for the expected layout.
EOF
  exit 1
fi

mkdir -p spec
linked=0
skipped=0
# Walk the main checkout's spec/ tree and symlink in whatever this worktree
# doesn't already have (as a real file/dir or an existing symlink). Recurses
# into directories that exist on both sides (e.g. spec/odf/, which has a mix
# of tracked and gitignored files) so partial dirs get filled in too.
while IFS= read -r -d '' entry; do
  rel="${entry#./}"
  src="$spec_source/$rel"
  dst="spec/$rel"
  if [[ -e "$dst" || -L "$dst" ]]; then
    skipped=$((skipped + 1))
    continue
  fi
  ln -s "$src" "$dst"
  linked=$((linked + 1))
done < <(cd "$spec_source" && find . -mindepth 1 -maxdepth 1 -print0)

# Second pass: for directories that already exist for real (not symlinked)
# in this worktree, recurse one level to catch partial dirs like spec/odf/.
for d in spec/*/; do
  [[ -d "$d" && ! -L "${d%/}" ]] || continue
  name=$(basename "$d")
  src_dir="$spec_source/$name"
  [[ -d "$src_dir" ]] || continue
  while IFS= read -r -d '' entry; do
    rel="${entry#./}"
    src="$src_dir/$rel"
    dst="$d$rel"
    if [[ -e "$dst" || -L "$dst" ]]; then
      skipped=$((skipped + 1))
      continue
    fi
    ln -s "$src" "$dst"
    linked=$((linked + 1))
  done < <(cd "$src_dir" && find . -mindepth 1 -maxdepth 1 -print0)
done

echo "Linked $linked missing entr$([ "$linked" = 1 ] && echo y || echo ies) under spec/ from $spec_source ($skipped already present)."
