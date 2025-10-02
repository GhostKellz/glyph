# Rune Integration Guide

**Integrating Glyph MCP with Rune (Zig)**

This guide explains how to integrate Glyph's Rust MCP server with Rune's high-performance Zig tools through the FFI layer.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Building the Integration](#building-the-integration)
- [FFI Interface Reference](#ffi-interface-reference)
- [Example: Calling Rune from Glyph](#example-calling-rune-from-glyph)
- [Example: Calling Glyph from Rune](#example-calling-glyph-from-rune)
- [Memory Management](#memory-management)
- [Error Handling](#error-handling)
- [Performance Considerations](#performance-considerations)
- [Troubleshooting](#troubleshooting)

---

## Overview

**Glyph** provides the MCP protocol implementation in Rust (server, client, transports), while **Rune** provides high-performance code manipulation tools in Zig (text selection, workspace ops, diagnostics).

The integration works through a **C ABI** compatibility layer:

```
┌─────────────────┐         ┌─────────────────┐
│  Glyph (Rust)   │         │   Rune (Zig)    │
│  MCP Protocol   │◄───────►│  Code Tools     │
│  Server/Client  │  C ABI  │  (librune.a)    │
└─────────────────┘         └─────────────────┘
```

**Use cases:**
- Expose Rune's Zig tools as MCP tools in Glyph server
- Call Glyph MCP client from Zig applications
- Build hybrid Rust+Zig applications with MCP support

---

## Architecture

### Glyph Side (Rust)

Glyph exposes:
- `glyph::ffi` module with C-compatible types
- FFI helper functions for string/JSON conversion
- Error codes following C conventions

### Rune Side (Zig)

Rune exposes (from your implementation):
- `selection.zig` - Text selection engine
- `workspace.zig` - Workspace operations
- `diagnostics.zig` - Diagnostics engine
- C ABI compatible functions in `librune.a`

### Integration Layer

```
Rust (Glyph)          C ABI           Zig (Rune)
─────────────────────────────────────────────────
Tool::call()     ──►  FFI call   ──►  rune_execute()
                 ◄──  JSON return ◄──  result
```

---

## Prerequisites

### Rust Requirements

- Rust 1.75+ (MSRV)
- Glyph crate installed (see [Installation](installation.md))

```toml
[dependencies]
glyph = { git = "https://github.com/ghostkellz/glyph", tag = "v0.1.0" }
```

### Zig Requirements

- Zig 0.11+ or 0.12+
- Rune library built (`librune.a`)

```bash
# Build Rune
cd /path/to/rune
zig build -Doptimize=ReleaseFast

# Verify librune.a exists
ls zig-out/lib/librune.a
```

### Linker Setup

Add to your Rust project's `build.rs`:

```rust
// build.rs
fn main() {
    // Link against Rune static library
    println!("cargo:rustc-link-search=native=/path/to/rune/zig-out/lib");
    println!("cargo:rustc-link-lib=static=rune");

    // Link against C++ stdlib (if Rune needs it)
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
```

---

## Building the Integration

### Step 1: Build Rune

```bash
cd ~/projects/rune
zig build -Doptimize=ReleaseFast
```

### Step 2: Build Glyph with FFI feature

```bash
cd ~/projects/glyph
cargo build --release --features ffi
```

### Step 3: Link Together

Create a Rust project that uses both:

```rust
// main.rs
use glyph::server::{Server, Tool};
use glyph::ffi::strings;

// Import Rune FFI functions
extern "C" {
    fn rune_init() -> i32;
    fn rune_execute(tool: *const i8, args: *const i8) -> *mut i8;
    fn rune_free(ptr: *mut i8);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Rune
    unsafe { rune_init() };

    // Start Glyph server
    let mut server = Server::builder()
        .transport_websocket("127.0.0.1:7331")
        .build()
        .await?;

    // Register Rune tools as MCP tools
    server.register(RuneSelectionTool);
    server.register(RuneWorkspaceTool);
    server.register(RuneDiagnosticsTool);

    server.run().await
}
```

---

## FFI Interface Reference

### Error Codes

```c
// glyph/src/ffi.rs

typedef enum {
    FFI_SUCCESS = 0,
    FFI_INVALID_ARGUMENT = -1,
    FFI_OUT_OF_MEMORY = -2,
    FFI_NOT_FOUND = -3,
    FFI_EXECUTION_FAILED = -4,
    FFI_VERSION_MISMATCH = -5,
    FFI_THREAD_SAFETY_VIOLATION = -6,
    FFI_IO_ERROR = -7,
    FFI_PERMISSION_DENIED = -8,
    FFI_TIMEOUT = -9,
    FFI_NOT_IMPLEMENTED = -10,
    FFI_UNKNOWN_ERROR = -99,
} FfiError;
```

### Version Info

```c
typedef struct {
    int major;
    int minor;
    int patch;
} FfiVersion;
```

### Result Type

```c
typedef struct {
    int error;           // FfiError code
    const char* data;    // JSON string (null-terminated)
} FfiResult;
```

---

## Example: Calling Rune from Glyph

### Rust Side: Wrap Rune tool as MCP tool

```rust
use glyph::server::{Tool, ToolContext};
use glyph::protocol::{CallToolResult, Content, ToolInputSchema};
use glyph::ffi::strings;
use std::ffi::CString;
use serde_json::Value;
use std::collections::HashMap;

extern "C" {
    fn rune_workspace_scan(args_json: *const i8) -> *mut i8;
    fn rune_free_string(ptr: *mut i8);
}

struct RuneWorkspaceTool;

#[async_trait::async_trait]
impl Tool for RuneWorkspaceTool {
    fn name(&self) -> &str {
        "workspace_scan"
    }

    fn description(&self) -> Option<&str> {
        Some("Fast workspace scanning powered by Rune (Zig)")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut props = HashMap::new();
        props.insert("path".to_string(), serde_json::json!({
            "type": "string",
            "description": "Workspace directory path"
        }));

        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(props),
            required: Some(vec!["path".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, _ctx: &ToolContext, args: Option<Value>) -> glyph::Result<CallToolResult> {
        let args = args.ok_or_else(|| glyph::Error::Protocol("Missing args".into()))?;

        // Convert to JSON string for FFI
        let args_cstr = strings::value_to_json_string(&args)
            .map_err(|_| glyph::Error::Protocol("Failed to serialize args".into()))?;

        // Call Rune
        let result_ptr = unsafe { rune_workspace_scan(args_cstr.as_ptr()) };

        if result_ptr.is_null() {
            return Err(glyph::Error::Protocol("Rune returned null".into()));
        }

        // Convert result back to Rust
        let result_str = unsafe { strings::cstr_to_string(result_ptr) }
            .map_err(|_| glyph::Error::Protocol("Failed to read Rune result".into()))?;

        let result_value = strings::json_string_to_value(&result_str)
            .map_err(|_| glyph::Error::Protocol("Failed to parse Rune result".into()))?;

        // Free Rune's allocated memory
        unsafe { rune_free_string(result_ptr) };

        Ok(CallToolResult {
            content: vec![Content::text(serde_json::to_string_pretty(&result_value).unwrap())],
            is_error: None,
            meta: Some(serde_json::json!({
                "engine": "rune",
                "language": "zig"
            })),
        })
    }
}
```

---

## Example: Calling Glyph from Rune

### Zig Side: Call Glyph MCP client

```zig
// rune_glyph_client.zig

const std = @import("std");

// Import Glyph FFI functions
extern fn glyph_client_new(url: [*:0]const u8) ?*anyopaque;
extern fn glyph_client_call_tool(client: *anyopaque, tool: [*:0]const u8, args: [*:0]const u8) ?[*:0]u8;
extern fn glyph_client_free(client: *anyopaque) void;
extern fn glyph_free_string(ptr: [*:0]u8) void;

pub const GlyphClient = struct {
    handle: *anyopaque,

    pub fn connect(allocator: std.mem.Allocator, url: []const u8) !GlyphClient {
        const url_z = try allocator.dupeZ(u8, url);
        defer allocator.free(url_z);

        const handle = glyph_client_new(url_z.ptr) orelse return error.ConnectionFailed;

        return GlyphClient{ .handle = handle };
    }

    pub fn callTool(self: *GlyphClient, allocator: std.mem.Allocator, tool: []const u8, args: anytype) ![]const u8 {
        const tool_z = try allocator.dupeZ(u8, tool);
        defer allocator.free(tool_z);

        const args_json = try std.json.stringifyAlloc(allocator, args, .{});
        defer allocator.free(args_json);

        const args_z = try allocator.dupeZ(u8, args_json);
        defer allocator.free(args_z);

        const result_ptr = glyph_client_call_tool(self.handle, tool_z.ptr, args_z.ptr) orelse return error.CallFailed;
        defer glyph_free_string(result_ptr);

        const result_len = std.mem.len(result_ptr);
        return try allocator.dupe(u8, result_ptr[0..result_len]);
    }

    pub fn deinit(self: *GlyphClient) void {
        glyph_client_free(self.handle);
    }
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    var client = try GlyphClient.connect(allocator, "ws://localhost:7331");
    defer client.deinit();

    const result = try client.callTool(allocator, "read_file", .{
        .path = "/etc/hosts",
    });
    defer allocator.free(result);

    std.debug.print("Result: {s}\n", .{result});
}
```

---

## Memory Management

### Rules

1. **Rust allocates, Rust frees**: Strings returned from Glyph must be freed by Glyph
2. **Zig allocates, Zig frees**: Strings returned from Rune must be freed by Rune
3. **No shared allocators**: Each side manages its own memory

### Example: Proper cleanup

```rust
// Rust calling Rune
let result_ptr = unsafe { rune_execute(...) };
// Use result_ptr...
unsafe { rune_free_string(result_ptr) }; // Rune frees
```

```zig
// Zig calling Glyph
const result = glyph_client_call_tool(...);
// Use result...
glyph_free_string(result); // Glyph frees
```

---

## Error Handling

### Rust Side

```rust
use glyph::ffi::FfiError;

let result = unsafe { rune_execute(...) };
if result.is_null() {
    return Err(glyph::Error::Protocol("Rune call failed".into()));
}
```

### Zig Side

```zig
const result = glyph_client_call_tool(...) orelse {
    std.log.err("Glyph call failed", .{});
    return error.GlyphCallFailed;
};
```

---

## Performance Considerations

### Zero-Copy Optimizations

- Rune uses arena allocators for batch operations
- Glyph uses `tokio::spawn_blocking` for FFI calls to avoid blocking async runtime

```rust
async fn call(&self, ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
    let args_json = serde_json::to_string(&args)?;

    // Run FFI call on blocking thread pool
    let result = tokio::task::spawn_blocking(move || {
        unsafe { call_rune_ffi(&args_json) }
    }).await?;

    Ok(result)
}
```

### Benchmarks

Based on Rune's performance targets:

- Text selection: **<1ms** response time (zero-copy)
- Workspace scanning: **3x faster** than pure Rust (parallel + SIMD)
- File operations: **>3x faster** (memory mapping)

---

## Troubleshooting

### Linker Errors

**Problem**: `undefined reference to rune_*`

**Solution**: Verify `librune.a` path in `build.rs`:
```rust
println!("cargo:rustc-link-search=native=/full/path/to/rune/zig-out/lib");
```

### Null Pointer Returns

**Problem**: Rune returns null pointer

**Solution**: Check Rune's error logs:
```zig
std.log.err("Rune error: {}", .{err});
```

### JSON Parsing Failures

**Problem**: `InvalidArgument` when parsing JSON

**Solution**: Validate JSON before FFI call:
```rust
serde_json::from_value::<T>(args)?; // Validate first
```

### Memory Leaks

**Problem**: Valgrind shows leaks

**Solution**: Ensure every `*_new()` has matching `*_free()`:
```rust
let ptr = unsafe { rune_execute(...) };
// ... use ptr ...
unsafe { rune_free_string(ptr) }; // Don't forget!
```

---

## Next Steps

- [Integration Contract](INTEGRATION_CONTRACT.md) - Full API surface
- [Rune Repository](https://github.com/ghostkellz/rune) - Zig implementation
- [FFI Tests](../tests/ffi_tests.rs) - Comprehensive test suite

---

**Document Version**: 0.1.0
**Last Updated**: 2025-10-02
**Compatible with**: Glyph v0.1.0, Rune Phase 2+
