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
        #[arg(long, env = "PORT", default_value = "8001")]
        port: u16,
        #[arg(long, env = "ATTACHMENTS_DIR", default_value = "attachments")]
        attachments_dir: String,
        #[arg(long, env = "MBOX_FILE")]
        mbox_file: Option<String>,
        #[arg(long, env = "INDEX_PATH", default_value = "tantivy_index")]
        index_path: String,
    },
    /// Index an MBOX file
    Index {
        #[arg(long)]
        mbox: String,
        #[arg(long)]
        reindex: bool,
        #[arg(long, default_value = "attachments")]
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
            mbox_file,
            index_path,
        }) => {
            // Auto-indexing logic migrated from entrypoint.sh
            std::fs::create_dir_all(&index_path)?;
            std::fs::create_dir_all(&attachments_dir)?;

            // Check if index exists (meta.json)
            let meta_path = std::path::Path::new(&index_path).join("meta.json");
            if !meta_path.exists() {
                if let Some(mbox) = mbox_file {
                    if std::path::Path::new(&mbox).exists() {
                        tracing::info!("Index not found. Auto-indexing MBOX at {}", mbox);
                        indexer::run_indexer(mbox, false, Some(attachments_dir.clone())).await?;
                    } else {
                        tracing::warn!("Index not found and MBOX file not found at: {}", mbox);
                    }
                } else {
                    tracing::warn!("Index not found and no MBOX_FILE environment variable set.");
                }
            } else {
                tracing::info!("Index found at {}", index_path);
            }

            // Set INDEX_PATH env var for server/store to usage (since store reads env var)
            // Ideally store should take path as arg, but strict refactoring might be too big now.
            // Actually, server::run_server reads INDEX_PATH env var on line 47.
            // We should ensure it's set or pass it.
            std::env::set_var("INDEX_PATH", &index_path);

            server::run_server(port, attachments_dir).await?;
        }
        None => {
            // Default behavior: equivalent to Serve with defaults/env vars
            // We manually parse envs or just default.
            // Easier to copy the logic or default to Serve command logic.
            // Let's re-parse as Serve to reuse logic.
            let port = std::env::var("PORT")
                .unwrap_or("8001".to_string())
                .parse()
                .unwrap_or(8001);
            let attachments_dir =
                std::env::var("ATTACHMENTS_DIR").unwrap_or("attachments".to_string());
            let mbox_file = std::env::var("MBOX_FILE").ok();
            let index_path = std::env::var("INDEX_PATH").unwrap_or("tantivy_index".to_string());

            // Auto-indexing logic
            std::fs::create_dir_all(&index_path)?;
            std::fs::create_dir_all(&attachments_dir)?;

            let meta_path = std::path::Path::new(&index_path).join("meta.json");
            if !meta_path.exists() {
                if let Some(mbox) = mbox_file {
                    if std::path::Path::new(&mbox).exists() {
                        tracing::info!("Index not found. Auto-indexing MBOX at {}", mbox);
                        indexer::run_indexer(mbox, false, Some(attachments_dir.clone())).await?;
                    }
                }
            }

            server::run_server(port, attachments_dir).await?;
        }
    }

    Ok(())
}
