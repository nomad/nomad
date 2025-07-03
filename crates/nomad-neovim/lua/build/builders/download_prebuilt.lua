---@class (exact) nomad.neovim.build.DownloadPrebuiltOpts

local future = require("nomad.future")

---@type nomad.Result
local Result = require("nomad.result")

---@type nomad.neovim.Command
local Command = require("nomad.neovim.command")

--- NOTE: this has to be kept in sync with the 'mkArchiveName' function in
--- neovim.nix which is responsible for naming the Neovim artifacts published
--- in the releases.
---
--- TODO: either eliminate one of the two sources of truth (not sure how), or
--- add a test in CI that fails if they diverge.
---
---@param nomad_version string
---@return nomad.Result<string, string>
local get_artifact_name = function(nomad_version)
  local arch = ({
    ["x64"] = "x86_64",
    ["arm64"] = "aarch64",
  })[jit.arch]

  if not arch then
    return Result.err(("unsupported architecture: %s"):format(jit.arch))
  end

  local os = ({
    ["linux"] = "linux",
    ["osx"] = "macos",
  })[jit.os:lower()]

  if not os then
    return Result.err(("unsupported OS: %s"):format(jit.os:lower()))
  end

  local version = vim.version()
  local neovim_version = ("%s.%s%s"):format(
    version.major, version.minor, version.prerelease and "-nightly" or ""
  )

  return Result.ok(("nomad-%s-for-neovim-%s-%s-%s.tar.gz")
    :format(nomad_version, neovim_version, os, arch))
end

local get_artifact_url = function(tag, artifact_name)
  return ("https://github.com/nomad/nomad/releases/download/%s/%s")
      :format(tag, artifact_name)
end

---@param opts nomad.neovim.build.DownloadPrebuiltOpts
---@param build_ctx nomad.neovim.build.Context
---@return nomad.future.Future<nomad.Result<nil, string>>
return function(opts, build_ctx)
  return future.async(function(ctx)
    ---@type string
    local tag

    local tag_res = Command.new("git")
        :args({ "describe", "--tags", "--exact-match" })
        :current_dir(build_ctx:repo_dir())
        :on_stdout(function(line) tag = line end)
        :await(ctx)

    -- We're not on a tag, so we can't download a pre-built artifact.
    if tag_res:is_err() then return tag_res:map_err(tostring) end
    if tag == nil then return Result.err("not on a tag") end

    local nomad_version = tag:gsub("^v", "")
    local artifact_res = get_artifact_name(nomad_version)
    -- We don't offer pre-built artifacts for this machine.
    if artifact_res:is_err() then return artifact_res:map(function() end) end

    local artifact_name = artifact_res:unwrap()

    -- /result is gitignored, which makes it a good place to store the
    -- downloaded artifact under.
    local out_dir = build_ctx:repo_dir():join("result")

    local mkdir_res = Command.new("mkdir")
        :args({ "-p", out_dir })
        :await(ctx)

    if mkdir_res:is_err() then return mkdir_res:map_err(tostring) end

    local curl_res = Command.new("curl")
        -- Follow redirects.
        :arg("--location")
        :arg("--output")
        :arg(out_dir:join(artifact_name))
        :arg(get_artifact_url(tag, artifact_name))
        :on_stdout(build_ctx.emit)
        :await(ctx)

    if curl_res:is_err() then return curl_res:map_err(tostring) end

    local tar_res = Command.new("tar")
        :args({ "-xzf", out_dir:join(artifact_name) })
        :args({ "-C", out_dir })
        :await(ctx)

    if tar_res:is_err() then return tar_res:map_err(tostring) end

    return Command.new("cp")
        :args({ out_dir:join("/lua/*"), "lua/" })
        :current_dir(build_ctx:repo_dir())
        :await(ctx)
        :map_err(tostring)
  end)
end
