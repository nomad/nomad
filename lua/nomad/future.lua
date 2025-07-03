--- A future is a unit of lazy, asynchronous computation. It can be polled with
--- a context, and:
---
--- * if the future hasn't yet completed, it must schedule the `ctx.wake`
--- function to be called when it's ready to make some progress;
---
--- * if the future has completed, polling it will return the output of the
---   computation.
--- @class (exact) nomad.future.Future<T>: { poll: fun(self: nomad.future.Future<T>, ctx: nomad.future.Context): T? }

--- TODO: docs.
---
--- @class (exact) nomad.future.Context
--- @field wake fun()

local Future = {}
Future.__index = Future

--- @generic T
--- @param poll fun(ctx: nomad.future.Context): T?
--- @return nomad.future.Future<T>
Future.new = function(poll)
  local self = setmetatable({}, Future)
  self._poll = poll
  return self
end

--- @generic T
--- @param self nomad.future.Future<T>
--- @param ctx nomad.future.Context
--- @return T?
function Future:poll(ctx)
  return self._poll(self, ctx)
end

--- @generic T
--- @param self nomad.future.Future<T>
--- @param ctx nomad.future.Context
--- @return T
function Future:await(ctx)
  while true do
    local maybe_out = self:poll(ctx)
    if maybe_out then
      return maybe_out
    end
    local success = pcall(coroutine.yield)
    if not success then
      error("await() can only be called from within an async block", 0)
    end
  end
end

---@generic T
---@param fun fun(ctx: nomad.future.Context): T
---@return nomad.future.Future<T>
local async = function(fun)
  -- We'll assign this to the right context when the future is polled.
  local ctx

  local thread = coroutine.create(function() return fun(ctx) end)

  local is_done = false

  return Future.new(function(new_ctx)
    if is_done then
      error("called poll() on a Future that has already completed", 0)
    end

    -- Update the context.
    ctx = new_ctx

    local success, output = coroutine.resume(thread)

    -- Rethrow the error, with level=0 to preserve the original error location.
    if not success then error(output, 0) end

    -- The coroutine has completed, and we can return its output.
    if coroutine.status(thread) == "dead" then
      is_done = true
      return output
    end
  end)
end

return {
  async = async,
  Future = Future,
}
