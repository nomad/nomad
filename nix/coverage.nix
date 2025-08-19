{
  ...
}:

{
  perSystem =
    {
      pkgs,
      crane,
      rust,
      ...
    }:
    let
      mkToolchain =
        prev:
        prev.override {
          extensions = (prev.extensions or [ ]) ++ [
            # Needed by cargo-llvm-cov to generate coverage.
            "llvm-tools-preview"
          ];
        };

      craneLib = (crane.overrideToolchain (_pkgs: prev: (mkToolchain prev))).lib;
    in
    {
      packages.coverage = craneLib.cargoLlvmCov (
        crane.commonArgs
        // {
          buildPhaseCargoCommand = ''
            # Run unit tests.
            cargo llvm-cov --no-report --workspace

            # Run integration tests.
            cargo llvm-cov --no-report --package=tests --all-features

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
          (mkToolchain rust.toolchain)
          pkgs.cargo-llvm-cov
        ];
      };
    };
}
