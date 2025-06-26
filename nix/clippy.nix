{
  ...
}:

{
  perSystem =
    {
      lib,
      crane,
      ...
    }:
    {
      checks.clippy = crane.lib.cargoClippy (
        crane.commonArgs
        // {
          cargoClippyExtraArgs = lib.concatStringsSep " " [
            "--all-features"
            "--all-targets"
            "--no-deps"
            "--workspace"
          ];
          env = (crane.commonArgs.env or { }) // {
            RUSTFLAGS = "--deny warnings";
          };
        }
      );

      ciDevShells.clippy = {
        packages = with crane.lib; [
          cargo
          clippy
          rustc
        ];
        env = {
          RUSTFLAGS = "--deny warnings";
        };
      };
    };
}
