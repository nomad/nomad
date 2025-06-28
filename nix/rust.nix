{ inputs, ... }:

{
  perSystem =
    {
      pkgs,
      lib,
      crane,
      ...
    }:
    {
      _module.args.rust = rec {
        buildInputs =
          with pkgs;
          lib.lists.optionals stdenv.isLinux [
            # Needed by crates/auth to let "keyring" access the Secret
            # Service.
            dbus
          ];

        nativeBuildInputs = with pkgs; [ pkg-config ];

        mkToolchain =
          pkgs: (inputs.rust-overlay.lib.mkRustBin { } pkgs).fromRustupToolchainFile ../rust-toolchain.toml;

        toolchain = mkToolchain pkgs;

        xtask = crane.lib.buildPackage (
          crane.commonArgs
          // rec {
            pname = "xtask";
            cargoExtraArgs = "--bin xtask";
            doCheck = false;
            env = {
              # Crane will compile xtask in release mode if this is not unset.
              CARGO_PROFILE = "";
              WORKSPACE_ROOT = crane.commonArgs.src.outPath;
            };
            nativeBuildInputs = [
              # Needed to call `wrapProgram`.
              pkgs.makeWrapper
            ];
            # Needed to shell out to `cargo metadata`.
            postInstall = ''
              wrapProgram $out/bin/${pname} \
                --set CARGO ${lib.getExe' toolchain "cargo"} \
                --set RUSTC ${lib.getExe' toolchain "rustc"}
            '';
          }
        );
      };
    };
}
