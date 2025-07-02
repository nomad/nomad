---@class (exact) nomad.neovim.build.builders
---
---Build the plugin from source with Cargo (needs the Nightly toolchain to be
---installed).
---@field cargo fun(opts: nomad.neovim.build.CargoOpts?): nomad.neovim.build.Builder
---
---Download a prebuilt binary for this machine from GitHub releases.
---@field download_prebuilt fun(opts: nomad.neovim.build.DownloadPrebuiltOpts?): nomad.neovim.build.Builder
---
---Build the plugin from source with Nix.
---@field nix fun(opts: nomad.neovim.build.NixOpts?): nomad.neovim.build.Builder

---@type nomad.neovim.build.Builder
local Builder = require("nomad.neovim.build.builder")

---@type nomad.neovim.build.builders
return {
  cargo = function(opts)
    local build_fn = require("nomad.neovim.build.builders.cargo")
    return Builder.new(function(ctx) return build_fn(opts, ctx) end)
  end,
  download_prebuilt = function(opts)
    local build_fn = require("nomad.neovim.build.builders.download_prebuilt")
    return Builder.new(function(ctx) return build_fn(opts, ctx) end)
  end,
  nix = function(opts)
    local build_fn = require("nomad.neovim.build.builders.nix")
    return Builder.new(function(ctx) return build_fn(opts, ctx) end)
  end
}
