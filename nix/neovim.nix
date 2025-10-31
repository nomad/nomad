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
      common,
      crane,
      rust,
      src,
      ...
    }:
    let
      inherit (common) releaseTag;

      cargoToml = lib.importTOML ../crates/nomad-neovim/Cargo.toml;

      mkPackage =
        isNightly: if isNightly then inputs'.neovim-nightly-overlay.packages.default else pkgs.neovim;

      mkCIShell =
        {
          isNightly,
        }:
        {
          packages = [
            (rust.toolchain)
            (mkPackage isNightly)
            pkgs.cargo-nextest
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
          targetPkgs ? pkgs,
        }:
        let
          inherit (targetPkgs.stdenv) buildPlatform hostPlatform;
          isCross = buildPlatform != hostPlatform;
          # Use native crane unless cross-compiling.
          targetCrane = if isCross then crane.overridePkgs targetPkgs else crane;
          craneLib = targetCrane.lib;
          rustPlugin = craneLib.buildPackage (
            targetCrane.commonArgs
            // {
              pname = cargoToml.package.name;
              version = if releaseTag != null then releaseTag else "dev";
              src = src.rust craneLib;
              doCheck = false;
              buildPhaseCargoCommand = ''
                ${lib.getExe common.xtask} neovim build \
                  ${lib.optionalString isNightly "--nightly"} \
                  ${lib.optionalString isRelease "--release"} \
                  ${lib.optionalString isCross "--target=${hostPlatform.rust.rustcTarget}"} \
                  --out-dir=$out \
                  --includes=
              '';
              # Installation was already handled by the build command.
              doNotPostBuildInstallCargoBinaries = true;
              installPhaseCommand = "";
            }
          );
        in
        targetPkgs.runCommand cargoToml.package.name { } ''
          mkdir -p $out
          cp -r ${rustPlugin}/lua $out/
          chmod -R +w $out/lua
          cp -r ${../lua}/* $out/lua
        '';

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
            nativeBuildInputs = (common.nativeBuildInputs or [ ]) ++ [
              (mkPackage isNightly)
            ];
          }
        );

      mkReleaseArtifacts =
        targetPackageSets:
        pkgs.stdenv.mkDerivation {
          pname = "${cargoToml.package.name}-release-artifacts";
          version = if releaseTag != null then releaseTag else "dev";
          src = null;
          dontUnpack = true;
          nativeBuildInputs = with pkgs; [
            gnutar
            gzip
          ];
          installPhase =
            let
              args = common.cartesianProduct [
                {
                  name = "isNightly";
                  values = [
                    true
                    false
                  ];
                }
                {
                  name = "targetPkgs";
                  values = targetPackageSets;
                }
              ];

              mkNeovimVersion =
                isNightly:
                let
                  stable = pkgs.neovim.version;
                in
                if !isNightly then
                  lib.versions.majorMinor stable
                else
                  # The 'version' of the package given by
                  # neovim-nightly-overlay is just "nightly", so we have to
                  # construct it manually by increasing the minor version of
                  # the latest stable release.
                  let
                    major = lib.versions.major stable;
                    minor = lib.versions.minor stable;
                    minorPlusOne = builtins.toString ((lib.toInt minor) + 1);
                  in
                  "${major}.${minorPlusOne}-nightly";

              mkArchiveName =
                args:
                let
                  inherit (common) releaseTag workspaceName;
                  tag = if releaseTag != null then "-${releaseTag}" else "";
                  neovimVersion = mkNeovimVersion args.isNightly;
                  arch = common.getArchString args.targetPkgs;
                  os = common.getOSString args.targetPkgs;
                in
                "${workspaceName}${tag}-for-neovim-${neovimVersion}-${os}-${arch}.tar.gz";

              archivePlugins =
                let
                  archivePlugin =
                    args:
                    let
                      archiveName = mkArchiveName args;
                      plugin = mkPlugin args;
                    in
                    "tar -czf \"$out/${archiveName}\" -C \"${plugin}\" lua";
                in
                builtins.map archivePlugin args;
            in
            ''
              runHook preInstall
              mkdir -p $out
              ${lib.concatStringsSep "\n" archivePlugins}
              runHook postInstall
            '';
        };
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
        neovim-debug = mkPlugin {
          isNightly = false;
          isRelease = false;
        };
        neovim-nightly = mkPlugin { isNightly = true; };
        neovim-nightly-debug = mkPlugin {
          isNightly = true;
          isRelease = false;
        };
        neovim-release-artifacts-linux = mkReleaseArtifacts [
          pkgs.pkgsCross.aarch64-multiplatform
          pkgs.pkgsCross.gnu64
        ];
        # Cross-compiling for macOS requires proprietary Apple tooling which is
        # only available when building on a macOS host.
        neovim-release-artifacts-macos = lib.mkIf pkgs.stdenv.isDarwin (mkReleaseArtifacts [
          pkgs.pkgsCross.aarch64-darwin
          pkgs.pkgsCross.x86_64-darwin
        ]);
      };
    };
}
