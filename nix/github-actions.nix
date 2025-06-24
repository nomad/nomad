{
  ...
}:

{
  perSystem =
    {
      pkgs,
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
          cleanedDevShell = builtins.removeAttrs devShell [ "packages" ];
        in
        pkgs.mkShell (
          cleanedDevShell
          // {
            buildInputs = (crane.commonArgs.buildInputs or [ ]) ++ (devShell.buildInputs or [ ]);
            nativeBuildInputs = (crane.commonArgs.nativeBuildInputs or [ ]) ++ (devShell.packages or [ ]);
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
