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
          sha256 = "sha256-SDu4snEWjuZU475PERvu+iO50Mi39KVjqCeJeNvpguU=";
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
