-- Neovim configuration for Glyph MCP
-- Add this to your init.lua or a plugin config file

local M = {}

-- Glyph MCP client configuration
M.setup = function()
  -- Configure MCP servers
  local mcp_servers = {
    glyph = {
      cmd = { "glyph", "serve", "--transport", "stdio" },
      env = {
        RUST_LOG = "info",
      },
    },
    ["glyph-custom"] = {
      cmd = { "/path/to/your/custom-server", "serve", "--transport", "stdio" },
      env = {
        RUST_LOG = "debug",
      },
    },
  }

  -- Start Glyph MCP server
  vim.api.nvim_create_user_command("GlyphStart", function()
    local Job = require("plenary.job")

    Job:new({
      command = "glyph",
      args = { "serve", "--transport", "stdio" },
      on_stdout = function(_, data)
        vim.notify("Glyph: " .. data, vim.log.levels.INFO)
      end,
      on_stderr = function(_, data)
        vim.notify("Glyph Error: " .. data, vim.log.levels.ERROR)
      end,
      on_exit = function(j, return_val)
        if return_val == 0 then
          vim.notify("Glyph MCP server started", vim.log.levels.INFO)
        else
          vim.notify("Glyph MCP server failed to start", vim.log.levels.ERROR)
        end
      end,
    }):start()
  end, {})

  -- Stop Glyph MCP server
  vim.api.nvim_create_user_command("GlyphStop", function()
    vim.fn.jobstop(vim.g.glyph_job_id)
    vim.notify("Glyph MCP server stopped", vim.log.levels.INFO)
  end, {})

  -- Call a Glyph tool
  vim.api.nvim_create_user_command("GlyphCall", function(opts)
    local tool = opts.fargs[1]
    local args_json = opts.fargs[2] or "{}"

    local request = string.format(
      '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"%s","arguments":%s}}',
      tool,
      args_json
    )

    vim.fn.system("echo '" .. request .. "' | glyph serve --transport stdio")
  end, { nargs = "+" })
end

return M
