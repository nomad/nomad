--- A future is a unit of lazy, asynchronous computation. It can be polled with
--- a context, and:
---
--- * if the future hasn't yet completed, it must schedule the `ctx.wake`
--- function to be called when it's ready to make some progress;
---
--- * if the future has completed, polling it will return the output of the
---   computation.
---@class (exact) nomad.future.Future<T>: { poll: fun(ctx: nomad.future.Context): nomad.Option<T> }

--- TODO: docs.
---
---@class (exact) nomad.future.Context
---@field wake fun()

local Option = require("nomad.option")

local Future = {}
Future.__index = Future

---@generic T
---@param poll fun(ctx: nomad.future.Context): nomad.Option<T>
---@return nomad.future.Future<T>
Future.new = function(poll)
  local self = setmetatable({}, Future)
  self.poll = poll
  return self
end

---@generic T
---@param self nomad.future.Future<T>
---@param ctx nomad.future.Context
---@return T
function Future:await(ctx)
  while true do
    local maybe_out = self.poll(ctx)
    if maybe_out:is_some() then
      return maybe_out:unwrap()
    end
    local success, next_ctx = pcall(coroutine.yield)
    if not success or not type(next_ctx) == "table" then
      error("await() can only be called from within an async block", 2)
    end
    ctx = next_ctx
  end
end

---@generic T
---@param fun fun(ctx: nomad.future.Context): T
---@return nomad.future.Future<T>
local async = function(fun)
  -- We'll initialize this with the context given to the first poll of the
  -- future we're about to create.
  ---@type nomad.future.Context
  local first_ctx

  local thread = coroutine.create(function() return fun(first_ctx) end)

  local is_done = false
  local is_first_poll = true

  return Future.new(function(ctx)
    if is_done then
      error("called poll() on a Future that has already completed", 2)
    end

    if is_first_poll then
      first_ctx = ctx
      is_first_poll = false
    end

    local success, output = coroutine.resume(thread, ctx)

    -- Rethrow the error, with level=0 to preserve the original error location.
    if not success then error(output, 0) end

    -- The coroutine has completed, and we can return its output.
    if coroutine.status(thread) == "dead" then
      is_done = true
      return Option.some(output)
    else
      return Option.none
    end
  end)
end

return {
  async = async,
  Future = Future,
}
