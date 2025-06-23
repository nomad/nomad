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
        ./nix/crane.nix
        ./nix/formatter.nix
        ./nix/neovim.nix
      ];

      perSystem =
        {
          config,
          lib,
          inputs',
          crane,
          ...
        }:
        {
          apps =
            {
              nix-develop-gha = {
                type = "app";
                program = "${inputs'.nix-develop-gha.packages.default}/bin/nix-develop-gha";
              };
            }
            # Workaround for https://github.com/NixOS/nix/issues/8881 so that
            # we can run individual checks with `nix run .#check-<foo>`.
            // lib.mapAttrs' (name: check: {
              name = "check-${name}";
              value = {
                type = "app";
                program = "${check}";
              };
            }) config.checks;
          checks = {
            clippy = crane.lib.cargoClippy (
              crane.commonArgs
              // {
                cargoClippyExtraArgs = lib.concatStringsSep " " [
                  "--all-features"
                  "--all-targets"
                  "--no-deps"
                  "--workspace"
                  "--"
                  "--deny warnings"
                ];
              }
            );
            docs = crane.lib.cargoDoc (
              crane.commonArgs
              // {
                cargoDocExtraArgs = lib.concatStringsSep " " [
                  "--all-features"
                  "--no-deps"
                  "--workspace"
                ];
                env = {
                  RUSTFLAGS = "--deny warnings";
                };
              }
            );
            fmt = config.treefmt.build.check inputs.self;
          };
          packages = {
            coverage = crane.lib.cargoLlvmCov (
              crane.commonArgs
              // {
                buildPhaseCargoCommand = ''
                  # Run unit tests.
                  (cd crates && cargo llvm-cov test --no-report)

                  # Run integration tests.
                  (cd tests && cargo llvm-cov test --no-report --features=auth,collab,mock,walkdir)

                  # Generate coverage report.
                  cargo llvm-cov report --codecov --output-path codecov.json
                '';
                installPhaseCommand = ''
                  mkdir -p $out
                  mv codecov.json $out/
                '';
                env = (crane.commonArgs.env or { }) // {
                  # Setting this will disable some tests that fail in headless
                  # environments like CI.
                  HEADLESS = "true";
                };
              }
            );
          };
          devShells = {
            default = crane.lib.devShell {
              inherit (config) checks;
            };
          };
        };
    };

  nixConfig = {
    extra-substituters = [ "https://nix-community.cachix.org" ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs"
    ];
  };
}
