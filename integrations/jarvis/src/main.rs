/// Jarvis MCP CLI
///
/// Interactive CLI for running Glyph MCP server with consent prompts and policy management.

use glyph::server::Server;
use glyph_jarvis::*;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use colored::Colorize;

#[derive(Parser)]
#[command(name = "jarvis-mcp")]
#[command(about = "Jarvis CLI with Glyph MCP backend", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start Jarvis MCP server
    Serve {
        /// Transport type (stdio or websocket)
        #[arg(short, long, default_value = "stdio")]
        transport: String,

        /// WebSocket address (if transport=websocket)
        #[arg(short, long, default_value = "127.0.0.1:7331")]
        address: String,

        /// Policy config file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Manage policy configuration
    Policy {
        #[command(subcommand)]
        action: PolicyActions,
    },

    /// Show audit logs
    Audit {
        /// Number of recent entries to show
        #[arg(short, long, default_value = "20")]
        tail: usize,
    },
}

#[derive(Subcommand)]
enum PolicyActions {
    /// Show current policy
    Show,

    /// Edit policy file
    Edit,

    /// Reset to default policy
    Reset,

    /// Add tool policy
    AddTool {
        /// Tool name
        name: String,

        /// Require consent
        #[arg(short, long)]
        consent: bool,

        /// Scopes (comma-separated)
        #[arg(short, long)]
        scopes: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,glyph=debug,jarvis=debug")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { transport, address, config } => {
            serve_command(transport, address, config).await?;
        }
        Commands::Policy { action } => {
            policy_command(action)?;
        }
        Commands::Audit { tail } => {
            audit_command(tail)?;
        }
    }

    Ok(())
}

async fn serve_command(
    transport: String,
    address: String,
    config_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    println!("{}", "╔════════════════════════════════════╗".cyan());
    println!("{}", "║   Jarvis MCP Server Starting...   ║".cyan());
    println!("{}", "╚════════════════════════════════════╝".cyan());

    // Load policy
    let config_path = config_path.unwrap_or_else(default_config_path);
    let policy = if config_path.exists() {
        println!("📋 Loading policy from: {}", config_path.display());
        load_policy(&config_path)?
    } else {
        println!("⚠️  No policy file found, using defaults");
        let policy = PolicyConfig::default();
        save_policy(&policy, &config_path)?;
        println!("✅ Created default policy at: {}", config_path.display());
        policy
    };

    println!("\n🔒 Consent mode: {:?}", policy.consent_mode);
    println!("📊 Audit logging: {}", if policy.audit.enabled { "enabled" } else { "disabled" });
    println!("🔧 Configured scopes: {}", policy.scopes.len());

    // Create consent guard and audit logger
    let guard = Arc::new(ConsentGuard::new(policy.clone()));
    let audit_logger = Arc::new(AuditLogger::new(policy.audit.clone()));

    // Build server
    let server = match transport.as_str() {
        "websocket" => {
            println!("\n🌐 Transport: WebSocket ({})", address);
            Server::builder()
                .with_server_info("jarvis-mcp-server", "0.1.0")
                .for_websocket(&address)
                .await?
        }
        "stdio" => {
            println!("\n📟 Transport: stdio");
            Server::builder()
                .with_server_info("jarvis-mcp-server", "0.1.0")
                .for_stdio()
        }
        _ => anyhow::bail!("Unknown transport: {}", transport),
    };

    println!("\n✨ Jarvis MCP server ready!\n");

    // Run server
    server.run().await?;

    Ok(())
}

fn policy_command(action: PolicyActions) -> anyhow::Result<()> {
    let config_path = default_config_path();

    match action {
        PolicyActions::Show => {
            if !config_path.exists() {
                println!("{}", "No policy file found. Run 'jarvis-mcp serve' first.".yellow());
                return Ok(());
            }

            let policy = load_policy(&config_path)?;
            println!("{}", "Current Policy Configuration:".bold());
            println!("{}", toml::to_string_pretty(&policy)?);
        }

        PolicyActions::Edit => {
            if !config_path.exists() {
                let policy = PolicyConfig::default();
                save_policy(&policy, &config_path)?;
                println!("✅ Created default policy");
            }

            println!("Opening policy file: {}", config_path.display());
            println!("Edit with your preferred editor:");
            println!("  vim {}", config_path.display());
            println!("  nano {}", config_path.display());
        }

        PolicyActions::Reset => {
            let policy = PolicyConfig::default();
            save_policy(&policy, &config_path)?;
            println!("{}", "✅ Policy reset to defaults".green());
        }

        PolicyActions::AddTool { name, consent, scopes } => {
            let mut policy = if config_path.exists() {
                load_policy(&config_path)?
            } else {
                PolicyConfig::default()
            };

            let tool_policy = ToolPolicy {
                consent_required: consent,
                scopes: scopes
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
                rate_limit: None,
            };

            policy.tool_policies.insert(name.clone(), tool_policy);
            save_policy(&policy, &config_path)?;

            println!("{}", format!("✅ Added policy for tool: {}", name).green());
        }
    }

    Ok(())
}

fn audit_command(tail: usize) -> anyhow::Result<()> {
    println!("{}", format!("Last {} audit log entries:", tail).bold());
    println!("{}", "(Audit logging to be implemented)".yellow());
    Ok(())
}
