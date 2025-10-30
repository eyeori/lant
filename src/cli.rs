use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// LAN Transfer
#[derive(Parser)]
#[command(version, about, long_about = "LAN Transfer")]
#[command(propagate_version = true)]
pub enum Command {
    /// Running a lant server
    Server {
        /// As a server, provide a port for incoming connections
        #[arg(short, long)]
        port: u16,

        /// As a server, provide a root dir for the client to operate
        #[arg(short, long)]
        root_dir: PathBuf,
    },
    /// Execute a lant client command
    Client {
        /// As a client, provide the 'ip:port' of the server to which you want to connect
        #[arg(short, long)]
        srv_addr: String,

        #[command(subcommand)]
        cmd: ClientCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ClientCommand {
    /// List the contents of the specified path
    Ls {
        /// Gets the contents of the given path
        #[arg(short, long)]
        remote_path: PathBuf,
    },
    /// Put a file to the specified path
    Put {
        /// Local file that need to push
        #[arg(short, long)]
        file: PathBuf,

        /// Remote dir where the file push to
        #[arg(short, long)]
        remote_dir: PathBuf,
    },
    /// Get a file from the specified path
    Get {
        /// Remote file that need to get
        #[arg(short, long)]
        file: PathBuf,

        /// Local dir where the file save on
        #[arg(short, long)]
        local_dir: PathBuf,
    },
}
