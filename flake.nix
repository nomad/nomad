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
      ];

      perSystem =
        {
          config,
          pkgs,
          lib,
          inputs',
          ...
        }:
        let
          crane =
            let
              mkToolchain =
                pkgs:
                let
                  toolchain =
                    (inputs.rust-overlay.lib.mkRustBin { } pkgs).fromRustupToolchainFile
                      ./rust-toolchain.toml;
                in
                toolchain.override {
                  extensions = (toolchain.extensions or [ ]) ++ [
                    # Needed by cargo-llvm-cov to generate coverage.
                    "llvm-tools-preview"
                  ];
                };
              craneLib = (inputs.crane.mkLib pkgs).overrideToolchain mkToolchain;
            in
            {
              lib = craneLib;
              commonArgs =
                let
                  args = {
                    src = craneLib.cleanCargoSource (craneLib.path ./.);
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
                    # `workspace.package.name` set in the workspace's
                    # Cargo.lock, so add a `pname` here to silence that.
                    pname = "mad";
                    COMMIT_HASH = inputs.self.rev;
                    COMMIT_UNIX_TIMESTAMP = toString inputs.self.lastModified;
                  };
                in
                args // { cargoArtifacts = craneLib.buildDepsOnly args; };
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
                  (cd tests && cargo llvm-cov test --no-report --features=auth,collab,walkdir)

                  # Generate coverage report.
                  cargo llvm-cov report --codecov --output-path codecov.json
                '';
                installPhaseCommand = ''
                  mkdir -p $out
                  mv codecov.json $out/
                '';
                # Clear default args since we're handling the build phase
                # manually.
                # cargoLlvmCovExtraArgs = "";
              }
            );
            neovim = neovim.packages.zero-dot-eleven;
            neovim-nightly = neovim.packages.nightly;
          };
          devShells = {
            default = common.devShell;
            neovim = neovim.devShells.zero-dot-eleven;
            neovim-nightly = neovim.devShells.nightly;
          };
          treefmt =
            let
              cargoSortPriority = 1;
            in
            {
              inherit (config.flake-root) projectRootFile;
              programs.nixfmt.enable = true;
              programs.rustfmt = {
                enable = true;
                package = crane.lib.rustfmt;
              };
              programs.taplo = {
                enable = true;
                # cargo-sort messes up the indentation, so make sure to run
                # taplo after it.
                priority = cargoSortPriority + 1;
              };
              # TODO: make it format [workspace.dependencies].
              settings.formatter.cargo-sort = {
                command = "${pkgs.cargo-sort}/bin/cargo-sort";
                options = [
                  # Only sort *within* newline-separated dependency groups, not
                  # *across* them.
                  "--grouped"
                  "--order=package,lib,features,dependencies,build-dependencies,dev-dependencies,lints"
                ];
                includes = [ "**/Cargo.toml" ];
                priority = cargoSortPriority;
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
