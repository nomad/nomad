# Installation

Being written in Rust, Nomad requires a build step before it can be loaded by
Neovim.

In principle, this could be done with any plugin manager that supports
user-defined build functions, but we currently only support lazy.nvim. If you'd
like to use Nomad with a different package manager, please open an issue!

## lazy.nvim

Simply add this snippet to your lazy config:

```lua
{
  "nomad/nomad",
  version = "*",
  build = function()
    ---@type nomad.neovim.build
    local build = require("nomad.neovim.build")

    build.builders.download_prebuilt():build(build.contexts.lazy())
  end,
  opts = {},
}
```

Nomad doesn't currently expose any configuration options, so you can set `opts`
to an empty table. See the [configuration](./configuration.md) docs for more
infos.

## Builders

The `build.builders` module contains different "builders", which are recipes
for creating the Nomad binary that Neovim will load. The `download_prebuilt()`
builder downloads pre-built binaries from the GitHub release specified by the
`version` tag.

There are two other builders for those that want to build the plugin from
source:

- `build.builders.cargo()`: builds the plugin using Cargo (needs `cargo` from a
  nightly version of the Rust toolchain in your `$PATH`).
  See [this](./building.md#building-with-cargo) for more infos;

- `build.builders.nix()`: builds the plugin using Nix (needs `nix` in your
  `$PATH`). See [this](./building.md#building-with-nix) for more infos;

Builders can also be composed together using the `:fallback()` method, which
creates a new builder that tries to use the first builder, falling back to the
second if that fails.

For example, you could have this build function:

```lua
function()
  ---@type nomad.neovim.build
  local build = require("nomad.neovim.build")

  build.builders.download_prebuilt()
      :fallback(build.builders.cargo())
      :fallback(build.builders.nix())
      :build(build.contexts.lazy())
end
```

This would build the plugin by first trying to download a pre-built binary,
falling back to building with Cargo if that fails (e.g., if you don't have
access to the internet), falling back to building with Nix if that also fails.

## Contexts

All builders are lazy, and creating one doesn't do anything other than
describing the computation to run. To actually drive the build to completion
you will call the `:build()` method with a *context*.

Don't worry about what those are, but contexts are tied to the specific plugin
manager being used. We currently only support lazy.nvim, so
`build.contexts.lazy()` is the only available context.
