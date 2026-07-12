use clap::{Parser, Subcommand};

use libereco::core::error::Result;
use libereco::ctl::{commands, create_backend};

#[derive(Parser)]
#[command(
    name = "liberecoctl",
    version,
    about = "LIBERECO management utility"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create virtual audio devices
    Init {
        /// Language code (en, ru, de...)
        language: String,
    },

    /// Remove virtual audio devices
    Remove {
        language: String,
    },

    /// Recreate virtual audio devices
    Repair {
        language: String,
    },

    /// List configured devices
    List {
        language: String,
    },

    /// Show device status
    Status {
        /// Language code (en, ru, de...)
        language: String,
    },

    /// Check system configuration
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let backend = create_backend()?;

    match cli.command {
        Command::Init { language } =>
            commands::init::run(backend.as_ref(), &language),

        Command::Remove { language } =>
            commands::remove::run(backend.as_ref(), &language),

        Command::Repair { language } =>
            commands::repair::run(backend.as_ref(), &language),

        Command::List { language } =>
            commands::list::run(backend.as_ref(), &language),

        Command::Status { language } =>
            commands::status::run(backend.as_ref(), &language),

        Command::Doctor =>
            commands::doctor::run(backend.as_ref()),
    }
}
