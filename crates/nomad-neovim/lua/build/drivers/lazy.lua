local Context = require("nomad.neovim.build.context")

---@type nomad.neovim.build.Driver
return function(build_fn)
  ---@type nomad.Result<nil, string>?
  local done_res = nil

  ---@type [string]
  local message_queue = {}

  local ctx = Context.new({
    emit = function(msg) message_queue[#message_queue + 1] = msg end,
    on_done = function(res) done_res = res end,
  })

  build_fn(ctx)

  -- Keep yielding until the builder is done. Lazy takes care of scheduling the
  -- next resume() to be called in the next tick of the event loop.
  --
  -- See https://lazy.folke.io/developers#building for more infos.
  while not done_res do
    local maybe_msg = table.remove(message_queue, 1)
    coroutine.yield(maybe_msg)
  end

  -- The builder is done, but display any remaining messages before returning.
  for _, msg in ipairs(message_queue) do
    coroutine.yield(msg)
  end

  if done_res:is_err() then
    error(done_res:unwrap_err())
  end
end
