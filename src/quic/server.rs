use crate::command::get::GetCommandServer;
use crate::command::ls::LsCommandServer;
use crate::command::put::PutCommandServer;
use crate::command::CommandServer;
use crate::message::{
    build_error_message, deconstruct_message, MessageType, RecvMessage, SendMessage,
};
use crate::quic::cert::{LTS_CERT, LTS_KEY};
use crate::utils::error::{MsgErr, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use net2::unix::UnixUdpBuilderExt;
use path_absolutize::Absolutize;
use quinn::TokioRuntime;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio::try_join;

pub struct Server {
    root_path: PathBuf,
    quic_server: QuicServer,
    conn_receiver: Arc<Receiver<quinn::Connection>>,
}

impl Server {
    pub fn new(listen_on: u16, root_path: PathBuf) -> Result<Self> {
        // root path check
        if !root_path.is_dir() {
            return MsgErr::res("root path is not a dir");
        }

        // conn channel
        let (conn_sender, conn_receiver) = channel();
        let conn_receiver = Arc::new(conn_receiver);

        let quic_server = QuicServer::new(
            listen_on,
            LTS_CERT.to_string(),
            LTS_KEY.to_string(),
            conn_sender,
        );

        Ok(Self {
            root_path,
            quic_server,
            conn_receiver,
        })
    }

    pub fn get_server_abs_root_dir(&self) -> Result<PathBuf> {
        let root_dir = &self.root_path;
        Ok(root_dir.absolutize()?.to_path_buf())
    }

    pub async fn start(&self) -> Result<()> {
        println!("Server starting...");

        let abs_root_path = self.get_server_abs_root_dir()?;
        println!("root path is {abs_root_path:?}",);

        // start server
        try_join!(
            self.handle_connection(abs_root_path),
            self.quic_server.start()
        )?;
        Ok(())
    }

    async fn handle_connection(&self, abs_root_path: PathBuf) -> Result<()> {
        let receiver = self.conn_receiver.clone();
        loop {
            match receiver.try_recv() {
                Ok(conn) => {
                    println!(
                        "[Server] Receive a connection, from {:?}",
                        conn.remote_address()
                    );
                    tokio::spawn(Self::handle_requests(abs_root_path.clone(), conn));
                }
                Err(e) => match e {
                    TryRecvError::Empty => {
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    TryRecvError::Disconnected => {
                        println!("[ERR][Server] Receive connection error, error={e}");
                        break;
                    }
                },
            }
        }
        Ok(())
    }

    async fn handle_requests(abs_root_dir: PathBuf, conn: quinn::Connection) {
        loop {
            match conn.accept_bi().await {
                Ok((ss, rs)) => {
                    tokio::spawn(Self::handle_request(abs_root_dir.clone(), ss, rs));
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

    async fn handle_request(
        abs_root_dir: PathBuf,
        mut ss: quinn::SendStream,
        mut rs: quinn::RecvStream,
    ) {
        // receive request data
        if let Ok(request) = rs.read_to_end(usize::MAX).await {
            // do business and build response
            let res_msg = Self::handle_business(abs_root_dir.clone(), request.into()).await;
            let mut response = res_msg.unwrap_or_else(|e| build_error_message(e.to_string()));

            // send response back
            if let Err(e) = ss.write_all_chunks(response.as_mut_slice()).await {
                println!("[ERR][Server] Send back message error, error={e}");
            }
        }
    }

    async fn handle_business(abs_root_dir: PathBuf, msg: RecvMessage) -> Result<SendMessage> {
        let (msg_type, msg_payload) = deconstruct_message(&msg)?;
        let req_payload = msg_payload.ok_or(MsgErr::new("request body is null"))?;
        match msg_type {
            MessageType::LsRequest => {
                LsCommandServer::new(abs_root_dir.clone())
                    .handle(req_payload)
                    .await
            }
            MessageType::PutRequest => {
                PutCommandServer::new(abs_root_dir.clone())
                    .handle(req_payload)
                    .await
            }
            MessageType::GetRequest => {
                GetCommandServer::new(abs_root_dir.clone())
                    .handle(req_payload)
                    .await
            }
            msg_type => MsgErr::res(format!("not supported message type, type={msg_type:?}")),
        }
    }
}

pub struct QuicServer {
    listen_port: u16,
    lts_cert: String,
    lts_key: String,
    conn_sender: Arc<Sender<quinn::Connection>>,
}

impl QuicServer {
    pub fn new(
        listen_port: u16,
        lts_cert: String,
        lts_key: String,
        conn_sender: Sender<quinn::Connection>,
    ) -> Self {
        QuicServer {
            listen_port,
            lts_cert,
            lts_key,
            conn_sender: Arc::new(conn_sender),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let config = self.build_config()?;
        let endpoint = self.listen(config)?;
        tokio::spawn(Self::handle_accept(endpoint, self.conn_sender.clone())).await?;
        Ok(())
    }

    fn build_config(&self) -> Result<quinn::ServerConfig> {
        let cert = CertificateDer::from(STANDARD.decode(&self.lts_cert)?);
        let key = PrivateKeyDer::try_from(STANDARD.decode(&self.lts_key)?)?;
        let config = quinn::ServerConfig::with_single_cert(vec![cert], key)?;
        Ok(config)
    }

    fn listen(&self, config: quinn::ServerConfig) -> Result<quinn::Endpoint> {
        let addr = SocketAddr::from((IpAddr::from(Ipv6Addr::UNSPECIFIED), self.listen_port));
        let socket = net2::UdpBuilder::new_v6()?
            .reuse_address(true)?
            .reuse_port(true)?
            .bind(addr)?;
        let endpoint = quinn::Endpoint::new(
            Default::default(),
            Some(config),
            socket,
            Arc::new(TokioRuntime),
        )?;
        println!("listen on {}", addr);
        Ok(endpoint)
    }

    async fn handle_accept(endpoint: quinn::Endpoint, conn_sender: Arc<Sender<quinn::Connection>>) {
        loop {
            if let Some(incoming) = endpoint.accept().await {
                tokio::spawn(Self::handle_incoming(incoming, conn_sender.clone()));
            }
        }
    }

    async fn handle_incoming(
        incoming: quinn::Incoming,
        conn_sender: Arc<Sender<quinn::Connection>>,
    ) {
        match incoming.await {
            Ok(conn) => {
                if let Err(e) = conn_sender.send(conn) {
                    println!("[ERR][Quic] Send new conn to higher level server error, error={e}");
                }
            }
            Err(e) => {
                println!("[ERR][Quic] Connection error, error={e}");
            }
        }
    }
}
