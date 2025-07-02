---@class nomad.neovim.process
---@field command nomad.neovim.process.Command

---@class nomad.neovim.process.Command
---@field new fun(command: string): nomad.neovim.process.Command
---@field arg fun(self: nomad.neovim.process.Command, arg: string): nomad.neovim.process.Command
---@field args fun(self: nomad.neovim.process.Command, args: [string]): nomad.neovim.process.Command
---@field current_dir fun(self: nomad.neovim.process.Command, dir: nomad.path.Path): nomad.neovim.process.Command
---@field on_stdout fun(self: nomad.neovim.process.Command, handler: fun(stdout_line: string)): nomad.neovim.process.Command
---@field on_stderr fun(self: nomad.neovim.process.Command, handler: fun(stdout_line: string)): nomad.neovim.process.Command
---@field on_done fun(self: nomad.neovim.process.Command, handler: fun(res: nomad.Result<nil, integer>): nomad.neovim.process.Command?): nomad.neovim.process.Command?

---@type nomad.result
local result = require("nomad.result")

local command = {}
command.__index = command

---@param cmd string
---@return nomad.neovim.process.Command
command.new = function(cmd)
  local self = {
    cmd = { cmd },
  }
  return setmetatable(self, command)
end

---@param arg string
---@return nomad.neovim.process.Command
function command:arg(arg)
  table.insert(self.cmd, arg)
  return self
end

---@param args [string]
---@return nomad.neovim.process.Command
function command:args(args)
  for _, arg in ipairs(args) do
    table.insert(self.cmd, arg)
  end
  return self
end

---@param dir neovim.path.Path
---@return nomad.neovim.process.Command
function command:current_dir(dir)
  self.cwd = dir
  return self
end

---@param handler fun(stdout_line: string)
---@return nomad.neovim.process.Command
function command:on_stdout(handler)
  self.on_stdout_line = handler
  return self
end

---@param handler fun(stderr_line: string)
---@return nomad.neovim.process.Command
function command:on_stderr(handler)
  self.on_stderr_line = handler
  return self
end

function command:on_done(handler)
  vim.system(self.cmd, {
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
    local res = out.code == 0 and result.ok(nil) or result.err(out.code)
    local maybe_next = handler(res)
    if not maybe_next then return end
    -- TODO: how do we chain commands?
  end)
end
