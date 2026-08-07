//! Optional, feature-gated resolution of `\documentclass`/`\usepackage`
//! (and `\LoadClass`/`\RequirePackage`) targets against **real** `.cls`/
//! `.sty` content, so uses of the commands those files define resolve the
//! same way an in-document `\newcommand` does — instead of every such
//! command falling through to the raw-preserve fallback described in
//! [`crate::parse`]'s module docs (which remains the *only* behavior when
//! this feature is off, or when resolution fails for a given package).
//!
//! # Why this is not vendoring
//!
//! No package source is bundled into this crate's repository or published
//! artifact. Real `.cls`/`.sty` content, when available at all, is
//! obtained entirely at the caller's runtime, from the caller's own
//! machine or network, via two independently-optional channels
//! ([`LocalFsSource`], [`NetworkSource`]) that degrade gracefully to "not
//! found" — never a hard error — when unavailable. What crosses into the
//! *disk cache* this module also provides is not that raw source at all:
//! it is the **resolved definition table** ([`Resolved`]) extracted from
//! it by [`crate::parse::extract_definitions`] — the same shape a
//! `\newcommand`/`\def` in the current document would produce, not
//! redistributable prose/code. Caching that derived table locally is the
//! same category of thing as cargo's registry cache or npm's package
//! cache: content the user's own machine already fetched for its own use,
//! kept nearby so the next run doesn't refetch it. See `TODO.md` for the
//! design history superseding the earlier vendoring approach.
//!
//! # Three channels, one cache
//!
//! ```text
//! PackageResolver::resolve(request)
//!     -> cache hit?              (CacheStore::get)                  -> Resolved
//!     -> local TeX install?      (LocalFsSource::locate)  -> bytes  -> extract -> cache put -> Resolved
//!     -> network (CTAN/mirror)?  (NetworkSource::locate)  -> bytes  -> extract -> cache put -> Resolved
//!     -> none available/found                                      -> Resolved::empty() (raw-preserve fallback)
//! ```
//!
//! Each channel is independently optional (a [`PackageResolver`] can be
//! built with either, both, or neither — with neither, every request
//! degrades straight to raw-preserve, same as the feature being off
//! except for the (cheap) cache lookup). Nothing here ever panics or
//! propagates an I/O error out of [`PackageResolver::resolve`]: a failed
//! probe, a failed fetch, and "genuinely not found" are all the same
//! outcome from the parser's point of view.
//!
//! # Cache key: a hash chain, not a flat key
//!
//! A package's effective resolved definitions can depend on more than its
//! own name: `\usepackage[options]{name}` options change what a package
//! defines, and TeX packages routinely branch on *what already loaded*
//! (`\@ifpackageloaded`, babel language options, hyperref driver
//! detection, ...). A flat `name` (or even `name+options`) key would
//! collide two documents that load the same package with the same
//! options but a different preceding load sequence, silently serving one
//! document's resolution to the other.
//!
//! The key actually used is a running hash **chain** over the document's
//! package-loading history so far:
//!
//! ```text
//! chain_0 = SHA256("latex-fmt/package-resolve/v1")           // fixed seed
//! chain_i = SHA256(chain_{i-1} || kind_byte || 0x00
//!                   || name_bytes || 0x00
//!                   || options.join(",")_bytes)
//! ```
//!
//! `chain_i` (hex-encoded) is both the cache lookup key *and* the cache
//! file's name for the `i`-th package-loading command encountered, in
//! document order. This is computable *before* any fetch happens (it only
//! needs the request's own identity plus the chain so far), which is the
//! point: a cache hit never requires touching the filesystem probe or the
//! network. The raw fetched bytes' SHA-256 is stored *inside* the cache
//! entry (`CachedResolved::source_sha256`) as a provenance/debugging
//! record, not as part of the key — it can't be, since the key must exist
//! before the bytes are fetched.
//!
//! Known limitation, documented rather than silently assumed away: this
//! models "prior load sequence" only as the flat list of package-loading
//! commands `parse_with_package_resolution`'s pre-scan finds in document
//! order. It does not model conditional loading (`\IfFileExists`,
//! `\@ifpackageloaded` branches actually taken) or options resolved via
//! `\PassOptionsToPackage`. A document that loads packages conditionally
//! may get a chain key that doesn't reflect the runtime-true history;
//! this only ever affects cache *hit rate*, never correctness, since a
//! wrong/stale chain key just produces a cache miss (re-resolved fresh)
//! rather than serving wrong data — the key is derived from static
//! syntactic occurrence, not claimed to model full TeX conditional
//! execution.
//!
//! # Cache directory layout
//!
//! ```text
//! {dirs::cache_dir()}/rescribe/latex-fmt/packages/v1/{chain_key_hex}.rkyv
//! ```
//!
//! Entries are `rkyv` bytes (see [`CacheStore`]'s doc comment for why
//! `rkyv` rather than `serde`+bincode/postcard). `v1` is a schema-version
//! segment: a future incompatible change to [`CachedResolved`]'s archived
//! shape bumps it, which orphans (never corrupts) old entries rather than
//! requiring a migration — `rkyv`'s `bytecheck` validation (on by
//! default, used by [`CacheStore::get`]) additionally guards against
//! misreading a stale/foreign-shaped file that slipped past that
//! versioning, degrading it to a cache miss rather than undefined
//! behavior. The directory is created on demand; a failure to create or
//! write it is treated as "cache unavailable" (degrades to always-miss),
//! never a hard error.

