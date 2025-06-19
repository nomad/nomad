{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    crane.url = "github:ipetkov/crane";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    neovim-nightly-overlay = {
      url = "github:nix-community/neovim-nightly-overlay/master";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };

    nix-develop-gha = {
      url = "github:nicknovitski/nix-develop";
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

      perSystem =
        {
          inputs',
          config,
          pkgs,
          lib,
          ...
        }:
        let
          crane = rec {
            lib =
              let
                mkToolchain =
                  pkgs: (inputs.rust-overlay.lib.mkRustBin { } pkgs).fromRustupToolchainFile ./rust-toolchain.toml;
              in
              (inputs.crane.mkLib pkgs).overrideToolchain mkToolchain;
            commonArgs =
              let
                args = {
                  src = lib.cleanCargoSource (lib.path ./.);
                  strictDeps = true;
                  nativeBuildInputs = with pkgs; [ pkg-config ];
                  buildInputs =
                    with pkgs;
                    [
                      # Needed by /benches to let git2 clone the Neovim repo.
                      openssl
                    ]
                    ++ lib.lists.optionals stdenv.isLinux [
                      # Needed by /crates/auth to let "keyring" access the
                      # Secret Service.
                      dbus
                    ];
                  # Crane will emit a warning if there's no
                  # `workspace.package.name` set in the workspace's Cargo.lock,
                  # so add a `pname` here to silence that.
                  pname = "mad";
                };
              in
              args // { cargoArtifacts = lib.buildDepsOnly args; };
          };

          common = {
            devShell = crane.lib.devShell {
              inherit (config) checks;
            };
          };

          neovim =
            let
              buildPlugin =
                {
                  isNightly,
                  isRelease ? true,
                }:
                crane.lib.buildPackage (
                  crane.commonArgs
                  // (
                    let
                      # Get the crate's name and version.
                      crateInfos = builtins.fromJSON (
                        builtins.readFile (
                          pkgs.runCommand "cargo-metadata"
                            {
                              nativeBuildInputs = [
                                crane.lib.cargo
                                pkgs.jq
                              ];
                            }
                            ''
                              cd ${crane.commonArgs.src}
                              cargo metadata \
                                --format-version 1 \
                                --no-deps \
                                --offline \
                                --manifest-path crates/mad-neovim/Cargo.toml | \
                              jq '
                                .workspace_default_members[0] as $default_id |
                                .packages[] |
                                select(.id == $default_id) |
                                {pname: .name, version: .version}
                              ' > $out
                            ''
                        )
                      );
                    in
                    {
                      inherit (crateInfos) pname version;
                      buildPhaseCargoCommand =
                        let
                          nightlyFlag = lib.optionalString isNightly "--nightly";
                          releaseFlag = lib.optionalString isRelease "--release";
                        in
                        "cargo xtask build ${nightlyFlag} ${releaseFlag}";
                      installPhaseCommand = ''
                        mkdir -p $out
                        cp -r lua $out/
                      '';
                      doCheck = false;
                    }
                  )
                );
            in
            {
              packages = {
                zero-dot-eleven = buildPlugin { isNightly = false; };
                nightly = buildPlugin { isNightly = true; };
              };
              devShells = {
                zero-dot-eleven = common.devShell.overrideAttrs (old: {
                  nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [
                    pkgs.neovim
                  ];
                });
                nightly = common.devShell.overrideAttrs (old: {
                  nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [
                    inputs'.neovim-nightly-overlay.packages.default
                  ];
                });
              };
            };
        in
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
                  "--workspace"
                  "--"
                  "--deny warnings"
                ];
              }
            );
          };
          packages = {
            neovim = neovim.packages.zero-dot-eleven;
            neovim-nightly = neovim.packages.nightly;
          };
          devShells = {
            default = common.devShell;
            neovim = neovim.devShells.zero-dot-eleven;
            neovim-nightly = neovim.devShells.nightly;
          };
          formatter = pkgs.nixfmt-rfc-style;
        };
    };

  nixConfig = {
    extra-substituters = [ "https://nix-community.cachix.org" ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs"
    ];
  };
}
