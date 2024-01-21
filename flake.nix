{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nixGL = {
      url = "github:nix-community/nixGL";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
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

  outputs = inputs:
  inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ inputs.cargo2nix.overlays.default inputs.nixGL.overlay ];
          config.allowUnfree = false;
        };
        lib = inputs.nixpkgs.lib;
        rust = import ./rust.nix {
          inherit lib pkgs;
          extra-overrides = { mkNativeDep, mkEnvDep, mkRpath, mkOverride, p }:
            [
              (mkRpath "pi" [
                p.libglvnd
                p.libxkbcommon
                p.wayland
                p.xorg.libX11
                p.xorg.libXcursor
                p.xorg.libXi
                p.xorg.libXrandr
              ])
            (mkOverride "egui_plot" (old: {
              patches = (old.patches or []) ++ [./egui_nodrag_plot.patch ];
            }))
            ];
        };
      in {
        devShells.default = rust.workspaceShell {
          packages = let p = pkgs;
          in [
            inputs.cargo2nix.outputs.packages.${system}.cargo2nix
            p.nixgl.nixGLIntel
            p.cargo-flamegraph
            p.rust-bin.stable.latest.clippy
            p.rust-bin.stable.latest.default
            p.rust-bin.stable.latest.rust-analyzer
          ];
          shellHook = ''
            git rev-parse --is-inside-work-tree > /dev/null && [ -n "$CARGO_TARGET_DIR_PREFIX" ] && \
            export CARGO_TARGET_DIR="$CARGO_TARGET_DIR_PREFIX$(git rev-parse --show-toplevel)"
            nixGLIntel zsh
          '';
        };

        packages = rec {
          default = piNixGLIntel;
          piNixGLIntel = pkgs.writeShellScriptBin "pi-nixGLIntel" ''
            exec ${lib.getExe pkgs.nixgl.nixGLIntel} ${lib.getExe rust.pi}
          '';
          inherit (rust) pi;
        };
      });
}
