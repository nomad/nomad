{ inputs, ... }:

{
  perSystem =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    {
      _module.args.crane =
        let
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (
            pkgs:
            let
              toolchain =
                (inputs.rust-overlay.lib.mkRustBin { } pkgs).fromRustupToolchainFile
                  ../rust-toolchain.toml;
            in
            toolchain.override {
              extensions = (toolchain.extensions or [ ]) ++ [
                # Needed by cargo-llvm-cov to generate coverage.
                "llvm-tools-preview"
              ];
            }
          );

          src = craneLib.cleanCargoSource (craneLib.path ../.);
        in
        {
          lib = craneLib;

          commonArgs =
            let
              args = {
                inherit src;
                strictDeps = true;
                nativeBuildInputs = with pkgs; [ pkg-config ];
                buildInputs =
                  with pkgs;
                  lib.lists.optionals stdenv.isLinux [
                    # Needed by crates/auth to let "keyring" access the Secret
                    # Service.
                    dbus
                  ];
                # Crane will emit a warning if there's no
                # `workspace.package.name` set in the workspace's Cargo.lock,
                # so add a `pname` here to silence that.
                pname = "nomad";
                env = {
                  # Crane will run all 'cargo' invocation with `--release` if
                  # this is not unset.
                  CARGO_PROFILE = "";
                  # The .git directory is always removed from the flake's
                  # source files, so set the latest commit's hash and timestamp
                  # via environment variables or crates/version's build script
                  # will fail.
                  COMMIT_HASH = inputs.self.rev or (lib.removeSuffix "-dirty" inputs.self.dirtyRev);
                  COMMIT_UNIX_TIMESTAMP = toString inputs.self.lastModified;
                };
              };
            in
            args // { cargoArtifacts = craneLib.buildDepsOnly args; };

          devShell = craneLib.devShell {
            inherit (config) checks;
          };

          xtask = craneLib.buildPackage rec {
            inherit src;
            pname = "xtask";
            cargoExtraArgs = "--bin xtask";
            doCheck = false;
            env = {
              WORKSPACE_ROOT = src.outPath;
            };
            nativeBuildInputs = [
              # Needed to call `wrapProgram`.
              pkgs.makeWrapper
            ];
            # Needed to shell out to `cargo metadata`.
            #
            # TODO: how the fuck do you get the correct path to the `cargo`
            # binary used by `crane`?
            #
            # `${craneLib.cargo}/bin/cargo` evals to something like
            # /nix/store/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-rust-default-1.89.0-nightly-2025-06-22/bin/cargo"
            postInstall = ''
              wrapProgram $out/bin/${pname} \
                --set CARGO ${lib.getExe' craneLib.cargo "cargo"}
            '';
          };
        };
    };
}
