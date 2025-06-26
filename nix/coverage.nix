{
  ...
}:

{
  perSystem =
    {
      pkgs,
      crane,
      ...
    }:
    {
      packages.coverage = crane.lib.cargoLlvmCov (
        crane.commonArgs
        // {
          buildPhaseCargoCommand = ''
            # Run unit tests.
            cargo llvm-cov --no-report --workspace

            # Run integration tests.
            cargo llvm-cov --no-report --package=tests --features=auth,collab,mock,walkdir

            # Generate coverage report.
            cargo llvm-cov report --codecov --output-path codecov.json
          '';
          installPhaseCommand = ''
            mkdir -p $out
            mv codecov.json $out/
          '';
        }
      );
      ciDevShells.coverage = {
        packages = [
          crane.lib.cargo
          crane.lib.rustc
          pkgs.cargo-llvm-cov
        ];
      };
    };
}
