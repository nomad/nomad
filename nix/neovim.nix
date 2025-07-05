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
      xtask = "${common.xtask}/bin/xtask";

      crateInfos = builtins.fromJSON (
        builtins.readFile (
          pkgs.runCommand "crate-infos" { } ''
            ${xtask} neovim print-crate-infos > $out
          ''
        )
      );

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
          pluginSrc = src.any [ src.filters.rust src.filters.lua ] craneLib;
        in
        craneLib.buildPackage (
          targetCrane.commonArgs
          // {
            pname = crateInfos.name;
            version = crateInfos.version;
            src = pluginSrc;
            doCheck = false;
            buildPhaseCargoCommand = ''
              ${xtask} neovim build \
                ${lib.optionalString isNightly "--nightly"} \
                ${lib.optionalString isRelease "--release"} \
                ${lib.optionalString isCross "--target=${hostPlatform.rust.rustcTarget}"} \
                --out-dir=$out \
                --includes='${pluginSrc}/lua/nomad'
            '';
            # Installation was already handled by the build command.
            doNotPostBuildInstallCargoBinaries = true;
            installPhaseCommand = "";
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
            nativeBuildInputs = (common.nativeBuildInputs or [ ]) ++ [
              (mkPackage isNightly)
            ];
          }
        );

      mkReleaseArtifacts =
        targetPackageSets:
        pkgs.stdenv.mkDerivation {
          inherit (crateInfos) version;
          pname = "${crateInfos.name}-release-artifacts";
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

              mkArchiveName =
                args:
                let
                  inherit (common) workspaceName;
                  inherit (crateInfos) version;
                  neovimVersion = if args.isNightly then "nightly" else "stable";
                  arch = common.getArchString args.targetPkgs;
                  os = common.getOSString args.targetPkgs;
                in
                # TODO: neovimVersion should be e.g. 0.11 or 0.12-nightly, get
                # it from package.
                "${workspaceName}-${version}-for-neovim-${neovimVersion}-${os}-${arch}.tar.gz";

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
        neovim-nightly = mkPlugin { isNightly = true; };
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