use crate::ast::Diagnostic;
use crate::parse::{self, MacroInfo};
use rkyv::rancor::Error as RkyvError;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// `.cls` (document class, `\documentclass`/`\LoadClass`) vs `.sty`
/// (style/package, `\usepackage`/`\RequirePackage`) — the two kinds of
/// file the LaTeX kernel's package-loading commands can name. Not part of
/// the persisted cache shape (see [`CachedResolved`]) — only
/// [`PackageRequest`] carries it, to compute the hash-chain key — so it
/// needs no `rkyv` derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageKind {
    Class,
    Style,
}

impl PackageKind {
    fn extension(self) -> &'static str {
        match self {
            PackageKind::Class => "cls",
            PackageKind::Style => "sty",
        }
    }

    fn byte_tag(self) -> u8 {
        match self {
            PackageKind::Class => b'C',
            PackageKind::Style => b'S',
        }
    }
}

/// One `\documentclass`/`\usepackage`/`\LoadClass`/`\RequirePackage`
/// target: a single package/class name plus whatever `[options]` were
/// given (order preserved — it is part of the cache key, see module
/// docs).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageRequest {
    pub name: String,
    pub kind: PackageKind,
    pub options: Vec<String>,
}

/// The persisted shape of a resolved package's definition table — exactly
/// what [`CacheStore`] reads/writes as `rkyv` bytes. Deliberately excludes
/// [`Diagnostic`] (`rescribe_format_api::Diagnostic` has no `rkyv` impl,
/// and diagnostics are informational about *how* an extraction went, not
/// needed for a cache hit to be correct/useful) — see [`Resolved`], the
/// public-facing type that adds them back as an empty, runtime-only
/// field.
#[derive(Debug, Clone, Default, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct CachedResolved {
    pub commands: HashMap<String, SerializedMacroInfo>,
    pub environments: HashMap<String, SerializedMacroInfo>,
    /// SHA-256 (hex) of the raw bytes this was extracted from. `None`
    /// when resolution found nothing (empty/fallback result).
    pub source_sha256: Option<String>,
    pub resolved_via: Option<ResolvedVia>,
    /// Unix timestamp (seconds) this entry was produced. Informational
    /// only — nothing here expires entries by age; content-addressing
    /// (a changed chain key) is what invalidates them.
    pub resolved_at_unix: Option<u64>,
}

/// The result of resolving one [`PackageRequest`]: [`CachedResolved`] plus
/// fidelity-relevant diagnostics from the extraction (e.g. its own
/// unresolved-command notes), re-surfaced to the caller rather than
/// silently swallowed. A cache-hit `Resolved` always has an empty
/// `diagnostics` (they are not persisted — see [`CachedResolved`]); a
/// fresh extraction has whatever [`parse::extract_definitions`] produced.
#[derive(Debug, Clone, Default)]
pub struct Resolved {
    pub cached: CachedResolved,
    pub diagnostics: Vec<Diagnostic>,
}

impl Resolved {
    pub fn is_empty(&self) -> bool {
        self.cached.commands.is_empty() && self.cached.environments.is_empty()
    }

    pub fn commands(&self) -> &HashMap<String, SerializedMacroInfo> {
        &self.cached.commands
    }

    pub fn environments(&self) -> &HashMap<String, SerializedMacroInfo> {
        &self.cached.environments
    }

    pub fn source_sha256(&self) -> Option<&str> {
        self.cached.source_sha256.as_deref()
    }

    pub fn resolved_via(&self) -> Option<ResolvedVia> {
        self.cached.resolved_via
    }
}

/// Which channel produced a [`Resolved`] entry — kept for
/// provenance/debugging, not consulted for correctness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, RkyvSerialize, RkyvDeserialize)]
pub enum ResolvedVia {
    LocalFs,
    Network,
}

/// An `rkyv`-archivable mirror of `crate::parse::MacroInfo` (which
/// deliberately carries no `rkyv` dependency of its own — only the
/// `package-resolve` feature needs archiving, and `parse.rs` is core,
/// dependency-free code shared by every feature).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct SerializedMacroInfo {
    pub mandatory: u8,
    pub has_optional_leading: bool,
}

impl From<MacroInfo> for SerializedMacroInfo {
    fn from(m: MacroInfo) -> Self {
        SerializedMacroInfo {
            mandatory: m.mandatory,
            has_optional_leading: m.has_optional_leading,
        }
    }
}

impl From<SerializedMacroInfo> for MacroInfo {
    fn from(m: SerializedMacroInfo) -> Self {
        MacroInfo {
            mandatory: m.mandatory,
            has_optional_leading: m.has_optional_leading,
        }
    }
}

fn to_macro_map(m: &HashMap<String, SerializedMacroInfo>) -> HashMap<String, MacroInfo> {
    m.iter().map(|(k, v)| (k.clone(), (*v).into())).collect()
}

fn from_macro_map(m: HashMap<String, MacroInfo>) -> HashMap<String, SerializedMacroInfo> {
    m.into_iter().map(|(k, v)| (k, v.into())).collect()
}

// ---- channel 1: local filesystem (real TeX install) -----------------------

