use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use std::path::PathBuf;

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
        #[arg(long, env = "HOST", default_value = "127.0.0.1")]
        host: String,
        #[arg(long, env = "ATTACHMENTS_DIR")]
        attachments_dir: Option<String>,
        #[arg(long, env = "MBOX_FILE")]
        mbox_file: Option<String>,
        #[arg(long, env = "INDEX_PATH")]
        index_path: Option<String>,
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

fn get_default_paths() -> (PathBuf, PathBuf) {
    if let Some(proj_dirs) = ProjectDirs::from("com", "tocisz", "mbox-viewer") {
        let data_dir = proj_dirs.data_dir();
        (data_dir.join("tantivy_index"), data_dir.join("attachments"))
    } else {
        // Fallback to current directory if we can't get standard valid paths
        (PathBuf::from("tantivy_index"), PathBuf::from("attachments"))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let (default_index_path, default_attachments_dir) = get_default_paths();

    match cli.command {
        Some(Commands::Index {
            mbox,
            reindex,
            attachments_dir,
        }) => {
            let attachments_dir = attachments_dir
                .unwrap_or_else(|| default_attachments_dir.to_string_lossy().to_string());
            indexer::run_indexer(mbox, reindex, Some(attachments_dir)).await?;
        }
        Some(Commands::Serve {
            port,
            host,
            attachments_dir,
            mbox_file,
            index_path,
        }) => {
            let index_path =
                index_path.unwrap_or_else(|| default_index_path.to_string_lossy().to_string());
            let attachments_dir = attachments_dir
                .unwrap_or_else(|| default_attachments_dir.to_string_lossy().to_string());

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

            std::env::set_var("INDEX_PATH", &index_path);

            server::run_server(host, port, attachments_dir).await?;
        }
        None => {
            // Default behavior: equivalent to Serve with defaults/env vars
            let port = std::env::var("PORT")
                .unwrap_or("8001".to_string())
                .parse()
                .unwrap_or(8001);

            let attachments_dir = std::env::var("ATTACHMENTS_DIR")
                .unwrap_or_else(|_| default_attachments_dir.to_string_lossy().to_string());

            let mbox_file = std::env::var("MBOX_FILE").ok();

            let index_path = std::env::var("INDEX_PATH")
                .unwrap_or_else(|_| default_index_path.to_string_lossy().to_string());

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

            let host = std::env::var("HOST").unwrap_or("127.0.0.1".to_string());

            // Ensure INDEX_PATH is set for the store module
            std::env::set_var("INDEX_PATH", &index_path);

            server::run_server(host, port, attachments_dir).await?;
        }
    }

    Ok(())
}
