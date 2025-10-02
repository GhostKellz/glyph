# Glyph MCP Roadmap — MVP ➝ Alpha ➝ Beta ➝ Theta ➝ RCs ➝ Release

_Last updated: October 2, 2025 (Evening)_

## � Current Snapshot (Beta & Theta complete, ready for RC1)
- ✅ **Core runtime**: stdio + WebSocket transports, 7 production-ready tools (echo, read/write/list/delete, shell, http), server builder, CLI binary.
- ✅ **Docs refreshed**: Quickstart, binary guide, transport guide, built-in tools reference, installation guide, integration contract.
- ✅ **Alpha cleanup complete**
  - All tests passing (28 unit + 5 CLI smoke tests)
  - Zero warnings from `cargo check`
  - Crate metadata polished with MSRV 1.75, keywords, categories
- 🎯 **Integration targets** (from `/archive`):
  - `ghostllm` (Rust LLM proxy) → provider tools + cost policy alignment.
  - `ghostflow` (Rust n8n alternative) → MCP nodes + flow publishing.
  - `jarvis` (Rust CLI copilot) → Glyph-hosted tool routing.
  - `zeke` (Zig dev companion) + `ZIG_MCP_TOOLS.md` Rune artifacts → FFI bridge.
  - External inspiration: `genai-toolbox` (database MCP), `github-mcp-server`, `gemini-cli`, Microsoft `mcp` repo, `playwright-mcp` for advanced tooling patterns.

---

## 🥅 Phase Breakdown

### 🟢 MVP (ship-ready core) — ✅ Complete
- ✅ Protocol types, server builder, transport abstraction with stdio & WebSocket.
- ✅ Tool registry + validation, resource/prompt registries scaffolded.
- ✅ CLI binary (`glyph serve/test`), release profile builds verified.
- ✅ Documentation: quickstart, binary, transport, tools, architecture outline.
- ✅ Rune FFI hooks compiled (per `ZIG_MCP_TOOLS.md`).

### 🟡 Alpha (hardening & packaging) — ✅ Complete
- [x] Fix `cargo test` by updating `examples/client_example.rs` & `examples/server_example.rs` to new tool traits.
- [x] Resolve `ProgressNotification` glob warning by explicit re-export in `src/protocol/mod.rs`.
- [x] Trim unused imports in `src/transport/{stdio,websocket}.rs` and `src/server/resources.rs`.
- [x] Add smoke tests for CLI (`glyph serve --transport stdio` with timeout harness).
- [x] Finish `docs/installation.md` (binary + crate + Docker) and cross-link from top-level README.
- [x] Publish crate metadata polish: categories, keywords, badges, MSRV in README.
- [x] Draft integration contract doc describing tool API surface for GhostLLM/GhostFlow/Jarvis.

### � Beta (first integrations & developer tooling) — ✅ Complete
- [x] **GhostFlow**: implement MCP node adapter referencing `archive/ghostflow` API (expose Glyph tools as nodes, import flows as MCP prompts).
- [x] **GhostLLM**: add provider passthrough tool set (OpenAI/Anthropic/Gemini) using GhostLLM proxy endpoints; align auth + rate-limit policy.
- [x] **Jarvis CLI**: wrap Glyph server as optional backend (`jarvis` plugin) with consent prompts and tool scopes.
- [x] Package Glyph as reusable crate for third-party Rust projects (feature docs, example integration crate).
- [x] Stabilize FFI layer with C ABI tests and publish Rune integration guide (leveraging `ZIG_MCP_TOOLS.md`).
- [x] Provide VS Code / Neovim host snippets mirroring `github-mcp-server` config patterns.

### 🟣 Theta (observability, policy, scale) — ✅ Complete
- [x] Implement policy engine (consent gates, audit trail) taking cues from `jarvis` + `ghostllm` governance.
- [x] Add Prometheus metrics + OpenTelemetry tracing; document dashboards inspired by `genai-toolbox` & `ghostllm` monitoring.
- [x] Harden security: secret redaction, rate limiting, TLS config templates.
- [ ] Implement resource subscriptions + change notifications (deferred to v0.2).
- [ ] WebSocket clustering + graceful shutdown for multi-client deployments (deferred to v0.2).

### 🧪 Release Candidates (RC1 → RC6)
- **RC1** – Alpha freeze: all tests green, docs reviewed, publish `v0.1.0-rc.1` tag.
- **RC2** – Packaging: crates.io dry-run, Docker image, Homebrew tap draft, verify Zig static linking.
- **RC3** – Performance: latency benchmarks (<10 ms local), load tests (GhostFlow scenario), memory profiling.
- **RC4** – Security & compliance review: cargo-audit, license scan, policy documentation.
- **RC5** – Integration validation: GhostLLM, GhostFlow, Jarvis, Zeke end-to-end smoke tests.
- **RC6** – Release rehearsal: changelog, migration notes, upgrade guide, marketing copy.

Each RC requires: ✅ previous RC checklists, ✅ zero blocking bugs, 📦 signed artifacts, 📚 updated release notes.

### 🟩 GA Release (v0.1.0)
- [ ] Publish crate to crates.io and annotated git tag.
- [ ] Publish Docker image + binary tarballs for Linux/macOS/Windows.
- [ ] Final docs pass: quickstart, integration guides, API reference, troubleshooting.
- [ ] Announce across GhostStack projects (GhostLLM, GhostFlow, Jarvis, Zeke) and update their READMEs with Glyph instructions.
- [ ] Establish support cadence (issue triage SLA, security response policy).

---

## � Reference Backlog & Links
- External MCP references: `archive/mcp`, `playwright-mcp`, `github-mcp-server`, `genai-toolbox`, `gemini-cli` for UX + packaging patterns.
- Future enhancements to track post-GA (inspired by original long-form TODO):
  - WebRTC/QUIC transports, plugin system, hot reload, distributed cluster mode.
  - glyph-ffi for additional languages (Python via PyO3, WASM build pipeline).
  - Observability bundles (Grafana dashboards, otel collector recipes).
  - Terraform/Helm modules for production deployment.
- Keep this roadmap updated after each milestone retro.