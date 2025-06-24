{
  ...
}:

{
  perSystem =
    {
      crane,
      ...
    }:
    {
      checks.tests = crane.lib.cargoTest (
        crane.commonArgs
        // {
          checkPhase = ''
            # Run unit tests.
            cargo test --workspace --no-fail-fast

            # Run integration tests.
            cargo test --package=tests --features=auth,collab,mock,walkdir
          '';
          env = (crane.commonArgs.env or { }) // {
            # Setting this will disable some tests that fail in headless
            # environments like CI.
            HEADLESS = "true";
          };
        }
      );
    };
}