/// Best-effort local-filesystem probe for a real TeX installation on the
/// machine running the parser. Two strategies, tried in order:
///
/// 1. `kpsewhich <name>.<ext>` (TeX Live/MiKTeX's own path-search tool) —
///    the correct answer whenever it's on `PATH`, since it already knows
///    every installed package's true location (including ones not under
///    any of the standard paths this module also probes).
/// 2. A small set of standard TeX install path patterns, checked directly,
///    for the case `kpsewhich` isn't installed/on `PATH` but a TeX tree
///    still exists at a conventional location. This is **not** a kpathsea
///    reimplementation — it is a narrow, documented fallback covering the
///    common case (a package installed directly under `texmf*/tex/latex/
///    <name>/`), not arbitrary kpathsea search-path configuration.
#[derive(Default)]
pub struct LocalFsSource {
    /// Extra roots to probe (in addition to the built-in standard-location
    /// list), e.g. for tests or nonstandard installs. Checked before
    /// `kpsewhich` so a caller-supplied override always wins.
    pub extra_roots: Vec<PathBuf>,
}

impl LocalFsSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn locate(&self, name: &str, kind: PackageKind) -> Option<Vec<u8>> {
        let filename = format!("{name}.{}", kind.extension());

        for root in &self.extra_roots {
            let candidate = root.join(&filename);
            if let Ok(bytes) = std::fs::read(&candidate) {
                return Some(bytes);
            }
        }

        if let Some(bytes) = self.via_kpsewhich(&filename) {
            return Some(bytes);
        }

        self.via_standard_paths(name, &filename)
    }

    fn via_kpsewhich(&self, filename: &str) -> Option<Vec<u8>> {
        let output = std::process::Command::new("kpsewhich")
            .arg(filename)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path_str = String::from_utf8(output.stdout).ok()?;
        let path = path_str.trim();
        if path.is_empty() {
            return None;
        }
        std::fs::read(path).ok()
    }

    fn via_standard_paths(&self, name: &str, filename: &str) -> Option<Vec<u8>> {
        let mut roots: Vec<PathBuf> = Vec::new();
        for var in ["TEXMFHOME", "TEXMFLOCAL", "TEXMFDIST"] {
            if let Ok(v) = std::env::var(var) {
                roots.push(PathBuf::from(v));
            }
        }
        if let Some(home) = dirs::home_dir() {
            roots.push(home.join("texmf"));
        }
        // Common Linux/macOS TeX Live install roots.
        for glob_root in ["/usr/share/texlive/texmf-dist", "/usr/local/texlive"] {
            roots.push(PathBuf::from(glob_root));
        }

        for root in roots {
            let candidate = root.join("tex").join("latex").join(name).join(filename);
            if let Ok(bytes) = std::fs::read(&candidate) {
                return Some(bytes);
            }
        }
        None
    }
}

// ---- channel 2: network (CTAN / mirror) ------------------------------------

/// Best-effort network fetch against CTAN or a configured mirror.
///
/// CTAN has no single uniform "give me the raw `.sty` for `<name>`" URL
/// for every package (packages ship in varied layouts — a bare file, a
/// `.dtx`/`.ins` pair requiring `tex`-time extraction, a `.tds.zip`, ...).
/// This channel therefore only covers the common case: packages published
/// as a directly-fetchable `<name>.sty`/`<name>.cls` under CTAN's
/// `macros/latex/contrib/<name>/` convention. It is a heuristic, not a
/// general CTAN client — documented here rather than silently assumed
/// complete. Packages that don't match are simply not found over this
/// channel (graceful degradation, same as any other miss).
pub struct NetworkSource {
    /// Base URL, e.g. `https://mirrors.ctan.org` (default) or a chosen
    /// mirror / local CTAN proxy.
    pub base_url: String,
    agent: ureq::Agent,
}

impl Default for NetworkSource {
    fn default() -> Self {
        NetworkSource::new("https://mirrors.ctan.org")
    }
}

impl NetworkSource {
    pub fn new(base_url: impl Into<String>) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(10)))
            .build();
        NetworkSource {
            base_url: base_url.into(),
            agent: ureq::Agent::new_with_config(config),
        }
    }

    pub fn locate(&self, name: &str, kind: PackageKind) -> Option<Vec<u8>> {
        let ext = kind.extension();
        // A small set of conventional CTAN layouts, tried in order; first
        // 200 response wins. Best-effort by design (see struct docs).
        let candidates = [
            format!("{}/macros/latex/contrib/{name}/{name}.{ext}", self.base_url),
            format!(
                "{}/install/macros/latex/contrib/{name}/{name}.{ext}",
                self.base_url
            ),
            format!("{}/macros/latex/base/{name}.{ext}", self.base_url),
        ];
        for url in candidates {
            if let Some(bytes) = self.fetch(&url) {
                return Some(bytes);
            }
        }
        None
    }

    fn fetch(&self, url: &str) -> Option<Vec<u8>> {
        let mut response = self
            .agent
            .get(url)
            .header(
                "User-Agent",
                "latex-fmt/package-resolve (+https://rhi.zone)",
            )
            .call()
            .ok()?;
        if response.status() != 200 {
            return None;
        }
        response.body_mut().read_to_vec().ok()
    }
}

