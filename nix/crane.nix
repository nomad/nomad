{ inputs, ... }:

{
  perSystem =
    {
      pkgs,
      lib,
      common,
      rust,
      ...
    }:
    {
      _module.args.crane =
        let
          mkCraneLib =
            targetPkgs: toolchain: (inputs.crane.mkLib targetPkgs).overrideToolchain (_targetPkgs: toolchain);

          mkCommonArgs =
            targetPkgs: toolchain:
            let
              craneLib = mkCraneLib targetPkgs toolchain;

              depsArgs = {
                inherit (common) nativeBuildInputs;
                buildInputs = common.buildInputsFor targetPkgs;
                pname = common.workspaceName;
                src = craneLib.cleanCargoSource (craneLib.path ../.);
                strictDeps = true;
              };
            in
            depsArgs
            // {
              cargoArtifacts = craneLib.buildDepsOnly depsArgs;
              env = {
                # Crane will run all 'cargo' invocations with `--release` if
                # this is not unset.
                CARGO_PROFILE = "";
                # The .git directory is always removed from the flake's source
                # files, so set the latest commit's hash and timestamp via
                # environment variables or crates/version's build script will
                # fail.
                COMMIT_HASH = inputs.self.rev or (lib.removeSuffix "-dirty" inputs.self.dirtyRev);
                COMMIT_UNIX_TIMESTAMP = toString inputs.self.lastModified;
              };
            };

          mkCrane = targetPkgs: toolchain: {
            # lib :: craneLib
            lib = mkCraneLib targetPkgs toolchain;

            # commonArgs :: set
            commonArgs = mkCommonArgs targetPkgs toolchain;

            # overridePkgs :: newPkgs -> crane
            #
            # Overrides the package set used by Crane. If the new package set
            # is for cross-compilation (i.e. its buildPlatform differs from its
            # hostPlatform), this will also add the Rust target corresponding
            # to the new hostPlatform to the Rust toolchain.
            overridePkgs =
              newTargetPkgs:
              let
                inherit (newTargetPkgs.stdenv) buildPlatform hostPlatform;

                newToolchain =
                  if buildPlatform != hostPlatform then
                    toolchain.override {
                      targets = lib.unique (
                        (toolchain.targets or [ ])
                        ++ [
                          hostPlatform.rust.rustcTarget
                        ]
                      );
                    }
                  else
                    toolchain;
              in
              mkCrane newTargetPkgs newToolchain;

            # overrideToolchain :: (pkgs: prevToolchain: newToolchain) -> crane
            #
            # Similar to `craneLib.overrideToolchain`, but is given the current
            # toolchain in addition to the package set, which allows us to keep
            # adding targets and extensions if called multiple times.
            overrideToolchain = mkNewToolchain: mkCrane targetPkgs (mkNewToolchain targetPkgs toolchain);

            # toolchain :: drv
            inherit toolchain;
          };
        in
        mkCrane pkgs (
          let
            toolchain = rust.mkToolchain pkgs;
          in
          # No fucking clue why this is necessary, but not having it causes
          # `lib.getExe' toolchain "cargo"` in the common.xtask derivation to
          # return a store path like
          # /nix/store/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-rust-default-1.89.0-nightly-2025-06-22/bin/cargo
          toolchain.override {
            extensions = (toolchain.extensions or [ ]);
          }
        );
    };
}
