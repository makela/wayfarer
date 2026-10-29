{
  description = "Tai Chi Wayfarer blog";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    {
      # Runs the blog as a systemd service, see nix/module.nix
      nixosModules.default = import ./nix/module.nix;
    }
    // flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      with pkgs;
      {
        packages.default = callPackage ./nix/package.nix { };

        devShells.default = mkShell {
          shellHook = ''
            # Use the versioned hooks in .githooks (cargo fmt check before commit)
            git config core.hooksPath .githooks 2>/dev/null || true
          '';
          buildInputs = [
            rust-bin.stable.latest.default
            just
            watchexec
            watchman
            tailwindcss_4
          ];
        };
      }
    );
}
