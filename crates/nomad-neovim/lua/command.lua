--- Same as Rust's `std::process::Command`, but async.
---@class nomad.neovim.Command

local future = require("nomad.future")
local Option = require("nomad.option")
---@type nomad.Result
local Result = require("nomad.result")

local Command = {}
Command.__index = Command

---@param cmd string
---@return nomad.neovim.Command
Command.new = function(cmd)
  local self = setmetatable({}, Command)
  self._cmd_args = { cmd }
  return self
end

---@param self nomad.neovim.Command
---@param arg string?
---@return nomad.neovim.Command
function Command:arg(arg)
  if arg then table.insert(self._cmd_args, arg) end
  return self
end

---@param args [string]
---@return nomad.neovim.Command
function Command:args(args)
  for _, arg in ipairs(args) do
    table.insert(self._cmd_args, arg)
  end
  return self
end

---@param dir neovim.path.Path
---@return nomad.neovim.Command
function Command:current_dir(dir)
  self._cwd = dir
  return self
end

---@param handler fun(stdout_line: string)
---@return nomad.neovim.Command
function Command:on_stdout(handler)
  self._on_stdout_line = handler
  return self
end

---@param handler fun(stderr_line: string)
---@return nomad.neovim.Command
function Command:on_stderr(handler)
  self._on_stderr_line = handler
  return self
end

---@param self nomad.neovim.Command
---@return nomad.future.Future<nomad.Result<nil, integer>>
function Command:into_future()
  ---@type integer?
  local exit_code = nil

  ---@type fun()
  local wake

  local start = function()
    vim.system(self._cmd_args, {
      cwd = self._cwd and tostring(self._cwd) or nil,
      stdout = function(_, data)
        if not data then return end
        if not self._on_stdout_line then return end
        for line in data:gmatch("([^\n]+)") do
          self._on_stdout_line(line)
        end
      end,
      stderr = function(_, data)
        if not data then return end
        if not self._on_stderr_line then return end
        for line in data:gmatch("([^\n]+)") do
          self._on_stderr_line(line)
        end
      end,
      text = true,
    }, function(out)
      exit_code = out.code
      wake()
    end)
  end

  local is_first_poll = true

  return future.Future.new(function(ctx)
    if exit_code then
      error("called poll() on a Command that has already completed")
    end

    -- Update the waker.
    wake = ctx.wake

    if is_first_poll then
      start()
      is_first_poll = false
    end

    if exit_code then
      local res = exit_code == 0 and Result.ok(nil) or Result.err(exit_code)
      return Option.some(res)
    else
      return Option.none
    end
  end)
end

---@param self nomad.neovim.Command
---@param ctx nomad.future.Context
---@return nomad.Result<nil, integer>
function Command:await(ctx)
  return self:into_future():await(ctx)
end

return Command
