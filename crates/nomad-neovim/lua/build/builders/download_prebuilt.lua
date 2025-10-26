--- Optional parameters for the 'download_prebuilt' builder. Currently unused.
---
---@class (exact) nomad.neovim.build.DownloadPrebuiltOpts

local future = require("nomad.future")

---@type nomad.Result
local Result = require("nomad.result")

---@type nomad.neovim.Command
local Command = require("nomad.neovim.command")

--- All the commands that this builder needs to be in the user's $PATH.
---
---@type table<string, string>
local commands = {
  cp = "cp",
  curl = "curl",
  git = "git",
  mkdir = "mkdir",
  tar = "tar",
}

---@param exit_code integer
---@return string
local err = function(exit_code)
  return ("Builder 'download_prebuilt' failed with exit code %s")
      :format(exit_code)
end

--- NOTE: this has to be kept in sync with the 'mkArchiveName' function in
--- neovim.nix which is responsible for naming the Neovim artifacts published
--- in the releases.
---
--- TODO: either eliminate one of the two sources of truth (not sure how), or
--- add a test in CI that fails if they diverge.
---
---@param tag string
---@return nomad.Result<string, string>
local get_archive_name = function(tag)
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
    :format(tag, neovim_version, os, arch))
end

local get_artifact_url = function(tag, artifact_name)
  return ("https://github.com/nomad/nomad/releases/download/%s/%s")
      :format(tag, artifact_name)
end

---@param opts nomad.neovim.build.DownloadPrebuiltOpts
---@param build_ctx nomad.neovim.build.Context
---@return nomad.future.Future<nomad.Result<nil, string>>
local build_fn = function(opts, build_ctx)
  return future.async(function(ctx)
    ---@type string
    local tag

    local tag_res = Command.new(commands.git)
        :args({ "describe", "--tags", "--exact-match" })
        :current_dir(build_ctx:repo_dir())
        :on_stdout(function(line) tag = line end)
        :on_stderr(build_ctx.notify)
        :await(ctx)

    -- We're not on a tag, so we can't download a pre-built artifact.
    if tag_res:is_err() then return tag_res:map_err(err) end
    if tag == nil then return Result.err("not on a tag") end

    local archive_name_res = get_archive_name(tag)
    -- We don't offer pre-built artifacts for this machine.
    if archive_name_res:is_err() then return archive_name_res:map(function() end) end
    local archive_name = archive_name_res:unwrap()

    -- /result is gitignored, which makes it a good place to store the
    -- downloaded archive under.
    local out_dir = build_ctx:repo_dir():join("result")

    local mkdir_res = Command.new(commands.mkdir)
        :args({ "-p", out_dir:display() })
        :on_stderr(build_ctx.notify)
        :await(ctx)

    if mkdir_res:is_err() then return mkdir_res:map_err(err) end

    local curl_res = Command.new(commands.curl)
        -- Follow redirects.
        :arg("--location")
        :arg("--output")
        :arg(out_dir:join(archive_name):display())
        :arg(get_artifact_url(tag, archive_name))
        :on_stderr(build_ctx.notify)
        :await(ctx)

    if curl_res:is_err() then return curl_res:map_err(err) end

    local tar_res = Command.new(commands.tar)
        :args({ "-xzf", out_dir:join(archive_name):display() })
        :args({ "-C", out_dir:display() })
        :on_stderr(build_ctx.notify)
        :await(ctx)

    if tar_res:is_err() then return tar_res:map_err(err) end

    return Command.new(commands.cp)
        :args({ out_dir:join("/lua/*"):display(), "lua/" })
        :current_dir(build_ctx:repo_dir())
        :await(ctx)
        :on_stderr(build_ctx.notify)
        :map_err(err)
  end)
end

---@type nomad.neovim.build.BuilderSpec
return {
  build_fn = build_fn,
  commands = vim.tbl_values(commands),
}
