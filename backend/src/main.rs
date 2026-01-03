use clap::{Parser, Subcommand};

mod common;
mod indexer;
mod server;
mod store;

#[derive(Parser)]
#[command(name = "backend")]
#[command(about = "Gmail MBOX Viewer Backend & Indexer")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the server (default)
    Serve {
        #[arg(long, default_value = "8001")]
        port: u16,
        #[arg(long, default_value = "attachments")]
        attachments_dir: String,
    },
    /// Index an MBOX file
    Index {
        #[arg(long)]
        mbox: String,
        #[arg(long)]
        reindex: bool,
        #[arg(long)]
        attachments_dir: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Index {
            mbox,
            reindex,
            attachments_dir,
        }) => {
            indexer::run_indexer(mbox, reindex, attachments_dir).await?;
        }
        Some(Commands::Serve {
            port,
            attachments_dir,
        }) => {
            server::run_server(port, attachments_dir).await?;
        }
        None => {
            // Default to serve, respecting env vars for backward compatibility
            let port = std::env::var("PORT")
                .unwrap_or("8001".to_string())
                .parse()
                .unwrap_or(8001);
            let attachments_dir =
                std::env::var("ATTACHMENTS_DIR").unwrap_or("attachments".to_string());
            server::run_server(port, attachments_dir).await?;
        }
    }

    Ok(())
}
