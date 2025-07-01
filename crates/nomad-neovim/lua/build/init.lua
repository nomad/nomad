---@class (exact) nomad.neovim.Build
---
---Something.
---@field build fun(builder: nomad.neovim.build.Builder)
---
---Something else.
---@field builders nomad.neovim.build.Builders

---@type nomad.neovim.Build
return {
  build = function(builder)
    print(vim.inspect(builder))
  end,
  builders = require("nomad.neovim.build.builders"),
}
