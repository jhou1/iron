use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod app;
mod db;
mod export;
mod i18n;
mod model;
mod tui;

#[derive(Parser)]
#[command(name = "iron", version, about = "Track your training")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Export all data to JSON
    Export {
        /// Output file path (defaults to ~/.iron/iron-export-YYYY-MM-DD.json)
        path: Option<PathBuf>,
    },
    /// Import data from JSON
    Import {
        /// Input file path
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    i18n::init();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Export { path }) => {
            let db = db::Database::open_default()?;
            export::export_to_json(&db, path)?;
            println!("{}", i18n::tr("cli-export-complete"));
        }
        Some(Commands::Import { path }) => {
            let db = db::Database::open_default()?;
            let count = export::import_from_json(&db, &path)?;
            println!("{}", i18n::tr_args("cli-imported", &[
                ("count", fluent_bundle::FluentValue::from(count as f64)),
            ]));
        }
        None => {
            app::run()?;
        }
    }

    Ok(())
}
