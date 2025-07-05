---@class (exact) nomad.neovim.build.CargoOpts

local future = require("nomad.future")

---@type nomad.neovim.Command
local Command = require("nomad.neovim.command")

--- All the commands that this builder needs to be in the user's $PATH.
---
---@type table<string, string>
local commands = {
  cargo = "cargo",
}

---@param exit_code integer
---@return string
local err = function(exit_code)
  return ("Builder 'cargo' failed with exit code %s"):format(exit_code)
end

---@param opts nomad.neovim.build.CargoOpts
---@param build_ctx nomad.neovim.build.Context
---@return nomad.future.Future<nomad.Result<nil, string>>
local build_fn = function(opts, build_ctx)
  return future.async(function(ctx)
    return Command.new(commands.cargo)
        :args({ "xtask", "neovim", "build", "--release" })
        :arg(vim.version().prerelease and "--nightly" or nil)
        :current_dir(build_ctx:repo_dir())
        :on_stdout(build_ctx.notify)
        :on_stderr(build_ctx.notify)
        :await(ctx)
        :map_err(err)
  end)
end

---@type nomad.neovim.build.BuilderSpec
return {
  build_fn = build_fn,
  commands = vim.tbl_values(commands),
}
