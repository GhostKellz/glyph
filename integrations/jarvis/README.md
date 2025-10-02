# Glyph ↔ Jarvis Integration

Jarvis CLI integration for Glyph MCP with interactive consent prompts, tool scopes, and audit logging.

## Features

- **Interactive Consent**: Prompt user before executing sensitive operations
- **Tool Scopes**: Fine-grained permission management (fs.read, fs.write, shell.execute)
- **Policy Engine**: Configurable consent modes (always, once, never, per-tool)
- **Audit Logging**: Track all tool invocations with timestamps
- **Rate Limiting**: Per-tool rate limits
- **CLI Management**: Easy policy configuration and audit log viewing

## Installation

```bash
cd integrations/jarvis
cargo build --release
cargo install --path .
```

## Usage

### 1. Start Jarvis MCP Server

```bash
# Start with stdio transport
jarvis-mcp serve

# Start with WebSocket
jarvis-mcp serve --transport websocket --address 127.0.0.1:7331

# Use custom policy file
jarvis-mcp serve --config ~/.jarvis/custom-policy.toml
```

On first run, Jarvis creates a default policy at `~/.config/jarvis/policy.toml`.

### 2. Policy Configuration

```bash
# Show current policy
jarvis-mcp policy show

# Edit policy
jarvis-mcp policy edit

# Reset to defaults
jarvis-mcp policy reset

# Add tool-specific policy
jarvis-mcp policy add-tool shell_execute --consent true --scopes "shell.execute"
```

### 3. Policy File Format

```toml
# ~/.config/jarvis/policy.toml

consent_mode = "per_tool"  # Options: always, once, never, per_tool

[audit]
enabled = true
log_file = "/var/log/jarvis/audit.log"
include_args = true
include_results = false

[scopes.fs_read]
name = "fs.read"
description = "Read files from filesystem"
permissions = ["read"]

[scopes.fs_write]
name = "fs.write"
description = "Write files to filesystem"
permissions = ["write"]

[scopes.shell_execute]
name = "shell.execute"
description = "Execute shell commands"
permissions = ["execute"]

[tool_policies.shell_execute]
consent_required = true
scopes = ["shell.execute"]

[tool_policies.write_file]
consent_required = true
scopes = ["fs.write"]

[tool_policies.delete_file]
consent_required = true
scopes = ["fs.write"]
```

### 4. Consent Modes

#### Always
Prompt for every tool invocation:

```toml
consent_mode = "always"
```

#### Once Per Session
Prompt once, then remember for session:

```toml
consent_mode = "once"
```

#### Never (Auto-approve)
Never prompt (use with caution):

```toml
consent_mode = "never"
```

#### Per-Tool
Use tool-specific policies:

```toml
consent_mode = "per_tool"

[tool_policies.shell_execute]
consent_required = true

[tool_policies.read_file]
consent_required = false  # No prompt for read operations
```

## Consent Flow

```
Client ──► Jarvis MCP ──► Policy Check ──► [Consent Prompt?] ──► Tool Execution
                                                    │
                                                    ├─ Approved ──► Execute
                                                    │
                                                    └─ Denied  ──► Return Error
```

### Interactive Prompt Example

When a sensitive tool is called:

```
┌─────────────────────────────────────┐
│  Consent Required                   │
├─────────────────────────────────────┤
│  Tool: shell_execute                │
│  Scope: shell.execute               │
│  Command: rm -rf /tmp/test          │
│                                     │
│  ❓ Allow this operation?           │
│                                     │
│  [Y] Yes, once                      │
│  [A] Yes, always for this session  │
│  [N] No, deny                       │
└─────────────────────────────────────┘
```

## Audit Logging

All tool invocations are logged:

```bash
# View recent audit logs
jarvis-mcp audit --tail 50
```

Example audit log entry:

```json
{
  "timestamp": "2025-10-02T12:34:56Z",
  "event": "tool_call",
  "tool": "shell_execute",
  "args": {
    "command": "ls -la"
  },
  "user": "alice",
  "approved": true
}
```

## Integration with Glyph Server

```rust
use glyph::server::Server;
use glyph_jarvis::{ConsentGuard, AuditLogger, JarvisTool, PolicyConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load policy
    let policy = PolicyConfig::default();

    // Create guard and logger
    let guard = Arc::new(ConsentGuard::new(policy.clone()));
    let audit_logger = Arc::new(AuditLogger::new(policy.audit.clone()));

    // Build server
    let server = Server::builder()
        .for_stdio()
        .await?;

    // Wrap tools with Jarvis consent and auditing
    // (tools would be wrapped individually)

    server.run().await
}
```

## Rate Limiting

Configure per-tool rate limits:

```toml
[tool_policies.openai_chat]
consent_required = false

[tool_policies.openai_chat.rate_limit]
max_calls = 10
per_seconds = 60  # 10 calls per minute
```

## Security Best Practices

1. **Enable Consent for Destructive Operations**:
   ```toml
   [tool_policies.delete_file]
   consent_required = true
   ```

2. **Enable Audit Logging**:
   ```toml
   [audit]
   enabled = true
   log_file = "/var/log/jarvis/audit.log"
   ```

3. **Use Scopes to Limit Permissions**:
   ```toml
   [tool_policies.my_tool]
   scopes = ["fs.read"]  # Read-only, no write
   ```

4. **Review Audit Logs Regularly**:
   ```bash
   jarvis-mcp audit --tail 100 | grep "error"
   ```

## Example Workflows

### Development Environment

```toml
# ~/.config/jarvis/dev-policy.toml
consent_mode = "once"

[tool_policies.shell_execute]
consent_required = true

[tool_policies.write_file]
consent_required = false  # Auto-approve in dev
```

### Production Environment

```toml
# /etc/jarvis/prod-policy.toml
consent_mode = "per_tool"

[audit]
enabled = true
log_file = "/var/log/jarvis/audit.log"
include_args = true
include_results = true

[tool_policies.shell_execute]
consent_required = true

[tool_policies.delete_file]
consent_required = true
```

## See Also

- [Integration Contract](../../docs/INTEGRATION_CONTRACT.md)
- [Policy Engine Documentation](./POLICY.md)
- [Audit Logging Guide](./AUDIT.md)
