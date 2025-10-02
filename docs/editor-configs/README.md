# Editor Integration Configurations

Configuration snippets for integrating Glyph MCP with popular editors and AI assistants.

## VS Code (Claude Dev, Cline, Continue)

### Claude Dev / Cline

Add to your MCP settings file (typically `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json` on macOS or `%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json` on Windows):

```json
{
  "mcpServers": {
    "glyph": {
      "command": "glyph",
      "args": ["serve", "--transport", "stdio"]
    }
  }
}
```

See [cline_mcp_settings.json](./cline_mcp_settings.json) for full example.

### Continue.dev

Add to `.continue/config.json`:

```json
{
  "mcpServers": {
    "glyph": {
      "command": "glyph",
      "args": ["serve", "--transport", "stdio"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Neovim

### Using Plenary.nvim

Add to your `init.lua`:

```lua
-- Load Glyph MCP integration
require('glyph').setup()

-- Start Glyph server
vim.cmd('GlyphStart')
```

See [neovim-glyph.lua](./neovim-glyph.lua) for full implementation.

### Using nvim-lspconfig style

```lua
local mcp = require('mcp')

mcp.setup({
  servers = {
    glyph = {
      cmd = { 'glyph', 'serve', '--transport', 'stdio' },
      filetypes = { '*' },
    },
  },
})
```

## Cursor

Add to Cursor settings (`File > Preferences > Settings > MCP`):

```json
{
  "mcpServers": {
    "glyph": {
      "command": "glyph",
      "args": ["serve", "--transport", "stdio"]
    }
  }
}
```

## Zed

Add to `~/.config/zed/settings.json`:

```json
{
  "language_models": {
    "mcp_servers": {
      "glyph": {
        "command": "glyph",
        "args": ["serve", "--transport", "stdio"]
      }
    }
  }
}
```

## Custom Integration Example

For your own MCP client:

```javascript
import { spawn } from 'child_process';
import { JSONRPCClient } from 'json-rpc-2.0';

// Start Glyph server
const glyphProcess = spawn('glyph', ['serve', '--transport', 'stdio']);

// Create JSON-RPC client
const client = new JSONRPCClient((request) => {
  glyphProcess.stdin.write(JSON.stringify(request) + '\n');
});

// Handle responses
glyphProcess.stdout.on('data', (data) => {
  const lines = data.toString().split('\n').filter(Boolean);
  lines.forEach((line) => {
    const response = JSON.parse(line);
    client.receive(response);
  });
});

// Initialize MCP session
await client.request('initialize', {
  protocolVersion: '2024-11-05',
  capabilities: {},
  clientInfo: {
    name: 'my-client',
    version: '1.0.0',
  },
});

// Call a tool
const result = await client.request('tools/call', {
  name: 'calculator',
  arguments: {
    operation: 'add',
    a: 5,
    b: 3,
  },
});

console.log(result);
```

## Verification

Test your configuration:

```bash
# Check Glyph is installed
glyph --version

# Test stdio transport
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | glyph serve --transport stdio

# Should return an initialize response
```

## Troubleshooting

### Server not starting

1. Check Glyph is in PATH:
   ```bash
   which glyph
   ```

2. Check permissions:
   ```bash
   chmod +x $(which glyph)
   ```

3. Test manually:
   ```bash
   glyph serve --transport stdio
   # Should wait for input
   ```

### No tools available

1. Check server initialized:
   ```json
   {"jsonrpc":"2.0","id":2,"method":"tools/list"}
   ```

2. Check logs:
   ```bash
   RUST_LOG=debug glyph serve --transport stdio
   ```

## Next Steps

- [Installation Guide](../installation.md)
- [Integration Contract](../INTEGRATION_CONTRACT.md)
- [Features Guide](../FEATURES.md)
