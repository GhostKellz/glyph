#!/bin/bash

# Glyph MCP Server Deployment Script
# This script builds and deploys the Glyph MCP server

set -e

echo "🚀 Building Glyph MCP Server..."

# Build the release binary
cargo build --release

echo "✅ Build complete!"

# Check if Docker is available
if command -v docker &> /dev/null; then
    echo "🐳 Building Docker image..."
    docker build -t glyph-mcp-server .
    echo "✅ Docker image built: glyph-mcp-server"

    echo "📦 To run with Docker:"
    echo "docker run -p 7331:7331 glyph-mcp-server"
else
    echo "⚠️  Docker not found. To run the server:"
    echo "./target/release/glyph serve"
fi

echo ""
echo "🎯 Deployment ready!"
echo ""
echo "Available commands:"
echo "  ./target/release/glyph serve              # Start WebSocket server"
echo "  ./target/release/glyph serve --transport stdio  # Start stdio server"
echo "  ./target/release/glyph --help             # Show all options"
echo ""
echo "Test with:"
echo "  cargo run --example test_client"