// ---- content-addressed local disk cache ------------------------------------

/// Local, content-addressed cache of [`CachedResolved`] entries, stored as
/// `rkyv` bytes (not `serde`+bincode/postcard — see the `package-resolve`
/// feature doc comment in `Cargo.toml` for why). Never committed to any
/// repository, never shipped as part of a published crate — it lives
/// under the OS cache directory ([`dirs::cache_dir`]), same as any other
/// local tool cache. See module docs for the directory layout and key
/// scheme.
///
/// Reads go through [`memmap2::Mmap`] + `rkyv::access` (validated,
/// zero-copy borrow straight out of the OS page cache — no text parsing,
/// no intermediate value tree) before the one unavoidable step,
/// `rkyv::deserialize` into an owned [`CachedResolved`], which this
/// module's public API needs since callers merge results into their own
/// owned maps. That last step is a structural copy of already-typed data
/// (`String`s and small `Copy` structs), categorically cheaper than
/// parsing JSON text or decoding a bincode/postcard byte stream — the
/// validation/page-in cost, which dominates on the realistic "cold cache
/// file, fresh process" case this cache is designed for, is the part
/// that's genuinely zero-copy.
pub struct CacheStore {
    dir: Option<PathBuf>,
}

impl Default for CacheStore {
    /// Uses `dirs::cache_dir()/rescribe/latex-fmt/packages/v1`. If the OS
    /// cache directory can't be determined, the cache is silently
    /// disabled (always-miss, never a hard error) rather than falling
    /// back to some other location this module wasn't asked to use.
    fn default() -> Self {
        let dir = dirs::cache_dir().map(|d| {
            d.join("rescribe")
                .join("latex-fmt")
                .join("packages")
                .join("v1")
        });
        CacheStore { dir }
    }
}

impl CacheStore {
    /// Cache rooted at an explicit directory instead of the OS cache dir
    /// (used by tests, and by callers who want an isolated/ephemeral
    /// cache).
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        CacheStore {
            dir: Some(dir.into()),
        }
    }

    /// Disabled cache: every lookup misses, every store is a no-op.
    pub fn disabled() -> Self {
        CacheStore { dir: None }
    }

    fn entry_path(&self, chain_key: &str) -> Option<PathBuf> {
        self.dir
            .as_ref()
            .map(|d| d.join(format!("{chain_key}.rkyv")))
    }

    /// Cache lookup: `mmap` the entry file (if present), validate +
    /// zero-copy-access it as `ArchivedCachedResolved`, then deserialize
    /// into an owned [`CachedResolved`] (see struct docs for why the last
    /// step is unavoidable, and why it's still cheap). Any failure at any
    /// step — file missing, `mmap` failure, `rkyv` validation failure
    /// (e.g. a corrupted or truncated entry, or a stale entry from an
    /// incompatible schema version that slipped past the `v1` directory
    /// segment) — is treated as a plain cache miss, never a panic or a
    /// propagated error.
    pub fn get(&self, chain_key: &str) -> Option<CachedResolved> {
        let path = self.entry_path(chain_key)?;
        let file = std::fs::File::open(path).ok()?;
        // SAFETY: this cache directory holds only content this module
        // itself wrote via `put` below, in a process-private location;
        // nothing else is expected to mutate an entry file concurrently
        // with a `get`, which is the property `Mmap::map` requires for
        // soundness. `rkyv::access`'s `bytecheck` validation (the default
        // feature, enabled here) additionally guards against a corrupted
        // or truncated file being read as valid data, degrading that case
        // to a cache miss rather than undefined behavior.
        let mmap = unsafe { memmap2::Mmap::map(&file) }.ok()?;
        let archived = rkyv::access::<ArchivedCachedResolved, RkyvError>(&mmap[..]).ok()?;
        rkyv::deserialize::<CachedResolved, RkyvError>(archived).ok()
    }

    pub fn put(&self, chain_key: &str, resolved: &CachedResolved) {
        let Some(path) = self.entry_path(chain_key) else {
            return;
        };
        let Some(parent) = path.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        if let Ok(bytes) = rkyv::to_bytes::<RkyvError>(resolved) {
            let _ = std::fs::write(path, &bytes[..]);
        }
    }
}

// ---- hash-chain key scheme --------------------------------------------------

const CHAIN_SEED: &str = "latex-fmt/package-resolve/v1";

/// Running hash-chain state over a document's package-loading history so
/// far. See module docs for the scheme and rationale.
#[derive(Debug, Clone)]
pub struct ChainState {
    current: [u8; 32],
}

impl Default for ChainState {
    fn default() -> Self {
        let mut hasher = Sha256::new();
        hasher.update(CHAIN_SEED.as_bytes());
        ChainState {
            current: hasher.finalize().into(),
        }
    }
}

impl ChainState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the chain past `req` and returns the resulting key (hex),
    /// which is both this request's cache lookup key and the new chain
    /// state for whatever package-loading command comes next.
    pub fn advance(&mut self, req: &PackageRequest) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.current);
        hasher.update([req.kind.byte_tag(), 0]);
        hasher.update(req.name.as_bytes());
        hasher.update([0]);
        hasher.update(req.options.join(",").as_bytes());
        let digest: [u8; 32] = hasher.finalize().into();
        self.current = digest;
        hex_encode(&digest)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

