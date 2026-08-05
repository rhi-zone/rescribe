<#
Share one target/ dir across the main checkout and every git worktree of
this repo (windows), by making each worktree's target/ a directory
junction (falling back to a symbolic link) pointing at the main checkout's
target/.

History: this used to write .cargo/config.local.toml, included by the
tracked .cargo/config.toml via its `include` key. That mechanism never
actually worked -- `include` requires the unstable `-Z config-include` flag
on the cargo actually installed here (1.91.1), confirmed by direct
reproduction (CARGO_LOG tracing showed config.local.toml was never
loaded). Every worktree silently built its own full local target/ instead,
which is what filled the machine's disk to ~100% with ~137GB of duplicated
build output across 11 concurrent worktrees. This script now does the
simple thing that needs zero cargo config awareness at all: it makes
target/ inside the worktree a real filesystem link to the shared dir, so
cargo just writes into it like any other directory.

Windows link choice: directory junctions (`New-Item -ItemType Junction`,
equivalent to `mklink /J`) do NOT require Administrator or Developer Mode
for a regular user to create -- unlike true NTFS symbolic links
(`New-Item -ItemType SymbolicLink` / `mklink /D`), which need the
SeCreateSymbolicLinkPrivilege (granted to Administrators by default, or to
any user if Developer Mode is enabled -- Windows 10 1703+, not on by
default). Junctions are therefore the zero-setup choice and are tried
first. Their one limitation: junctions only work within the same NTFS
volume/drive letter (unlike symlinks, which can cross drives). For the
normal case -- a repo and its worktrees all under one user's home volume --
this is not a real constraint. If the main checkout and this worktree are
on different drives, junction creation will fail; this script then tries a
true symbolic link as a fallback, which works cross-drive but needs admin
or Developer Mode. If both fail, it stops with an explicit error rather
than silently leaving target/ unlinked.

Run once per worktree, from inside that worktree (or let the post-checkout
hook in .githooks/ run it automatically -- confirmed empirically that
`git worktree add` fires post-checkout):
  scripts/setup-worktree-target.ps1

Safe to re-run (idempotent). If this worktree's target/ is already a
link (junction or symlink) to the right place, this is a no-op.

If target/ already exists in this worktree as a REAL directory (not a
link) -- e.g. a worktree that built its own local cache before this script
existed -- this script refuses to touch it and exits non-zero, rather than
silently deleting a build cache that might be the only copy of something.
Pass -AdoptExisting to explicitly merge that directory's contents into the
shared target dir (via robocopy /MOVE) and replace it with a link.
#>

param(
  [switch]$AdoptExisting
)

$ErrorActionPreference = "Stop"

$repoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $repoRoot

$gitCommonDir = (git rev-parse --git-common-dir).Trim()
# Resolve symlinks the same way .envrc's `realpath` does.
$commonDirAbs = (Resolve-Path -LiteralPath $gitCommonDir).Path
$mainRoot = Split-Path -Parent $commonDirAbs
$targetDir = Join-Path $mainRoot "target"
$worktreeRoot = (Get-Location).Path

# Clean up the dead marker file from the old (broken) include-based
# mechanism, if present. Nothing reads it anymore; leaving it around could
# mislead someone into thinking it's still load-bearing.
$staleConfig = ".cargo/config.local.toml"
if (Test-Path -LiteralPath $staleConfig) {
  Remove-Item -LiteralPath $staleConfig -Force
  Write-Host "Removed stale $staleConfig (old cargo include-based mechanism -- never actually worked, see this script's header)"
}

if (-not (Test-Path -LiteralPath $targetDir)) {
  New-Item -ItemType Directory -Path $targetDir | Out-Null
}

if ($worktreeRoot -eq $mainRoot) {
  Write-Host "This is the main checkout ($mainRoot) -- target/ is already the shared dir, nothing to link."
  exit 0
}

$localTarget = Join-Path $worktreeRoot "target"

if (Test-Path -LiteralPath $localTarget) {
  $item = Get-Item -LiteralPath $localTarget -Force
  $isLink = [bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
  if ($isLink) {
    Write-Host "target is already a link -- removing and relinking to $targetDir to make sure it points at the right place."
    Remove-Item -LiteralPath $localTarget -Force
  } elseif ($AdoptExisting) {
    Write-Host "Adopting existing target/ into $targetDir (merging via robocopy /MOVE -- files already in the shared dir are overwritten by this worktree's copy on conflict)..."
    robocopy $localTarget $targetDir /E /MOVE /NFL /NDL /NJH /NJS | Out-Null
    if (Test-Path -LiteralPath $localTarget) {
      Remove-Item -LiteralPath $localTarget -Recurse -Force
    }
    Write-Host "Merged local target/ into $targetDir."
  } else {
    Write-Error @"
target/ already exists in this worktree as a real directory ($localTarget).
Refusing to delete it automatically -- it may be a build cache from before
this mechanism existed (this is exactly the state the incident that
prompted this rewrite left worktrees in).

To adopt it (merge its contents into the shared dir at $targetDir, then
replace it with a link), re-run:
  scripts\setup-worktree-target.ps1 -AdoptExisting

Or move/remove it yourself first, then re-run this script with no args.
"@
    exit 1
  }
}

try {
  New-Item -ItemType Junction -Path $localTarget -Target $targetDir -ErrorAction Stop | Out-Null
  Write-Host "Linked (junction): $localTarget -> $targetDir"
} catch {
  Write-Host "Junction creation failed ($($_.Exception.Message)) -- likely because $mainRoot and $worktreeRoot are on different volumes (junctions are same-volume only). Trying a symbolic link instead (requires Administrator or Developer Mode)..."
  try {
    New-Item -ItemType SymbolicLink -Path $localTarget -Target $targetDir -ErrorAction Stop | Out-Null
    Write-Host "Linked (symlink): $localTarget -> $targetDir"
  } catch {
    Write-Error @"
Could not link target/ -> $targetDir as either a junction or a symbolic link.
Junctions need $mainRoot and $worktreeRoot on the same drive; symbolic links
need Administrator privileges or Developer Mode (Settings > System > For
developers) enabled. Enable one of those, or move this worktree onto the
same drive as $mainRoot, then re-run this script.
"@
    exit 1
  }
}
