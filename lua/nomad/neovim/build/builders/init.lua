---@class (exact) nomad.neovim.build.builders
---
--- Build the plugin from source with Cargo (needs the Nightly toolchain to be
--- installed).
---@field cargo fun(opts: nomad.neovim.build.CargoOpts?): nomad.neovim.build.Builder
---
--- Download a prebuilt binary for this machine from GitHub releases.
---@field download_prebuilt fun(opts: nomad.neovim.build.DownloadPrebuiltOpts?): nomad.neovim.build.Builder
---
--- Build the plugin from source with Nix.
---@field nix fun(opts: nomad.neovim.build.NixOpts?): nomad.neovim.build.Builder

---@class (exact) nomad.neovim.build.BuilderSpec
---
---@field build_fn fun(opts: table<string, any>, ctx: nomad.neovim.build.Context): nomad.future.Future<nomad.Result<nil, string>>
---@field commands [string]

local future = require("nomad.future")

---@type nomad.Result
local Result = require("nomad.result")

---@type nomad.neovim.Command
local Command = require("nomad.neovim.command")

---@type nomad.neovim.build.Builder
local Builder = require("nomad.neovim.build.builder")

---@param commands [string]
---@return nomad.Result<nil, string>
local check_commands_in_path = function(commands)
  local missing_commands = vim.tbl_filter(function(command)
    return not Command.is_in_path(command)
  end, commands)

  if #missing_commands == 0 then
    return Result.ok(nil)
  else
    -- Sort the missing commands alphabetically to make the error message
    -- stable across runs.
    table.sort(missing_commands)

    return Result.err(("command%s not in $PATH: %s")
      :format(
        #missing_commands == 1 and "" or "s",
        table.concat(missing_commands, ", ")
      ))
  end
end

---@param spec nomad.neovim.build.BuilderSpec
---@param opts table<string, any>
---@return nomad.neovim.build.BuildFn
local make_build_fn = function(spec, opts)
  return function(build_ctx)
    return future.async(function(ctx)
      local commands_res = check_commands_in_path(spec.commands)
      if commands_res:is_err() then return commands_res end
      return spec.build_fn(opts, build_ctx):await(ctx)
    end)
  end
end

---@type nomad.neovim.build.builders
return {
  cargo = function(opts)
    local spec = require("nomad.neovim.build.builders.cargo")
    return Builder.new(make_build_fn(spec, opts or {}))
  end,
  download_prebuilt = function(opts)
    local spec = require("nomad.neovim.build.builders.download_prebuilt")
    return Builder.new(make_build_fn(spec, opts or {}))
  end,
  nix = function(opts)
    local spec = require("nomad.neovim.build.builders.nix")
    return Builder.new(make_build_fn(spec, opts or {}))
  end
}
