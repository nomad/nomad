---@class (exact) nomad.neovim.build.Builder
---
---Build with the given driver.
---@field build fun(self: nomad.neovim.build.Builder, driver: nomad.neovim.build.Driver)
---
---Fallback.
---@field fallback fun(self: nomad.neovim.build.Builder, fallback_builder: nomad.neovim.build.Builder): nomad.neovim.build.Builder

local Builder = {}
Builder.__index = Builder

---@param build_fn fun(ctx: nomad.neovim.build.Context)
---@return nomad.neovim.build.Builder
Builder.new = function(build_fn)
  local self = {
    build_fn = build_fn,
  }
  return setmetatable(self, Builder)
end

---@param self nomad.neovim.build.Builder
---@param driver nomad.neovim.build.Driver
function Builder:build(driver)
  driver(self.build_fn)
end

---@param self nomad.neovim.build.Builder
---@param fallback_builder nomad.neovim.build.Builder
---@return nomad.neovim.build.Builder
function Builder:fallback(fallback_builder)
  return Builder.new(function(ctx)
    self.build_fn(ctx:override({
      on_done = function(res)
        if res:is_err() then
          ctx.emit(res:unwrap_err())
          fallback_builder.build_fn(ctx)
        end
      end
    }))
  end)
end

return Builder
