{
  inputs = {
    systems.url = "github:nix-systems/default";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixGL = {
      url = "github:nix-community/nixGL";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = inputs:
    inputs.flake-utils.lib.eachSystem
    [ inputs.flake-utils.lib.system.x86_64-linux ] (system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays =
            [ inputs.rust-overlay.overlays.default inputs.nixGL.overlay ];
          config.allowUnfree = false;
        };
        lib = inputs.nixpkgs.lib;
        cargoNix = import ./Cargo.nix {
          inherit pkgs;
          buildRustCrateForPkgs = pkgs:
            pkgs.buildRustCrate.override {
              defaultCrateOverrides = pkgs.defaultCrateOverrides // {
                egui_plot = old: {
                  patches = (old.patches or [ ])
                    ++ [ ./egui_nodrag_plot.patch ];
                };
              };
            };
        };
        pi = cargoNix.workspaceMembers.pi.build;
      in {
        formatter = pkgs.writeShellApplication {
          name = "format";
          runtimeInputs = [ pkgs.rust-bin.stable.latest.default pkgs.nixfmt ];
          text = ''
            set -v
            cargo fmt
            find . -name '*.nix' | grep -v Cargo.nix | xargs nixfmt'';
        };

        devShells.default = pkgs.mkShell {
          packages = let p = pkgs;
          in [
            p.bashInteractive
            p.cargo-flamegraph
            p.crate2nix
            p.nixgl.nixGLIntel
            (p.rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
          ];
          shellHook = ''
            git rev-parse --is-inside-work-tree > /dev/null && [ -n "$CARGO_TARGET_DIR_PREFIX" ] && \
            export CARGO_TARGET_DIR="$CARGO_TARGET_DIR_PREFIX$(git rev-parse --show-toplevel)"
            exec nixGLIntel zsh
          '';
        };

        packages = rec {
          default = piNixGLIntel;
          piNixGLIntel = pkgs.writeShellScriptBin "pi-nixGLIntel" ''
            exec ${lib.getExe pkgs.nixgl.nixGLIntel} ${lib.getExe pi}
          '';
          inherit pi;
        };
      });
}
