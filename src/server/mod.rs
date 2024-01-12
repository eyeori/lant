use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use path_absolutize::Absolutize;
use tokio::try_join;

use crate::message::{
    build_error_message, deconstruct_message, MessageType, RecvMessage, SendMessage,
};
use crate::quic::quic_server;

mod get;
mod ls;
mod put;

pub struct ServerContext {
    pub root_path: PathBuf,
}

impl ServerContext {
    pub fn new(root_path: &Path) -> Self {
        Self {
            root_path: root_path.to_path_buf(),
        }
    }
}

static SERVER_CONTEXT: OnceCell<ServerContext> = OnceCell::new();

pub fn get_server_abs_root_dir() -> Result<PathBuf> {
    let server_context = SERVER_CONTEXT
        .get()
        .ok_or(anyhow!("server context not initialized"))?;
    let root_dir = &server_context.root_path;
    Ok(root_dir.absolutize()?.to_path_buf())
}

pub(crate) async fn start(listen_on: u16, root_path: &Path) -> Result<()> {
    // root path check
    if !root_path.is_dir() {
        return Err(anyhow!("root path is not a dir"));
    }

    println!("Server starting...");
    println!("root path is {}", root_path.absolutize()?.to_str().unwrap());

    // init server context
    let _ = SERVER_CONTEXT.set(ServerContext::new(root_path));

    // conn channel
    let (conn_sender, conn_receiver) = channel();
    let conn_sender = Arc::new(conn_sender);
    let conn_receiver = Rc::new(conn_receiver);

    // start server
    let server_fut = server_start(conn_receiver);
    let quic_server_fut = quic_server::start(listen_on, conn_sender);
    try_join!(server_fut, quic_server_fut)?;

    Ok(())
}

async fn server_start(conn_receiver: Rc<Receiver<quinn::Connection>>) -> Result<()> {
    handle_connection(conn_receiver);
    Ok(())
}

fn handle_connection(receiver: Rc<Receiver<quinn::Connection>>) {
    loop {
        match receiver.try_recv() {
            Ok(conn) => {
                println!(
                    "[Server] Receive a connection, from {:?}",
                    conn.remote_address()
                );
                tokio::spawn(handle_requests(conn));
            }
            Err(e) => match e {
                TryRecvError::Empty => {
                    sleep(Duration::from_millis(100));
                    continue;
                }
                TryRecvError::Disconnected => {
                    println!("[ERR][Server] Receive connection error, error={e}");
                    break;
                }
            },
        }
    }
}

async fn handle_requests(conn: quinn::Connection) {
    loop {
        match conn.accept_bi().await {
            Ok((ss, rs)) => {
                tokio::spawn(handle_request(ss, rs));
            }
            e @ Err(
                quinn::ConnectionError::ConnectionClosed(_)
                | quinn::ConnectionError::ApplicationClosed(_)
                | quinn::ConnectionError::Reset
                | quinn::ConnectionError::LocallyClosed,
            ) => {
                println!(
                    "[Server] Connection closed, connection={:?}, reason={:?}",
                    conn.remote_address(),
                    e.unwrap_err()
                );
                break;
            }
            Err(e) => {
                println!(
                    "[ERR][Server] No more bi streams on connection {:?}, error={e}",
                    conn.remote_address()
                );
                break;
            }
        }
    }
}

async fn handle_request(mut ss: quinn::SendStream, mut rs: quinn::RecvStream) {
    // receive request data
    if let Ok(request) = rs.read_to_end(usize::MAX).await {
        // do business and build response
        let res_msg = handle_business(request.into()).await;
        let mut response = res_msg.unwrap_or_else(|e| build_error_message(e.to_string()));

        // send response back
        if let Err(e) = ss.write_all_chunks(response.as_mut_slice()).await {
            println!("[ERR][Server] Send back message error, error={e}");
        }
    }
}

async fn handle_business(msg: RecvMessage) -> Result<SendMessage> {
    let (msg_type, msg_payload) = deconstruct_message(&msg)?;
    let req_payload = msg_payload.ok_or(anyhow!("request body is null"))?;
    match msg_type {
        MessageType::LsRequest => ls::request(req_payload).await,
        MessageType::PutRequest => put::request(req_payload).await,
        MessageType::GetRequest => get::request(req_payload).await,
        msg_type => Err(anyhow!("not supported message type, type={msg_type:?}")),
    }
}