// ---- the resolver: wires the three channels together -----------------------

/// Resolves [`PackageRequest`]s against the cache, then local filesystem,
/// then network — each step optional, each falling through gracefully to
/// [`Resolved::default`] (empty: the existing raw-preserve fallback) if
/// unavailable or unsuccessful. Construct via [`PackageResolver::builder`]
/// to opt into only the channels wanted (e.g. cache + local-fs only, no
/// network).
pub struct PackageResolver {
    cache: CacheStore,
    local: Option<LocalFsSource>,
    network: Option<NetworkSource>,
}

/// Builder for [`PackageResolver`]; every channel starts disabled so an
/// empty `PackageResolverBuilder::new().build()` is inert (always
/// raw-preserve fallback), matching the feature-off default behavior
/// exactly except for a no-op cache lookup.
pub struct PackageResolverBuilder {
    cache: CacheStore,
    local: Option<LocalFsSource>,
    network: Option<NetworkSource>,
}

impl PackageResolver {
    pub fn builder() -> PackageResolverBuilder {
        PackageResolverBuilder {
            cache: CacheStore::disabled(),
            local: None,
            network: None,
        }
    }

    /// Convenience: cache (OS default location) + local-fs enabled,
    /// network disabled — the reasonable default for most callers (no
    /// silent network egress).
    pub fn local_only() -> Self {
        PackageResolver::builder()
            .with_cache(CacheStore::default())
            .with_local_fs(LocalFsSource::default())
            .build()
    }

    /// Convenience: all three channels enabled with default
    /// configuration (OS cache dir, `kpsewhich`/standard-path probing,
    /// `mirrors.ctan.org`).
    pub fn full() -> Self {
        PackageResolver::builder()
            .with_cache(CacheStore::default())
            .with_local_fs(LocalFsSource::default())
            .with_network(NetworkSource::default())
            .build()
    }

    /// Resolves one request, given the chain key already computed for it
    /// (by a [`ChainState`] the caller advances in document order — kept
    /// external to this method so multiple requests in one document share
    /// one chain).
    pub fn resolve(&self, req: &PackageRequest, chain_key: &str) -> Resolved {
        if let Some(cached) = self.cache.get(chain_key) {
            return Resolved {
                cached,
                diagnostics: Vec::new(),
            };
        }

        if let Some(local) = &self.local
            && let Some(bytes) = local.locate(&req.name, req.kind)
        {
            return self.extract_and_store(chain_key, &bytes, ResolvedVia::LocalFs);
        }

        if let Some(network) = &self.network
            && let Some(bytes) = network.locate(&req.name, req.kind)
        {
            return self.extract_and_store(chain_key, &bytes, ResolvedVia::Network);
        }

        Resolved::default()
    }

    fn extract_and_store(&self, chain_key: &str, bytes: &[u8], via: ResolvedVia) -> Resolved {
        let text = String::from_utf8_lossy(bytes);
        let (commands, environments, diagnostics) = parse::extract_definitions(&text);
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let source_sha256 = hex_encode(&hasher.finalize());
        let resolved_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
        let cached = CachedResolved {
            commands: from_macro_map(commands),
            environments: from_macro_map(environments),
            source_sha256: Some(source_sha256),
            resolved_via: Some(via),
            resolved_at_unix,
        };
        self.cache.put(chain_key, &cached);
        Resolved {
            cached,
            diagnostics,
        }
    }
}

impl PackageResolverBuilder {
    pub fn with_cache(mut self, cache: CacheStore) -> Self {
        self.cache = cache;
        self
    }

    pub fn with_local_fs(mut self, local: LocalFsSource) -> Self {
        self.local = Some(local);
        self
    }

    pub fn with_network(mut self, network: NetworkSource) -> Self {
        self.network = Some(network);
        self
    }

    pub fn build(self) -> PackageResolver {
        PackageResolver {
            cache: self.cache,
            local: self.local,
            network: self.network,
        }
    }
}

// ---- document pre-scan: find package-loading commands ----------------------

/// Kernel-level package-loading command names. Recognizing these four is
/// knowledge of the LaTeX *package-loading mechanism itself* — the same
/// category `crate::parse::COMMAND_DEFINERS` already carries for
/// `\newcommand`/`\def`, never an assertion about what an arbitrary
/// document-level command like `\section` means.
const PACKAGE_LOADERS: &[(&str, PackageKind)] = &[
    ("documentclass", PackageKind::Class),
    ("LoadClass", PackageKind::Class),
    ("usepackage", PackageKind::Style),
    ("RequirePackage", PackageKind::Style),
];

