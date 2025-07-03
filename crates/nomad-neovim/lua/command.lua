---@class nomad.neovim.Command
---@field new fun(command: string): nomad.neovim.Command
---@field arg fun(self: nomad.neovim.Command, arg: string): nomad.neovim.Command
---@field args fun(self: nomad.neovim.Command, args: [string]): nomad.neovim.Command
---@field current_dir fun(self: nomad.neovim.Command, dir: nomad.path.Path): nomad.neovim.Command
---@field on_stdout fun(self: nomad.neovim.Command, handler: fun(stdout_line: string)): nomad.neovim.Command
---@field on_stderr fun(self: nomad.neovim.Command, handler: fun(stdout_line: string)): nomad.neovim.Command
---@field on_done fun(self: nomad.neovim.Command, handler: fun(res: nomad.Result<nil, integer>): nomad.neovim.Command?): nomad.neovim.Command?

local future = require("nomad.future")
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
  self.cwd = dir
  return self
end

---@param handler fun(stdout_line: string)
---@return nomad.neovim.Command
function Command:on_stdout(handler)
  self.on_stdout_line = handler
  return self
end

---@param handler fun(stderr_line: string)
---@return nomad.neovim.Command
function Command:on_stderr(handler)
  self.on_stderr_line = handler
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
      cwd = tostring(self.cwd),
      stdout = function(_, data)
        if not data then return end
        if not self.on_stdout_line then return end
        for line in data:gmatch("([^\n]+)") do
          self.on_stdout_line(line)
        end
      end,
      stderr = function(_, data)
        if not data then return end
        if not self.on_stderr_line then return end
        for line in data:gmatch("([^\n]+)") do
          self.on_stderr_line(line)
        end
      end,
      text = true,
    }, function(out)
      exit_code = out.code
      wake()
    end)
  end

  local has_completed = false
  local has_started = false

  return future.Future.new(function(ctx)
    if has_completed then
      error("called poll() on a Command that has already completed")
    end

    -- Update the waker.
    wake = ctx.wake
    if not has_started then start() end
    has_started = true
    if not exit_code then return end
    has_completed = true
    return exit_code == 0 and Result.ok(nil) or Result.err(exit_code)
  end)
end

return Command
