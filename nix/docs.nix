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
      checks.docs = crane.lib.cargoDoc (
        crane.commonArgs
        // {
          cargoDocExtraArgs = lib.concatStringsSep " " [
            "--all-features"
            "--no-deps"
            "--workspace"
          ];
          env = (crane.commonArgs.env or { }) // {
            RUSTFLAGS = "--deny warnings";
          };
        }
      );

      ciDevShells.docs = {
        packages = with crane.lib; [
          cargo
          rustc
        ];
        env = {
          RUSTFLAGS = "--deny warnings";
        };
      };
    };
}
