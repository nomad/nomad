---@class (exact) nomad.neovim.build.Context: nomad.neovim.build.ContextOpts
---
---@field override fun(self: nomad.neovim.build.Context, overrides: table<string, any>): nomad.neovim.build.Context

---@class (exact) nomad.neovim.build.ContextOpts
---
---@field emit fun(msg: string)
---@field on_done fun(res: nomad.Result<nil, string>)

---@type nomad.path
local path = require("nomad.path")

--- @generic T
--- @param list [T]
--- @param start_idx integer
--- @param end_idx integer
--- @return [T]
local slice = function(list, start_idx, end_idx)
  local sliced = {}
  for idx = start_idx, end_idx do
    sliced[#sliced + 1] = list[idx]
  end
  return sliced
end

local Context = {}
Context.__index = Context

---@param opts nomad.neovim.build.ContextOpts
---@return nomad.neovim.build.Context
Context.new = function(opts)
  return setmetatable(opts, Context)
end

---@return nomad.neovim.build.Context
function Context:override(overrides)
  return setmetatable(vim.tbl_extend("force", self, overrides), Context)
end

---@return nomad.path.Path
function Context:repo_dir()
  if not self._repo_dir then
    local src = debug.getinfo(1, "S").source
    if src:sub(1, 1) ~= "@" then
      error("not a in file source")
    end
    local file_components = vim.split(src:sub(2), path.separator)
    local repo_components = slice(file_components, 1, #file_components - 6)
    self._repo_dir = path.Path.from_components(repo_components)
  end
  return self._repo_dir
end

return Context
