---@class (exact) nomad.neovim.build.NixOpts

---@type nomad.neovim.Process
local process = require("nomad.neovim.build.process")

---@type nomad.result.ResultModule
local result = require("nomad.result")

---@param opts nomad.neovim.build.NixOpts?
---@param ctx nomad.neovim.build.Context
return function(opts, ctx)
  process.command.new("nix")
      :arg("build")
      :arg(".#neovim" .. (ctx.is_nightly() and "-nightly" or ""))
      :arg("--accept-flake-config")
      :current_dir(ctx.repo_dir())
      :on_stdout(ctx.emit)
      :on_stderr(ctx.emit)
      :on_done(function(res)
        if res:is_err() then return ctx.on_done(res) end

        return process.command.new("cp")
            :args({ "result/lua/*", "lua/" })
            :current_dir(ctx.repo_dir())
      end)
      :on_done(function(res)
        ctx.on_done(res:map(tostring))
      end)
end
