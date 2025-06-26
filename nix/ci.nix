{
  lib,
  flake-parts-lib,
  ...
}:

{
  options = {
    perSystem = flake-parts-lib.mkPerSystemOption (
      {
        config,
        pkgs,
        crane,
        ...
      }:
      {
        options.ciDevShells = lib.mkOption {
          type =
            let
              inherit (lib) types;
            in
            types.attrsOf (
              types.submodule {
                options = {
                  buildInputs = lib.mkOption {
                    type = types.listOf types.package;
                    default = [ ];
                    description = "List of packages to add to buildInputs";
                  };
                  packages = lib.mkOption {
                    type = types.listOf types.package;
                    default = [ ];
                    description = "List of packages to add to packages";
                  };
                  env = lib.mkOption {
                    type = types.attrsOf types.str;
                    default = { };
                    description = "Environment variables to set";
                  };
                };
              }
            );
          default = { };
          description = "CI development shells configuration";
        };

        config.devShells =
          let
            mkDevShell =
              devShell:
              let
                cleanedDevShell = builtins.removeAttrs devShell [
                  "buildInputs"
                  "packages"
                  "env"
                ];
              in
              pkgs.mkShell (
                cleanedDevShell
                // {
                  buildInputs = devShell.buildInputs ++ (crane.commonArgs.buildInputs or [ ]);
                  packages = devShell.packages ++ (crane.commonArgs.nativeBuildInputs or [ ]);
                  env = devShell.env // {
                    # Fingerprint code by file contents instead of mtime.
                    #
                    # Without this all the crates in the workspace would get
                    # re-built every time — even if there's a cache hit —
                    # because cargo's default behavior is to use a file's mtime
                    # to detect changes, and since the CI runners clone the
                    # repo from scratch every time, the source files would have
                    # newer timestamps than the cached build artifacts, making
                    # cargo think everything is stale.
                    #
                    # See the following links for more infos:
                    #
                    # https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#checksum-freshness
                    # https://github.com/rust-lang/cargo/issues/14136
                    # https://github.com/rust-lang/cargo/issues/6529
                    # https://blog.arriven.wtf/posts/rust-ci-cache/#target-based-cache
                    CARGO_UNSTABLE_CHECKSUM_FRESHNESS = "true";

                    # Setting this will disable some tests that fail in
                    # headless environments like CI.
                    HEADLESS = "true";
                  };
                }
              );
          in
          lib.mapAttrs' (name: devShell: {
            name = "ci-${name}";
            value = mkDevShell devShell;
          }) config.ciDevShells;
      }
    );
  };
}
