<#
Give a fresh git worktree access to spec/ (windows).

spec/ holds large, gitignored reference material (ECMA-376 schema zips and
their extracted .rnc/.xsd files, ODF schemas, JATS RNG, etc. -- see
.gitignore's "/spec/*" rule and docs/ooxml/SPEC.md). `git worktree add`
only checks out tracked files, so a fresh worktree's spec/ has just the
small set of tracked exceptions (spec/*.yaml, spec/fixtures/,
spec/odf/README.md, spec/odf/odf-1.2.rnc) -- everything else (the
extracted ECMA-376 RNC/XSD trees that ooxml-wml/-sml/-dml/-pml's build.rs
and ooxml-codegen's tests read) is simply missing, and fails with a bare
"file not found" that reads like an unrelated flake rather than a
missing-setup-step.

spec/ is never written to by any build script or test (only read via
read_to_string/PathBuf joins), so it's safe to share the main checkout's
copy across every worktree via symlinks -- no per-worktree copy or
re-download needed. Because spec/ already has some real, tracked content
checked out in every worktree (the yaml/fixtures/odf files above), this
links in only the individual files/directories that are missing, rather
than replacing spec/ itself with one symlink.

Run once per worktree, from inside that worktree:
  scripts/setup-worktree-spec.ps1

Safe to re-run (idempotent): already-linked or already-tracked entries are
left alone. If the main checkout has no spec/ yet, see
scripts/ooxml/download-spec.sh and spec/odf/README.md to populate it there
first.

Creating a symlink on Windows normally requires Developer Mode enabled or
an elevated shell -- if this script can't create a link, it reports how
many it managed and how many it couldn't, with the manual `mklink` fallback.
#>

$ErrorActionPreference = "Stop"

$repoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $repoRoot

$gitCommonDir = (git rev-parse --git-common-dir).Trim()
$commonDirAbs = (Resolve-Path -LiteralPath $gitCommonDir).Path
$mainWorktree = Split-Path -Parent $commonDirAbs
$specSource = Join-Path $mainWorktree "spec"

if ((Resolve-Path -LiteralPath $repoRoot).Path -eq $mainWorktree) {
  Write-Host "This is the main checkout, not a linked worktree -- nothing to do."
  exit 0
}

if (-not (Test-Path -LiteralPath $specSource)) {
  Write-Host "Main checkout has no spec/ dir to link to ($specSource doesn't exist)."
  Write-Host "Populate it there first, then re-run this script:"
  Write-Host "  scripts/ooxml/download-spec.sh"
  Write-Host "  cd $specSource; Expand-Archive *.zip ."
  Write-Host "See spec/odf/README.md for the ODF schema (partly non-redistributable,"
  Write-Host "generated locally with trang) and docs/ooxml/SPEC.md for the expected layout."
  exit 1
}

if (-not (Test-Path -LiteralPath "spec")) {
  New-Item -ItemType Directory -Path "spec" | Out-Null
}

$linked = 0
$skipped = 0
$failed = 0

function Link-MissingEntries($srcDir, $dstDir) {
  if (-not (Test-Path -LiteralPath $dstDir)) {
    New-Item -ItemType Directory -Path $dstDir | Out-Null
  }
  Get-ChildItem -LiteralPath $srcDir -Force | ForEach-Object {
    $dst = Join-Path $dstDir $_.Name
    if (Test-Path -LiteralPath $dst) {
      $script:skipped++
      return
    }
    try {
      $itemType = if ($_.PSIsContainer) { "SymbolicLink" } else { "SymbolicLink" }
      New-Item -ItemType $itemType -Path $dst -Target $_.FullName -ErrorAction Stop | Out-Null
      $script:linked++
    } catch {
      Write-Host "Could not link $dst -> $($_.Exception.Message)"
      $script:failed++
    }
  }
}

# Top level: link whatever's missing straight from the main checkout's spec/.
Link-MissingEntries $specSource "spec"

# Second pass: recurse one level into dirs that already exist for real here
# (not as a symlink we just made), to catch partial dirs like spec/odf/.
Get-ChildItem -LiteralPath "spec" -Directory -Force | ForEach-Object {
  if ($_.LinkType -eq "SymbolicLink") { return }
  $srcDir = Join-Path $specSource $_.Name
  if (Test-Path -LiteralPath $srcDir) {
    Link-MissingEntries $srcDir $_.FullName
  }
}

Write-Host "Linked $linked missing entries under spec/ from $specSource ($skipped already present, $failed failed)."
if ($failed -gt 0) {
  Write-Host "Symlink creation failed for some entries -- this usually means Developer Mode is off and the shell isn't elevated."
  Write-Host "Re-run as Administrator, or enable Developer Mode, then re-run this script."
  exit 1
}