/// Scans `input`'s token stream (via the same tokenizer `parse()` uses,
/// not a reimplementation) for `\documentclass`/`\usepackage`/
/// `\LoadClass`/`\RequirePackage` invocations, in document order,
/// expanding `\usepackage{a,b,c}`'s comma-separated name list into one
/// [`PackageRequest`] per name (all sharing that invocation's options).
/// Malformed/incomplete invocations (missing the mandatory name group)
/// are simply skipped — not an error, since the normal parser will still
/// raw-preserve them.
fn scan_package_loaders(input: &str) -> Vec<PackageRequest> {
    use crate::tokenize::{Lexer, Tok};

    let mut out = Vec::new();
    let mut lex = Lexer::new(input);
    while let Some((tok, _span)) = lex.next_token() {
        let Tok::Cs(name) = tok else { continue };
        let Some((_, kind)) = PACKAGE_LOADERS.iter().find(|(n, _)| *n == name) else {
            continue;
        };

        // Optional `[options]` and mandatory `{name(s)}` are scanned
        // directly off the raw remaining bytes (bracket/brace-depth
        // aware), the same pattern `parse.rs`'s `try_scan_optional_arg`
        // uses for the same reason: `[`/`{` are not distinct catcode
        // classes the tokenizer special-cases ahead of time here. Same
        // documented limitation applies (unbalanced brackets/braces
        // nested inside are not specially handled).
        let options = scan_bracket_options(&mut lex, input).unwrap_or_default();

        let Some(names) = scan_brace_names(&mut lex, input) else {
            continue;
        };
        for n in names {
            out.push(PackageRequest {
                name: n,
                kind: *kind,
                options: options.clone(),
            });
        }
    }
    out
}

