mod cli;

use clap::{Parser, Subcommand};
use cli::commands;

#[derive(Parser)]
#[command(name = "qoxide")]
#[command(about = "A lightweight local job queue backed by SQLite", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "./qoxide.db", help = "Database path")]
    db: String,

    #[arg(long, help = "Output in JSON format")]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Add a message to the queue")]
    Add {
        #[arg(help = "Message payload (base64 encoded, or UTF-8 with --utf8 flag)")]
        payload: String,

        #[arg(long, help = "Treat payload as UTF-8 string instead of base64")]
        utf8: bool,
    },

    #[command(about = "Reserve the next pending message")]
    Reserve {
        #[arg(long, help = "Output payload as UTF-8 string instead of base64")]
        utf8: bool,
    },

    #[command(about = "Mark a message as completed")]
    Complete {
        #[arg(help = "Message ID")]
        id: i64,
    },

    #[command(about = "Mark a message as failed")]
    Fail {
        #[arg(help = "Message ID")]
        id: i64,
    },

    #[command(about = "Remove a message permanently")]
    Remove {
        #[arg(help = "Message ID")]
        id: i64,
    },

    #[command(about = "Get a message payload by ID")]
    Get {
        #[arg(help = "Message ID")]
        id: i64,

        #[arg(long, help = "Output payload as UTF-8 string instead of base64")]
        utf8: bool,
    },

    #[command(about = "Show queue statistics")]
    Size,

    #[command(about = "List dead letter message IDs", name = "dead-letters")]
    DeadLetters,

    #[command(about = "Requeue dead letter messages back to pending")]
    Requeue {
        #[arg(help = "Message IDs to requeue", num_args = 1..)]
        ids: Vec<i64>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Add { payload, utf8 } => {
            commands::add(&cli.db, &payload, utf8, cli.json);
        }
        Command::Reserve { utf8 } => {
            commands::reserve(&cli.db, utf8, cli.json);
        }
        Command::Complete { id } => {
            commands::complete(&cli.db, id, cli.json);
        }
        Command::Fail { id } => {
            commands::fail(&cli.db, id, cli.json);
        }
        Command::Remove { id } => {
            commands::remove(&cli.db, id, cli.json);
        }
        Command::Get { id, utf8 } => {
            commands::get(&cli.db, id, utf8, cli.json);
        }
        Command::Size => {
            commands::show_size(&cli.db, cli.json);
        }
        Command::DeadLetters => {
            commands::list_dead_letters(&cli.db, cli.json);
        }
        Command::Requeue { ids } => {
            commands::requeue_dead_letters(&cli.db, &ids, cli.json);
        }
    }
}
