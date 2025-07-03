---@type [string]
local message_queue = {}

---@type nomad.neovim.build.Driver
return {
  block_on_build = function(build_fut)
    --- Lazy already takes care of scheduling a coroutine.resume() to run in
    --- the next tick of the event loop every time we yield, so we can just use
    --- a no-op waker that does nothing.
    ---
    --- See https://lazy.folke.io/developers#building for more infos.
    ---
    ---@type nomad.future.Context
    local noop_ctx = { wake = function() end }

    ---@type nomad.Result<nil, string>
    local build_res

    -- Keep polling the future until it completes.
    while true do
      local maybe_res = build_fut.poll(noop_ctx)

      if maybe_res:is_some() then
        build_res = maybe_res:unwrap()
        break
      end

      -- Yield with the message in front of the queue (if any), which will
      -- cause it to be displayed in Lazy's UI.
      coroutine.yield(table.remove(message_queue, 1))
    end

    -- The future is done, but display any remaining messages before returning.
    for _, msg in ipairs(message_queue) do
      coroutine.yield(msg)
    end

    if build_res:is_err() then
      error(build_res:unwrap_err())
    end
  end,

  emit = function(message)
    -- Just push the message to the back of the queue, our yield() will take
    -- care of displaying it in the UI.
    message_queue[#message_queue + 1] = message
  end
}
