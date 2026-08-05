#!/usr/bin/env bash
# Share one target/ dir across the main checkout and every git worktree of
# this repo (mac/linux), by making each worktree's target/ a symlink to the
# main checkout's target/.
#
# History: this used to write .cargo/config.local.toml, included by the
# tracked .cargo/config.toml via its `include` key. That mechanism never
# actually worked -- `include` requires the unstable `-Z config-include`
# flag on the cargo actually installed here (1.91.1), confirmed by direct
# reproduction (CARGO_LOG tracing showed config.local.toml was never
# loaded). Every worktree silently built its own full local target/
# instead, which is what filled the machine's disk to ~100% with ~137GB of
# duplicated build output across 11 concurrent worktrees. This script now
# does the simple thing that needs zero cargo config awareness at all: it
# makes target/ inside the worktree an actual symlink to the shared dir, so
# cargo just writes into it like any other directory.
#
# Run once per worktree, from inside that worktree (or let the
# post-checkout hook in .githooks/ run it automatically -- confirmed
# empirically that `git worktree add` fires post-checkout):
#   scripts/setup-worktree-target.sh
#
# Safe to re-run (idempotent). If this worktree's target/ is already a
# symlink to the right place, this is a no-op.
#
# If target/ already exists in this worktree as a REAL directory (not a
# symlink) -- e.g. a worktree that built its own local cache before this
# script existed -- this script refuses to touch it and exits non-zero,
# rather than silently deleting a build cache that might be the only copy
# of something. Pass --adopt-existing to explicitly merge that directory's
# contents into the shared target dir and replace it with a symlink.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

git_common_dir=$(git rev-parse --git-common-dir)
# Resolve symlinks the same way .envrc's `realpath` does, without depending
# on a `realpath` binary being present (not guaranteed on older macOS).
common_dir_abs=$(cd "$git_common_dir" && pwd -P)
main_root="$(dirname "$common_dir_abs")"
target_dir="$main_root/target"
worktree_root="$(pwd -P)"

# Clean up the dead marker file from the old (broken) include-based
# mechanism, if present. Nothing reads it anymore; leaving it around could
# mislead someone into thinking it's still load-bearing.
if [ -f ".cargo/config.local.toml" ]; then
  rm -f ".cargo/config.local.toml"
  echo "Removed stale .cargo/config.local.toml (old cargo include-based mechanism -- never actually worked, see this script's header)"
fi

mkdir -p "$target_dir"

if [ "$worktree_root" = "$main_root" ]; then
  echo "This is the main checkout ($main_root) -- target/ is already the shared dir, nothing to link."
  exit 0
fi

resolve_symlink() {
  if readlink -f / >/dev/null 2>&1; then
    readlink -f "$1"
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$1"
  else
    (cd "$1" && pwd -P)
  fi
}

if [ -L "target" ]; then
  current=$(resolve_symlink "target")
  if [ "$current" = "$target_dir" ]; then
    echo "Already linked: target -> $target_dir"
    exit 0
  fi
  echo "target is a symlink pointing elsewhere ($current) -- replacing with a link to $target_dir"
  rm "target"
  ln -s "$target_dir" "target"
  echo "Linked: target -> $target_dir"
  exit 0
fi

if [ -e "target" ]; then
  if [ "${1:-}" = "--adopt-existing" ]; then
    echo "Adopting existing target/ into $target_dir (merging -- files already in the shared dir are overwritten by this worktree's copy on conflict)..."
    cp -a target/. "$target_dir/"
    rm -rf "target"
    ln -s "$target_dir" "target"
    echo "Merged local target/ into $target_dir and linked: target -> $target_dir"
    exit 0
  fi
  echo "error: target/ already exists in this worktree as a real directory ($worktree_root/target)." >&2
  echo "Refusing to delete it automatically -- it may be a build cache from" >&2
  echo "before this mechanism existed (this is exactly the state the incident" >&2
  echo "that prompted this rewrite left worktrees in)." >&2
  echo "" >&2
  echo "To adopt it (merge its contents into the shared dir at $target_dir," >&2
  echo "then replace it with a symlink), re-run:" >&2
  echo "  $0 --adopt-existing" >&2
  echo "" >&2
  echo "Or move/remove it yourself first, then re-run this script with no args." >&2
  exit 1
fi

ln -s "$target_dir" "target"
echo "Linked: target -> $target_dir"
