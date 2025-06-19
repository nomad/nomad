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
          pkgs,
          lib,
          ...
        }:
        let
          common = rec {
            devShells = {
              default = pkgs.mkShell {
                buildInputs =
                  with pkgs;
                  [
                    pkg-config
                    # Needed by /benches to let git2 clone the Neovim repo.
                    openssl
                  ]
                  ++ lib.lists.optionals stdenv.isLinux [
                    # Needed by /crates/auth to let "keyring" access the Secret
                    # Service.
                    dbus
                  ];
                nativeBuildInputs = with pkgs; [
                  pkg-config
                  openssl
                  (rust.toolchain.withComponents [
                    "cargo"
                    "clippy"
                    "rust-src"
                    "rust-std"
                    "rustc"
                    "rustfmt"
                  ])
                ];
              };
            };
            crane = rec {
              lib = (inputs.crane.mkLib pkgs).overrideToolchain (
                pkgs: (inputs.rust-overlay.lib.mkRustBin { } pkgs).fromRustupToolchainFile ./rust-toolchain.toml
              );
              src = lib.cleanCargoSource (lib.path ./.);
              commonArgs =
                let
                  args = {
                    inherit src;
                    strictDeps = true;
                    buildInputs = with pkgs; [
                      pkg-config
                      openssl.dev
                    ];
                  };
                in
                args // { cargoArtifacts = lib.buildDepsOnly args; };
            };
          };

          neovim =
            let
              buildPlugin =
                let
                  pname = "mad-neovim";
                  version = "0.1.0";
                in
                {
                  isNightly,
                  isRelease ? true,
                }:
                common.crane.lib.buildPackage (
                  common.crane.commonArgs
                  // {
                    inherit pname version;
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
                );
            in
            {
              packages = {
                zero-dot-eleven = buildPlugin { isNightly = false; };
                nightly = buildPlugin { isNightly = true; };
              };
              devShells = {
                zero-dot-eleven = common.devShells.default.overrideAttrs (old: {
                  nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [
                    pkgs.neovim
                  ];
                });
                nightly = common.devShells.default.overrideAttrs (old: {
                  nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [
                    inputs'.neovim-nightly-overlay.packages.default
                  ];
                });
              };
            };
        in
        {
          apps = {
            nix-develop-gha = {
              type = "app";
              program = "${inputs'.nix-develop-gha.packages.default}/bin/nix-develop-gha";
            };
          };
          packages = {
            neovim = neovim.packages.zero-dot-eleven;
            neovim-nightly = neovim.packages.nightly;
          };
          devShells = {
            default = common.devShells.default;
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
