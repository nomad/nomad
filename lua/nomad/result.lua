---@class nomad.Result<T, E>: { value: T?, error: E? }

local Result = {}
Result.__index = Result

---@generic T
---@param value T
---@return nomad.Result<T, any>
Result.ok = function(value)
  local self = {
    value = value,
    error = nil,
  }
  return setmetatable(self, Result)
end

---@generic E
---@param error E
---@return nomad.Result<any, E>
Result.err = function(error)
  local self = {
    value = nil,
    error = error,
  }
  return setmetatable(self, Result)
end

function Result:is_ok()
  return self.value ~= nil
end

function Result:is_err()
  return self.error ~= nil
end

function Result:map(fun)
  return self:is_err() and self or Result.ok(fun(self:unwrap()))
end

function Result:map_err(fun)
  return self:is_ok() and self or Result.err(fun(self:unwrap_err()))
end

function Result:unwrap()
  if self:is_ok() then
    return self.value
  else
    error("called `unwrap()` on an error value")
  end
end

function Result:unwrap_err()
  if self:is_err() then
    return self.error
  else
    error("called `unwrap_err()` on an ok value")
  end
end

return Result
