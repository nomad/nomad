---@class nomad.path.Path

local separator = package.config:sub(1, 1)

local Path = {}
Path.__index = Path

Path.__tostring = function(self)
  return self._path
end

---@param components [string]
---@return nomad.path.Path
Path.from_components = function(components)
  local self = setmetatable({}, Path)
  self._path = separator .. table.concat(components, separator)
  return self
end

---@param self nomad.path.Path
---@return string
function Path:display()
  return self._path
end

---@param self nomad.path.Path
---@param file_name string
---@return nomad.path.Path
function Path:join(file_name)
  self._path = self._path .. separator .. file_name
  return self
end

return {
  separator = separator,
  Path = Path,
}
