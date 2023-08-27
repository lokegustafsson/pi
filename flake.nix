{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix";
      inputs.rust-overlay.follows = "rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, cargo2nix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ cargo2nix.overlays.default ];
          config.allowUnfree = false;
        };
        lib = nixpkgs.lib;
        rust = import ./rust.nix {
          inherit lib pkgs;
          extra-overrides = { mkNativeDep, mkEnvDep, mkRpath, mkOverride, p }:
            [
              (mkRpath "pi" [
                p.xorg.libX11
                p.xorg.libXcursor
                p.xorg.libXi
                p.xorg.libXrandr
                p.libglvnd
              ])
            ];
        };
      in {
        devShells.default = rust.workspaceShell {
          packages = let p = pkgs;
          in [
            cargo2nix.outputs.packages.${system}.cargo2nix
            p.rust-bin.stable.latest.clippy
            p.rust-bin.stable.latest.default
          ];
          shellHook = ''
            git rev-parse --is-inside-work-tree > /dev/null && \
            export CARGO_TARGET_DIR="$HOME/cargo-target-dir$(git rev-parse --show-toplevel)"
          '';

          LD_LIBRARY_PATH = let p = pkgs;
          in lib.makeLibraryPath [
            p.xorg.libX11
            p.xorg.libXcursor
            p.xorg.libXi
            p.xorg.libXrandr
            p.libglvnd
          ];
        };

        packages.default = rust.pi;
      });
}
