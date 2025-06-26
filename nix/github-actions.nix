{
  ...
}:

{
  perSystem =
    {
      inputs',
      ...
    }:
    {
      apps.nix-develop-gha = {
        type = "app";
        program = "${inputs'.nix-develop-gha.packages.default}/bin/nix-develop-gha";
      };
    };
}
