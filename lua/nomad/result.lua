---@class nomad.Result<T, E>: { _is_ok: boolean, _value: T?, _error: E? }

local Result = {}
Result.__index = Result

---@generic T
---@param value T
---@return nomad.Result<T, any>
Result.ok = function(value)
  local self = setmetatable({}, Result)
  self._is_ok = true
  self._value = value
  return self
end

---@generic E
---@param error E
---@return nomad.Result<any, E>
Result.err = function(error)
  local self = setmetatable({}, Result)
  self._is_ok = false
  self._error = error
  return self
end

function Result:is_ok()
  return self._is_ok
end

function Result:is_err()
  return not self:is_ok()
end

function Result:map(fun)
  return self:is_err() and self or Result.ok(fun(self:unwrap()))
end

function Result:map_err(fun)
  return self:is_ok() and self or Result.err(fun(self:unwrap_err()))
end

function Result:unwrap()
  if self:is_ok() then
    return self._value
  else
    error("called Result:unwrap() on an error value", 2)
  end
end

function Result:unwrap_err()
  if self:is_err() then
    return self._error
  else
    error("called Result:unwrap_err() on an ok value", 2)
  end
end

return Result
