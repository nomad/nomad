{ ... }:

{
  perSystem =
    { lib, ... }:
    let
      repoRoot = craneLib: craneLib.path ../.;

      filters = {
        # Keeps all the files matched by at least one of the given filters.
        any =
          filtersList: path: type: craneLib:
          builtins.any (filter: filter path type craneLib) filtersList;

        # Keeps all the Lua files and all symlinks under /lua/nomad.
        lua =
          path: type: craneLib:
          (
            (lib.hasSuffix ".lua" (builtins.baseNameOf path))
            || (type == "symlink" && lib.hasPrefix "${repoRoot craneLib}/lua/nomad" path)
          );

        # Keeps all the Rust-related files in the whole repo.
        rust =
          path: type: craneLib:
          craneLib.filterCargoSources path type;

        # Keeps all the Rust-related files under /xtask (and the workspace
        # manifest).
        #
        # NOTE: using this instead of the 'rust' filter doesn't work because it
        # excludes various directories mentioned in the workspace manifest's
        # [workspace.members] section.
        #
        # TODO: instead of copying the manifest verbatim, can we overwrite its
        # members to only keep e.g. crates/* and xtask?
        xtask =
          path: type: craneLib:
          ((lib.hasPrefix "${repoRoot craneLib}/xtask" path) && (craneLib.filterCargoSources path type))
          || (type == "file" && path == "${repoRoot craneLib}/Cargo.lock")
          || (type == "file" && path == "${repoRoot craneLib}/Cargo.toml");
      };

      mkSource =
        filter: craneLib:
        craneLib.cleanCargoSource.override {
          filterCargoSources = path: type: (filter path type craneLib);
        } (repoRoot craneLib);
    in
    {
      _module.args.src = {
        inherit filters;
        any = filtersList: mkSource (filters.any filtersList);
        lua = mkSource filters.lua;
        rust = mkSource filters.rust;
      };
    };
}
