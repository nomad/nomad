---@class (exact) nomad.neovim.build.CargoOpts

---@type nomad.neovim.Process
local process = require("nomad.neovim.build.process")

---@type nomad.result.ResultModule
local result = require("nomad.result")

---@param opts nomad.neovim.build.CargoOpts?
---@param ctx nomad.neovim.build.Context
return function(opts, ctx)
  process.command.new("cargo")
      :args({ "xtask", "neovim", "build", "--release" })
      :arg(ctx.is_nightly() and "--nightly" or nil)
      :current_dir(ctx.repo_dir())
      :on_stdout(ctx.emit)
      :on_stderr(ctx.emit)
      :on_done(function(res)
        ctx.on_done(res:map(tostring))
      end)
end
