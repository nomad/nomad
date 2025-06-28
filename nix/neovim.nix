{
  ...
}:

{
  perSystem =
    {
      config,
      pkgs,
      lib,
      inputs',
      crane,
      ...
    }:
    let
      mkPackage =
        isNightly: if isNightly then inputs'.neovim-nightly-overlay.packages.default else pkgs.neovim;

      mkCIShell =
        {
          isNightly,
        }:
        {
          packages = with crane.lib; [
            cargo
            rustc
            (mkPackage isNightly)
          ];
        };

      mkDevShell =
        {
          isNightly,
        }:
        config.devShells.default.overrideAttrs (drv: {
          nativeBuildInputs = (drv.nativeBuildInputs or [ ]) ++ [
            (mkPackage isNightly)
          ];
        });

      mkPlugin =
        {
          isNightly,
          isRelease ? true,
        }:
        let
          xtask = "${crane.xtask}/bin/xtask";

          # Get the crate's name and version.
          crateInfos = builtins.fromJSON (
            builtins.readFile (
              pkgs.runCommand "crate-infos" { } ''
                ${xtask} neovim print-crate-infos > $out"
              ''
            )
          );
        in
        crane.lib.buildPackage (
          crane.commonArgs
          // {
            inherit (crateInfos) pname version;
            doCheck = false;
            # We'll handle the installation ourselves.
            doNotPostBuildInstallCargoBinaries = true;
            buildPhaseCargoCommand =
              let
                nightlyFlag = lib.optionalString isNightly "--nightly";
                releaseFlag = lib.optionalString isRelease "--release";
              in
              "${xtask} neovim build ${nightlyFlag} ${releaseFlag}";
            installPhaseCommand = ''
              mkdir -p $out
              mv lua $out/
            '';
          }
        );

      mkTests =
        {
          isNightly,
        }:
        crane.lib.cargoTest (
          crane.commonArgs
          // {
            cargoTestExtraArgs = lib.concatStringsSep " " [
              "--package=tests"
              "--features=neovim${lib.optionalString isNightly "-nightly"}"
              "--no-fail-fast"
            ];
            nativeBuildInputs = (crane.commonArgs.nativeBuildInputs or [ ]) ++ [
              (mkPackage isNightly)
            ];
          }
        );
    in
    {
      checks = {
        tests-neovim = mkTests { isNightly = false; };
        tests-neovim-nightly = mkTests { isNightly = true; };
      };
      ciDevShells = {
        tests-neovim = mkCIShell { isNightly = false; };
        tests-neovim-nightly = mkCIShell { isNightly = true; };
      };
      devShells = {
        neovim = mkDevShell { isNightly = false; };
        neovim-nightly = mkDevShell { isNightly = true; };
      };
      packages = {
        neovim = mkPlugin { isNightly = false; };
        neovim-nightly = mkPlugin { isNightly = true; };
        neovim-release-artifacts = builtins.derivation {
          # - os: ubuntu-latest
          #   targets: "aarch64-unknown-linux-gnu,x86_64-unknown-linux-gnu"
          # - os: macos-latest
          #   targets: "aarch64-apple-darwin,x86_64-apple-darwin"
          #
          #
          # mkdir -p build
          #
          # IFS=',' read -ra TARGETS <<< "${{ matrix.targets }}"
          #
          # for neovim in "0.11" "nightly"; do
          #   for target in "${TARGETS[@]}"; do
          #     nightly_flag=$([ "$neovim" = "nightly" ] && echo "--nightly" || echo "")
          #     cargo xtask build $nightly_flag -- --release --target $target
          #     tar -czf "build/mad-neovim-$neovim-$target.tar.gz" lua
          #   done
          # done
        };
      };
    };
}
