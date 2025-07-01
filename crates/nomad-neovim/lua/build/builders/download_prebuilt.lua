---@class (exact) nomad.neovim.build.DownloadPrebuiltOpts

---@type nomad.neovim.Process
local process = require("nomad.neovim.build.process")

---@type nomad.result.ResultModule
local result = require("nomad.result")

---@param nomad_version string
---@param neovim_version string
---@return nomad.Result<string, string>
local artifact_name = function(nomad_version, neovim_version)
  local arch = ({
    ["x64"] = "x86_64",
    ["arm64"] = "aarch64",
  })[jit.arch]

  if not arch then
    return result.err(("unsupported architecture: %s"):format(jit.arch))
  end

  local os = ({
    ["linux"] = "linux",
    ["osx"] = "macos",
  })[jit.os:lower()]

  if not os then
    return result.err(("unsupported OS: %s"):format(jit.os:lower()))
  end

  return result.ok(("nomad-%s-for-neovim-%s-%s-%s.tar.gz")
    :format(nomad_version, neovim_version, os, arch))
end

local artifact_url = function(tag, artifact_name)
  return ("https://github.com/nomad/nomad/releases/download/%s/%s")
      :format(tag, artifact_name)
end

---@param opts nomad.neovim.build.DownloadPrebuiltOpts?
---@param ctx nomad.neovim.build.Context
return function(opts, ctx)
  ---@type string?
  local tag = nil

  process.command.new("git")
      :args({ "describe", "--tags", "--exact-match" })
      :current_dir(ctx.repo_dir())
      :on_stdout(function(line) tag = line end)
      :on_done(function(res)
        -- We're not on a tag, so we can't download a pre-built artifact.
        if res:is_err() then return ctx.on_done(res) end
        if tag == nil then return ctx.on_done(result.err("not on a tag")) end

        -- We don't offer pre-built artifacts for this machine.
        local nomad_version = tag:gsub("^v", "")
        local res = artifact_name(nomad_version, ctx.neovim_version())
        if res:is_err() then return ctx.on_done(res) end

        local artifact_name = res:unwrap()
        local url = artifact_url(tag, artifact_name)
        local out_dir = ctx.repo_dir():join("result")

        -- Download the artifact from the releases page.
        local command = process.command.new("curl")
            -- Follow redirects.
            :arg("--location")
            :arg("--output")
            :arg(out_dir:join(artifact_name))
            :arg(url)
            :on_stdout(ctx.emit)

        return command, out_dir, artifact_name
      end)
      :on_done(function(res, out_dir, artifact_name)
        if res:is_err() then return ctx.on_done(res) end

        local command = process.command.new("tar")
            :args({ "-xzf", out_dir:join(artifact_name) })
            :args({ "-C", out_dir })

        return command, out_dir
      end)
      :on_done(function(res, out_dir)
        if res:is_err() then return ctx.on_done(res) end

        return process.command.new("cp")
            :args({ out_dir:join("/lua/*"), "lua/" })
            :current_dir(ctx.repo_dir())
      end)
      :on_done(function(res)
        ctx.on_done(res:map(tostring))
      end)
end
