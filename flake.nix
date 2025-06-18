{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts.url = "github:hercules-ci/flake-parts";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    neovim-nightly-overlay = {
      url = "github:nix-community/neovim-nightly-overlay/master";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
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
        {
          packages =
            let
              nightly-toolchain = inputs'.fenix.packages.fromToolchainFile {
                file = ./rust-toolchain.toml;
                sha256 = "sha256-SISBvV1h7Ajhs8g0pNezC1/KGA0hnXnApQ/5//STUbs=";
              };
            in
            {
              neovim =
                (pkgs.makeRustPlatform {
                  cargo = nightly-toolchain;
                  rustc = nightly-toolchain;
                }).buildRustPackage
                  {
                    pname = "mad-neovim";
                    version = "0.1.0";
                    src = ./.;
                    cargoLock = {
                      lockFile = ./Cargo.lock;
                      # TODO: remove after publishing private crates.
                      outputHashes = {
                        "abs-path-0.1.0" = lib.fakeHash;
                        "cauchy-0.1.0" = lib.fakeHash;
                        "codecs-0.0.9" = lib.fakeHash;
                        "lazy-await-0.1.0" = lib.fakeHash;
                        "nvim-oxi-0.6.0" = lib.fakeHash;
                        "pando-0.1.0" = lib.fakeHash;
                        "puff-0.1.0" = lib.fakeHash;
                      };
                    };
                    buildPhase = ''
                      runHook preBuild
                      cargo xtask build --release
                      runHook postBuild
                    '';
                    installPhase = ''
                      runHook preInstall
                      mkdir -p $out
                      cp -r lua $out/
                      runHook postInstall
                    '';
                  };
              neovim-nightly = { };
            };
          devShells = {
            default = pkgs.mkShell { };
            neovim = pkgs.mkShell { };
            neovim-nightly = pkgs.mkShell { };
          };
          formatter = pkgs.nixfmt-rfc-style;
        };
    };
}
