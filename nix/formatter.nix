{
  inputs,
  ...
}:

{
  perSystem =
    {
      config,
      pkgs,
      lib,
      crane,
      ...
    }:
    {
      checks.format = config.treefmt.build.check inputs.self;

      treefmt =
        let
          cargoSortPriority = 1;
        in
        {
          # We've already added a 'format' check.
          flakeCheck = false;
          inherit (config.flake-root) projectRootFile;
          programs.nixfmt.enable = true;
          programs.rustfmt = {
            enable = true;
            # cargo-sort messes up the indentation, so make sure to run taplo
            # after it.
            package = crane.lib.rustfmt;
          };
          programs.taplo = {
            enable = true;
            priority = cargoSortPriority + 1;
          };
          # TODO: make it format [workspace.dependencies].
          settings.formatter.cargo-sort = {
            command = "${pkgs.cargo-sort}/bin/cargo-sort";
            options =
              let
                cargoDotTomlSections = [
                  "package"
                  "lib"
                  "features"
                  "dependencies"
                  "build-dependencies"
                  "dev-dependencies"
                  "lints"
                ];
              in
              [
                # Only sort *within* newline-separated dependency groups, not
                # *across* them.
                "--grouped"
                "--order=${lib.concatStringsSep "," cargoDotTomlSections}"
              ];
            includes = [ "**/Cargo.toml" ];
            priority = cargoSortPriority;
          };
        };
    };
}
