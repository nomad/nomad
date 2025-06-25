{
  ...
}:

{
  perSystem =
    {
      lib,
      inputs',
      crane,
      ...
    }:
    let
      devShells = {
        tests = {
          packages = with crane.lib; [
            cargo
            rustc
          ];
          env = {
            # Setting this will disable some tests that fail in headless
            # environments like CI.
            HEADLESS = "true";
          };
        };
      };

      mkDevShell =
        devShell:
        let
          cleanedDevShell = builtins.removeAttrs devShell [
            "packages"
            "env"
          ];
        in
        inputs'.devshell.legacyPackages.mkShell (
          cleanedDevShell
          // {
            packages =
              (crane.commonArgs.buildInputs or [ ])
              ++ (crane.commonArgs.nativeBuildInputs or [ ])
              ++ (devShell.packages or [ ]);

            env =
              let
                envVars = (devShell.env or { }) // {
                  # Fingerprint code by file contents instead of mtime.
                  #
                  # Without this all the crates in the workspace would get
                  # re-built every time — even if there's a cache hit — because
                  # cargo's default behavior is to use a file's mtime to detect
                  # changes, and since the CI runners clone the repo from
                  # scratch every time, the source files would have newer
                  # timestamps than the cached build artifacts, making cargo
                  # think everything is stale.
                  #
                  # See the following links for more infos:
                  #
                  # https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#checksum-freshness
                  # https://github.com/rust-lang/cargo/issues/14136
                  # https://github.com/rust-lang/cargo/issues/6529
                  # https://blog.arriven.wtf/posts/rust-ci-cache/#target-based-cache
                  CARGO_UNSTABLE_CHECKSUM_FRESHNESS = "true";
                };
              in
              lib.mapAttrsToList (name: value: {
                inherit name value;
              }) envVars;
          }
        );
    in
    {
      apps.nix-develop-gha = {
        type = "app";
        program = "${inputs'.nix-develop-gha.packages.default}/bin/nix-develop-gha";
      };

      devShells = lib.mapAttrs' (name: devShell: {
        name = "gha-${name}";
        value = mkDevShell devShell;
      }) devShells;
    };
}
