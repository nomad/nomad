---@class (exact) nomad.neovim.build.CargoOpts

local future = require("nomad.future")

---@type nomad.neovim.Command
local Command = require("nomad.neovim.command")

---@param opts nomad.neovim.build.CargoOpts
---@param build_ctx nomad.neovim.build.Context
---@return nomad.future.Future<nomad.Result<nil, string>>
return function(opts, build_ctx)
  return future.async(function(ctx)
    return Command.new("cargo")
        :args({ "xtask", "neovim", "build", "--release" })
        :arg(vim.version().prerelease and "--nightly" or nil)
        :current_dir(build_ctx:repo_dir())
        :on_stdout(build_ctx.notify)
        :on_stderr(build_ctx.notify)
        :await(ctx)
        :map_err(tostring)
  end)
end
