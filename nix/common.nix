{ ... }:

{
  perSystem =
    {
      pkgs,
      lib,
      crane,
      src,
      ...
    }:
    {
      _module.args.common = {
        # The list of libraries to be linked against needed to compile all the
        # crates in the workspace with only their default features enabled.
        buildInputsFor =
          targetPkgs:
          with targetPkgs;
          lib.lists.optionals stdenv.isLinux [
            # Needed by crates/auth to let "keyring" access the Secret
            # Service.
            dbus
          ];

        cartesianProduct =
          specs:
          let
            # Fold over the specs, extending each partial combination with
            # every value for the current key.
            step = (
              acc: spec:
              lib.flatten (
                builtins.map (combo: builtins.map (val: combo // { "${spec.name}" = val; }) spec.values) acc
              )
            );
          in
          # Start with a list containing a single empty attrset.
          builtins.foldl' step [ { } ] specs;

        # Returns the human-readable architecture string for the given package
        # set ("x86_64" or "aarch64") to be used in the release artifacts' file
        # names.
        getArchString =
          pkgs:
          let
            inherit (pkgs.stdenv) hostPlatform;
          in
          if hostPlatform.isx86_64 then
            "x86_64"
          else if hostPlatform.isAarch64 then
            "aarch64"
          else
            throw "unsupported target architecture: ${hostPlatform.system}";

        # Returns the human-readable OS string for the given package set
        # ("linux" or "darwin") to be used in the release artifacts' file
        # names.
        getOSString =
          pkgs:
          let
            inherit (pkgs.stdenv) hostPlatform;
          in
          if hostPlatform.isLinux then
            "linux"
          else if hostPlatform.isDarwin then
            "macos"
          else
            throw "unsupported target OS: ${hostPlatform.system}";

        # The list of executables that have to be in $PATH needed to compile
        # all the crates in the workspace with only their default features
        # enabled (excluding packages from the Rust toolchain like cargo and
        # rustc).
        nativeBuildInputs = with pkgs; [ pkg-config ];

        # A compiled version of the xtask executable defined in this workspace.
        xtask = crane.lib.buildPackage (
          let
            pname = "xtask";
            xtaskSrc = src.rust crane.lib;
          in
          {
            inherit (crane.commonArgs) cargoArtifacts strictDeps;
            inherit pname;
            src = xtaskSrc;
            cargoExtraArgs = "--bin xtask";
            doCheck = false;
            env = {
              # Crane will compile xtask in release mode if this is not unset.
              CARGO_PROFILE = "";
              WORKSPACE_ROOT = xtaskSrc.outPath;
            };
          }
        );

        # Our workspace doesn't have a default package, so set one here to be
        # used in the release artifacts' file names.
        workspaceName = "nomad";
      };
    };
}
