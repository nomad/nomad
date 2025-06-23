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
      mkNeovimPkg =
        isNightly: if isNightly then inputs'.neovim-nightly-overlay.packages.default else pkgs.neovim;

      mkDevShell =
        {
          isNightly,
        }:
        config.devShells.default.overrideAttrs (drv: {
          nativeBuildInputs = (drv.nativeBuildInputs or [ ]) ++ [
            (mkNeovimPkg isNightly)
          ];
        });

      mkPlugin =
        {
          isNightly,
          isRelease ? true,
        }:
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
                  cargo metadata \
                    --format-version 1 \
                    --no-deps \
                    --offline \
                    --manifest-path ${crane.commonArgs.src}/crates/mad-neovim/Cargo.toml | \
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
              "cargo xtask build ${nightlyFlag} ${releaseFlag}";
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
              (mkNeovimPkg isNightly)
            ];
          }
        );
    in
    {
      checks = {
        tests-neovim = mkTests { isNightly = false; };
        tests-neovim-nightly = mkTests { isNightly = true; };
      };
      devShells = {
        neovim = mkDevShell { isNightly = false; };
        neovim-nightly = mkDevShell { isNightly = true; };
      };
      packages = {
        neovim = mkPlugin { isNightly = false; };
        neovim-nightly = mkPlugin { isNightly = true; };
      };
    };
}
