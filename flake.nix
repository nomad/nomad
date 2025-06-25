{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    flake-root.url = "github:srid/flake-root";

    crane.url = "github:ipetkov/crane";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    neovim-nightly-overlay = {
      url = "github:nix-community/neovim-nightly-overlay/master";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
      inputs.treefmt-nix.follows = "treefmt-nix";
    };

    nix-develop-gha = {
      url = "github:nicknovitski/nix-develop";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];

      imports = [
        inputs.flake-root.flakeModule
        inputs.treefmt-nix.flakeModule
        ./nix/clippy.nix
        ./nix/coverage.nix
        ./nix/crane.nix
        ./nix/docs.nix
        ./nix/formatter.nix
        ./nix/github-actions.nix
        ./nix/neovim.nix
        ./nix/tests.nix
      ];

      perSystem =
        {
          config,
          lib,
          pkgs,
          crane,
          ...
        }:
        {
          # Workaround for https://github.com/NixOS/nix/issues/8881 so that
          # we can run individual checks with `nix run .#check-<foo>`.
          apps = lib.mapAttrs' (name: check: {
            name = "check-${name}";
            value = {
              type = "app";
              program =
                (pkgs.writeShellScript "check-${name}" ''
                  # Force evaluation of check ${check}.
                  echo -e "\033[1;32m✓\033[0m Check '${name}' passed"
                '').outPath;
            };
          }) config.checks;

          devShells.default = crane.devShell;
        };
    };

  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
      "https://nomad.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs"
      "nomad.cachix.org-1:jQ4al6yxQyvUBB7YJVJbMbc9rASokqamqvPhBUrVjww="
    ];
  };
}
