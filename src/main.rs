use crate::cli::{ClientCommand, Command};
use crate::command::get::GetCommandClient;
use crate::command::ls::LsCommandClient;
use crate::command::put::PutCommandClient;
use crate::command::CommandClient;
use crate::quic::client::Client;
use crate::quic::server::Server;
use anyhow::Result;
use clap::Parser;

mod cli;
mod command;
mod message;
mod quic;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    match Command::parse() {
        Command::Server { port, root_dir } => Server::new(port, &root_dir)?.start().await?,
        Command::Client { srv_addr, cmd } => {
            let client = Client::new(&srv_addr)?;
            match cmd {
                ClientCommand::Ls { remote_path } => {
                    let cmd = LsCommandClient::new(&client, &remote_path);
                    cmd.request().await;
                }
                ClientCommand::Put { file, remote_dir } => {
                    let cmd = PutCommandClient::new(&client, &file, &remote_dir);
                    cmd.request().await;
                }
                ClientCommand::Get { file, local_dir } => {
                    let cmd = GetCommandClient::new(&client, &file, &local_dir);
                    cmd.request().await;
                }
            };
            client.wait().await;
        }
    }
    Ok(())
}
