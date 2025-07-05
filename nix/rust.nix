{ inputs, ... }:

{
  perSystem =
    {
      pkgs,
      crane,
      ...
    }:
    {
      _module.args.rust =
        let
          mkToolchain =
            pkgs:
            (inputs.rust-overlay.lib.mkRustBin { } pkgs).fromRustupToolchainFile (
              crane.lib.path ../rust-toolchain.toml
            );
        in
        {
          inherit mkToolchain;
          toolchain = mkToolchain pkgs;
        };
    };
}
