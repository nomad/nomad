---@class (exact) nomad.neovim.build.Builder
---
---Something.
---@field command string something
---
---Something.
---@field fallback fun(builder: nomad.neovim.build.Builder): nomad.neovim.build.Builder

---@class (exact) nomad.neovim.build.Builders
---
---Something.
---@field cargo fun(opts: nomad.neovim.build.CargoOpts?): nomad.neovim.build.Builder
---
---Something.
---@field download_prebuilt fun(opts: nomad.neovim.build.DownloadPrebuiltOpts?): nomad.neovim.build.Builder
---
---Something.
---@field nix fun(opts: nomad.neovim.build.NixOpts?): nomad.neovim.build.Builder

---@type nomad.neovim.build.Builders
return {
  cargo = require("nomad.neovim.build.cargo"),
  download_prebuilt = require("nomad.neovim.build.download_prebuilt"),
  nix = require("nomad.neovim.build.nix"),
}
