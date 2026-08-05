{
  description = "rescribe - Universal document conversion library";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenixPkgs = fenix.packages.${system};
        # Pinned stable toolchain, driven by rust-toolchain.toml. This is the single
        # source of truth for the Rust/rustfmt/clippy version: nixpkgs' own rustc
        # tracks nixpkgs-unstable and silently drifts whenever `flake.lock` is
        # updated, which previously caused a bare `cargo fmt` to reformat the whole
        # tree. Building the toolchain from the toml file via fenix instead means
        # the version only changes when someone deliberately bumps
        # rust-toolchain.toml (and CI is pinned to the same version — see
        # .github/workflows/ci.yml).
        rustToolchain = fenixPkgs.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-A1abGIbOtcBSdrUMhDGrER3pRM1hQP4fp9gh3Y4PKc8=";
        };
        # Nightly toolchain for fuzzing
        nightlyToolchain = fenixPkgs.latest.withComponents [
          "cargo"
          "rustc"
          "rust-src"
          "llvm-tools-preview"
        ];
      in
      {
        devShells.default = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            stdenv.cc.cc
            # Rust toolchain (pinned — see rust-toolchain.toml)
            rustToolchain
            rust-analyzer
            # Fast linker for incremental builds
            mold
            clang
            # JS tooling for docs
            bun
            # Pandoc — used by the local fixture harness (tests/pandoc.rs)
            pandoc
            # trang — RNG↔RNC schema conversion (for odf-fmt codegen)
            jing-trang
          ];
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH";

          # Sharing one target/ dir across the main checkout and every git
          # worktree of this repo is NOT handled here -- there is no
          # shellHook for it. It's handled at the filesystem level: each
          # worktree's target/ is a symlink (junction on windows) to the
          # main checkout's target/, created automatically by the
          # .githooks/post-checkout hook, or manually via
          # scripts/setup-worktree-target.sh / .ps1 -- see those scripts
          # and CLAUDE.md's "Development" section. Applies to nix users
          # too; there is nothing for this shell to do about it.
        };

        # Fuzzing shell with nightly Rust
        devShells.fuzz = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            stdenv.cc.cc
            # Nightly Rust for fuzzing
            nightlyToolchain
            # Fuzzing tool
            cargo-fuzz
            # Fast linker
            mold
            clang
          ];
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH";
        };
      }
    );
}