/// If the lexer is currently sitting at a `[`, consumes through the
/// matching `]` (bracket-depth aware) and returns the comma-split,
/// trimmed option list; otherwise leaves the lexer position untouched and
/// returns `None`.
fn scan_bracket_options(lex: &mut crate::tokenize::Lexer<'_>, input: &str) -> Option<Vec<String>> {
    let start = lex.pos();
    let bytes = input.as_bytes();
    if bytes.get(start) != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = start;
    loop {
        match bytes.get(i) {
            None => return None, // unterminated: leave options empty, caller still tries {name}
            Some(b'[') => {
                depth += 1;
                i += 1;
            }
            Some(b']') => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    break;
                }
            }
            Some(_) => i += 1,
        }
    }
    let inner = &input[start + 1..i - 1];
    lex.seek(i);
    Some(
        inner
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

/// Scans a mandatory `{name}` (or `{name1,name2}`) group directly off the
/// raw remaining bytes (brace-depth aware, so a nested `{...}` inside the
/// name group doesn't terminate early — not that one is legal LaTeX for a
/// class/package name, but this must never panic or desync on
/// adversarial/malformed input either). Returns the comma-split, trimmed
/// name list, or `None` if the lexer isn't sitting at a `{`.
fn scan_brace_names(lex: &mut crate::tokenize::Lexer<'_>, input: &str) -> Option<Vec<String>> {
    let start = lex.pos();
    let bytes = input.as_bytes();
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = start;
    loop {
        match bytes.get(i) {
            None => return None, // unterminated: no name found
            Some(b'{') => {
                depth += 1;
                i += 1;
            }
            Some(b'}') => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    break;
                }
            }
            Some(_) => i += 1,
        }
    }
    let inner = &input[start + 1..i - 1];
    lex.seek(i);
    Some(
        inner
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

// ---- top-level entry point --------------------------------------------------

/// Like [`crate::parse::parse`], but every `\documentclass`/`\usepackage`/
/// `\LoadClass`/`\RequirePackage` target found in `input` is resolved
/// through `resolver` first (cache, then local filesystem, then network —
/// see [`PackageResolver::resolve`]), and the resulting definitions are
/// seeded into the document's global scope before the normal in-document
/// `\def`/`\newcommand` scan runs. A name resolved this way behaves
/// exactly like an in-document `\newcommand` of the same name for the
/// rest of the document (including being shadowed/redefined locally, same
/// as any other definition — see `crate::parse`'s scope-stack docs).
///
/// `\documentclass{...}`/`\usepackage{...}` themselves are *not* given any
/// special structural meaning here — they still raw-preserve via the
/// normal unresolved-command path (correct: they have no body-visible
/// output). Only the definitions the referenced content itself contains
/// are consulted.
///
/// Packages that fail to resolve on every channel contribute nothing
/// (silently, since the normal per-command `latex::unresolved-command`
/// diagnostic already fires for every use their absence leaves
/// unresolved) — never delete or fall back the document to plain
/// [`crate::parse::parse`] partway through.
pub fn parse_with_package_resolution(
    input: &str,
    resolver: &PackageResolver,
) -> (crate::ast::LatexDoc, Vec<Diagnostic>) {
    let requests = scan_package_loaders(input);

    let mut chain = ChainState::new();
    let mut seed_commands: HashMap<String, MacroInfo> = HashMap::new();
    let mut seed_environments: HashMap<String, MacroInfo> = HashMap::new();
    let mut extra_diags = Vec::new();

    for req in &requests {
        let key = chain.advance(req);
        let resolved = resolver.resolve(req, &key);
        // Later packages' definitions win on name collision, matching
        // real LaTeX: a later \usepackage that redefines a name loaded by
        // an earlier one takes effect for the rest of the document.
        seed_commands.extend(to_macro_map(resolved.commands()));
        seed_environments.extend(to_macro_map(resolved.environments()));
        extra_diags.extend(resolved.diagnostics);
    }

    let (doc, mut diags) = parse::parse_seeded(input, seed_commands, seed_environments);
    diags.extend(extra_diags);
    (doc, diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(name: &str, kind: PackageKind, options: &[&str]) -> PackageRequest {
        PackageRequest {
            name: name.to_string(),
            kind,
            options: options.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ---- chain key ---------------------------------------------------------

    #[test]
    fn chain_key_is_deterministic() {
        let mut a = ChainState::new();
        let mut b = ChainState::new();
        let k1 = a.advance(&req("amsmath", PackageKind::Style, &[]));
        let k2 = b.advance(&req("amsmath", PackageKind::Style, &[]));
        assert_eq!(k1, k2);
    }

    #[test]
    fn chain_key_depends_on_options() {
        let mut a = ChainState::new();
        let mut b = ChainState::new();
        let k1 = a.advance(&req("babel", PackageKind::Style, &["english"]));
        let k2 = b.advance(&req("babel", PackageKind::Style, &["german"]));
        assert_ne!(k1, k2);
    }

    #[test]
    fn chain_key_depends_on_kind() {
        let mut a = ChainState::new();
        let mut b = ChainState::new();
        let k1 = a.advance(&req("foo", PackageKind::Class, &[]));
        let k2 = b.advance(&req("foo", PackageKind::Style, &[]));
        assert_ne!(k1, k2);
    }

    #[test]
    fn chain_key_depends_on_prior_load_sequence() {
        // Same second request ("bar"), different first request -> the
        // *second* request's key must differ, since it's chained off a
        // different prior state.
        let mut a = ChainState::new();
        let mut b = ChainState::new();
        let _ = a.advance(&req("foo", PackageKind::Style, &[]));
        let _ = b.advance(&req("quux", PackageKind::Style, &[]));
        let k1 = a.advance(&req("bar", PackageKind::Style, &[]));
        let k2 = b.advance(&req("bar", PackageKind::Style, &[]));
        assert_ne!(k1, k2);
    }

    // ---- cache mechanics (no real fs/network) -------------------------------

    #[test]
    fn disabled_cache_always_misses() {
        let cache = CacheStore::disabled();
        let mut cached = CachedResolved::default();
        cached.commands.insert(
            "foo".to_string(),
            SerializedMacroInfo {
                mandatory: 1,
                has_optional_leading: false,
            },
        );
        cache.put("somekey", &cached);
        assert!(cache.get("somekey").is_none());
    }

    #[test]
    fn cache_roundtrips_through_a_real_temp_dir() {
        let dir = std::env::temp_dir().join(format!(
            "latex-fmt-package-resolve-test-{}-{}",
            std::process::id(),
            "cache_roundtrips_through_a_real_temp_dir"
        ));
        let cache = CacheStore::at(&dir);
        let mut cached = CachedResolved::default();
        cached.commands.insert(
            "foo".to_string(),
            SerializedMacroInfo {
                mandatory: 2,
                has_optional_leading: true,
            },
        );
        cached.source_sha256 = Some("deadbeef".to_string());
        cached.resolved_via = Some(ResolvedVia::LocalFs);

        assert!(cache.get("abc123").is_none());
        cache.put("abc123", &cached);
        let hit = cache.get("abc123").expect("cache hit after put");
        assert_eq!(hit.commands["foo"].mandatory, 2);
        assert!(hit.commands["foo"].has_optional_leading);
        assert_eq!(hit.source_sha256.as_deref(), Some("deadbeef"));
        assert_eq!(hit.resolved_via, Some(ResolvedVia::LocalFs));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- resolver wiring, with a mock local-fs source (no real TeX/network) --

    #[test]
    fn resolver_with_no_channels_always_falls_back_empty() {
        let resolver = PackageResolver::builder().build();
        let r = req("nonexistent-package", PackageKind::Style, &[]);
        let resolved = resolver.resolve(&r, "anykey");
        assert!(resolved.is_empty());
        assert!(resolved.resolved_via().is_none());
    }

    #[test]
    fn resolver_extracts_definitions_from_mock_local_fs_content() {
        let dir = std::env::temp_dir().join(format!(
            "latex-fmt-package-resolve-test-{}-{}",
            std::process::id(),
            "resolver_extracts_definitions_from_mock_local_fs_content"
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mypkg.sty"),
            r"\newcommand{\mypkgcmd}[2]{#1 and #2}",
        )
        .unwrap();

        let cache_dir = dir.join("cache");
        let resolver = PackageResolver::builder()
            .with_cache(CacheStore::at(&cache_dir))
            .with_local_fs(LocalFsSource {
                extra_roots: vec![dir.clone()],
            })
            .build();

        let r = req("mypkg", PackageKind::Style, &[]);
        let mut chain = ChainState::new();
        let key = chain.advance(&r);
        let resolved = resolver.resolve(&r, &key);

        assert!(!resolved.is_empty());
        assert_eq!(resolved.commands()["mypkgcmd"].mandatory, 2);
        assert_eq!(resolved.resolved_via(), Some(ResolvedVia::LocalFs));
        assert!(resolved.source_sha256().is_some());

        // Second resolve() with the same key must come from cache, not
        // the local-fs channel again -- prove it by removing the source
        // file and confirming the result is unchanged.
        std::fs::remove_file(dir.join("mypkg.sty")).unwrap();
        let resolved_again = resolver.resolve(&r, &key);
        assert_eq!(resolved_again.commands()["mypkgcmd"].mandatory, 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- pre-scan ------------------------------------------------------------

    #[test]
    fn scan_finds_documentclass_and_usepackage() {
        let src = r"\documentclass[11pt]{article}\usepackage{amsmath}\usepackage[utf8]{inputenc}";
        let reqs = scan_package_loaders(src);
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].name, "article");
        assert_eq!(reqs[0].kind, PackageKind::Class);
        assert_eq!(reqs[0].options, vec!["11pt"]);
        assert_eq!(reqs[1].name, "amsmath");
        assert!(reqs[1].options.is_empty());
        assert_eq!(reqs[2].name, "inputenc");
        assert_eq!(reqs[2].options, vec!["utf8"]);
    }

    #[test]
    fn scan_expands_comma_separated_usepackage_list() {
        let src = r"\usepackage{amsmath,amssymb,graphicx}";
        let reqs = scan_package_loaders(src);
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].name, "amsmath");
        assert_eq!(reqs[1].name, "amssymb");
        assert_eq!(reqs[2].name, "graphicx");
    }

    #[test]
    fn scan_ignores_unrelated_commands() {
        let src = r"\textbf{hello}\section{intro}";
        let reqs = scan_package_loaders(src);
        assert!(reqs.is_empty());
    }

    // ---- end-to-end with a mock resolver (no real fs/network) -----------------

    #[test]
    fn end_to_end_seeds_definitions_from_resolved_package() {
        let dir = std::env::temp_dir().join(format!(
            "latex-fmt-package-resolve-test-{}-{}",
            std::process::id(),
            "end_to_end_seeds_definitions_from_resolved_package"
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mypkg.sty"), r"\newcommand{\greet}[1]{Hello, #1!}").unwrap();

        let resolver = PackageResolver::builder()
            .with_cache(CacheStore::disabled())
            .with_local_fs(LocalFsSource {
                extra_roots: vec![dir.clone()],
            })
            .build();

        let src = r"\usepackage{mypkg}\greet{world}";
        let (doc, diags) = parse_with_package_resolution(src, &resolver);

        // \greet resolved (not raw-preserved): exactly one Command node
        // named "greet" with one mandatory arg, and no
        // latex::unresolved-command diagnostic for it.
        let has_resolved_greet = doc.nodes.iter().any(|n| {
            matches!(n, crate::ast::Node::Command { name, args, .. } if name == "greet" && args.len() == 1)
        });
        assert!(has_resolved_greet, "{doc:#?}");
        assert!(
            !diags
                .iter()
                .any(|d| d.code == "latex::unresolved-command" && d.message.contains("greet"))
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn end_to_end_unresolvable_package_falls_back_to_raw_preserve() {
        let resolver = PackageResolver::builder().build(); // no channels
        let src = r"\usepackage{doesnotexist}\undefinedcmd{x}";
        let (doc, diags) = parse_with_package_resolution(src, &resolver);
        // \undefinedcmd still raw-preserves with its Info diagnostic,
        // exactly as plain parse() would.
        let has_control_symbol = doc.nodes.iter().any(
            |n| matches!(n, crate::ast::Node::ControlSymbol { name, .. } if name == "undefinedcmd"),
        );
        assert!(has_control_symbol, "{doc:#?}");
        assert!(diags.iter().any(|d| d.code == "latex::unresolved-command"));
    }
}

// ---- environment-dependent integration tests (real fs / real network) -----
//
// Gated `#[ignore]` since they depend on state outside this repo's
// control (a real TeX Live/MiKTeX install for the local-fs channel, a
// live network path to CTAN for the network channel). Run explicitly with
// `cargo test -p latex-fmt --all-features -- --ignored` on a machine known
// to have one or both available.
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    #[ignore = "requires a real local TeX installation (kpsewhich on PATH or a standard texmf tree)"]
    fn local_fs_resolves_a_real_installed_package() {
        let local = LocalFsSource::new();
        // amsmath ships with every standard LaTeX install.
        let bytes = local
            .locate("amsmath", PackageKind::Style)
            .expect("amsmath.sty should be found on a machine with a real TeX install");
        assert!(!bytes.is_empty());
        let text = String::from_utf8_lossy(&bytes);
        let (commands, _environments, _diags) = parse::extract_definitions(&text);
        assert!(
            !commands.is_empty(),
            "amsmath.sty should define at least one command the bounded engine can see"
        );
    }

    #[test]
    #[ignore = "requires live network access to CTAN"]
    fn network_resolves_a_real_ctan_package() {
        let network = NetworkSource::default();
        let bytes = network.locate("amsmath", PackageKind::Style);
        // Best-effort channel (see NetworkSource docs): assert only that
        // when it *does* find something, the bytes look like real LaTeX
        // source, not that it always succeeds for every package/mirror
        // layout.
        if let Some(bytes) = bytes {
            let text = String::from_utf8_lossy(&bytes);
            assert!(text.contains("\\ProvidesPackage") || text.contains("\\NeedsTeXFormat"));
        }
    }

    #[test]
    #[ignore = "requires live network access to CTAN"]
    fn full_resolver_end_to_end_against_real_ctan() {
        let dir = std::env::temp_dir().join(format!(
            "latex-fmt-package-resolve-integration-{}",
            std::process::id()
        ));
        let resolver = PackageResolver::builder()
            .with_cache(CacheStore::at(&dir))
            .with_network(NetworkSource::default())
            .build();
        let src = r"\usepackage{amsmath}";
        let (_doc, _diags) = parse_with_package_resolution(src, &resolver);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
