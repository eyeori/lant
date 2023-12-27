use std::path::PathBuf;

use anyhow::Result;
use structopt::StructOpt;

mod client;
mod message;
mod quic;
mod server;
mod utils;

#[derive(StructOpt, Debug)]
#[structopt(name = "lant", about = "LAN Transfer")]
enum Command {
    #[structopt(about = "Running a lant server")]
    Server {
        #[structopt(
            short,
            long,
            long_help = "As a server, provide a port for incoming connections"
        )]
        listen_on: u16,

        #[structopt(
            short,
            long,
            parse(from_os_str),
            long_help = "As a server, provide a root path for the client to operate"
        )]
        root_path: PathBuf,
    },
    #[structopt(about = "Execute a lant client command")]
    Client {
        #[structopt(
            short,
            long,
            long_help = "As a client, provide the 'ip:port' of the server to which you want to connect"
        )]
        connect_to: String,

        #[structopt(subcommand)]
        sub_command: ClientSubCommand,
    },
}

#[derive(StructOpt, Debug)]
enum ClientSubCommand {
    #[structopt(about = "List the contents of the specified path")]
    Ls {
        #[structopt(
            short,
            long,
            parse(from_os_str),
            long_help = "Gets the contents of the given path"
        )]
        path_on_remote: PathBuf,
    },
    #[structopt(about = "Put a file to the specified path")]
    Put {
        #[structopt(
            short,
            long,
            parse(from_os_str),
            long_help = "Local file that need to push"
        )]
        file_path: PathBuf,

        #[structopt(
            short,
            long,
            parse(from_os_str),
            long_help = "Remote dir where the file push to"
        )]
        remote_dir: PathBuf,
    },
    #[structopt(about = "Get a file from the specified path")]
    Get {
        #[structopt(
            short,
            long,
            parse(from_os_str),
            long_help = "Remote file that need to get"
        )]
        file_path: PathBuf,

        #[structopt(
            short,
            long,
            parse(from_os_str),
            long_help = "Local dir where the file save on"
        )]
        local_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = Command::from_args();

    match &cmd {
        Command::Server {
            listen_on,
            root_path,
        } => {
            server::start(*listen_on, root_path).await?;
        }
        Command::Client {
            connect_to,
            sub_command,
        } => {
            let (endpoint, connecting) = client::init(connect_to)?;
            match sub_command {
                ClientSubCommand::Ls { path_on_remote } => {
                    client::ls(connecting, path_on_remote).await
                }
                ClientSubCommand::Put {
                    file_path,
                    remote_dir,
                } => {
                    client::put(connecting, file_path, remote_dir).await;
                }
                ClientSubCommand::Get {
                    file_path,
                    local_dir,
                } => {
                    client::get(connecting, file_path, local_dir).await;
                }
            }
            endpoint.wait_idle().await;
        }
    }
    Ok(())
}
