use std::path::PathBuf;
use std::process::exit;

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
    Server {
        #[structopt(
            short,
            long,
            about = "As a server, provide a port for incoming connections"
        )]
        listen_on: u16,

        #[structopt(
            short,
            long,
            parse(from_os_str),
            about = "As a server, provide a root path for the client to operate"
        )]
        root_path: PathBuf,
    },
    Client {
        #[structopt(
            short,
            long,
            about = "As a client, provide the 'ip:port' of the server to which you want to connect"
        )]
        connect_to: String,

        #[structopt(subcommand, about = "As a client, provide the operator")]
        sub_command: ClientSubCommand,
    },
}

#[derive(StructOpt, Debug)]
#[structopt(about = "Client sub command")]
enum ClientSubCommand {
    Ls {
        #[structopt(
            short,
            long,
            parse(from_os_str),
            about = "Gets the contents of the given path"
        )]
        path_on_remote: PathBuf,
    },
    Put {
        #[structopt(
            short,
            long,
            parse(from_os_str),
            about = "Local file that need to push"
        )]
        file_path: PathBuf,

        #[structopt(
            short,
            long,
            parse(from_os_str),
            about = "Remote dir where the file push to"
        )]
        remote_dir: PathBuf,
    },
    Get {
        #[structopt(
            short,
            long,
            parse(from_os_str),
            about = "Remote file that need to get"
        )]
        file_path: PathBuf,

        #[structopt(
            short,
            long,
            parse(from_os_str),
            about = "Local dir where the file save on"
        )]
        local_dir: PathBuf,
    },
}

fn main() {
    let cmd = Command::from_args();

    let code = match run(&cmd) {
        Ok(()) => 0,
        Err(e) => {
            println!("[ERR]{e}");
            1
        }
    };

    exit(code);
}

#[tokio::main]
#[allow(clippy::field_reassign_with_default)]
async fn run(cmd: &Command) -> Result<()> {
    match cmd {
        Command::Server {
            listen_on,
            root_path,
        } => {
            server::start(listen_on, root_path).await?;
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
