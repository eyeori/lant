use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::utils::error::Result;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use net2::unix::UnixUdpBuilderExt;
use quinn::TokioRuntime;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use crate::quic::cert::{LTS_CERT, LTS_KEY};

pub async fn start(listen_port: u16, conn_sender: Arc<Sender<quinn::Connection>>) -> Result<()> {
    tokio::spawn(quic_handle_accept(
        quic_start_listen(listen_port)?,
        conn_sender,
    ))
    .await?;
    Ok(())
}

fn quic_build_server_config() -> Result<quinn::ServerConfig> {
    let cert = CertificateDer::from(STANDARD.decode(LTS_CERT)?);
    let key = PrivateKeyDer::try_from(STANDARD.decode(LTS_KEY)?)?;
    let config = quinn::ServerConfig::with_single_cert(vec![cert], key)?;
    Ok(config)
}

fn quic_start_listen(port: u16) -> Result<quinn::Endpoint> {
    let server_config = Some(quic_build_server_config()?);
    let addr = SocketAddr::from((IpAddr::from(Ipv6Addr::UNSPECIFIED), port));
    let socket = net2::UdpBuilder::new_v6()?
        .reuse_address(true)?
        .reuse_port(true)?
        .bind(addr)?;
    let endpoint = quinn::Endpoint::new(
        Default::default(),
        server_config,
        socket,
        Arc::new(TokioRuntime),
    )?;
    println!("listen on {}", addr);
    Ok(endpoint)
}

async fn quic_handle_accept(
    endpoint: quinn::Endpoint,
    conn_sender: Arc<Sender<quinn::Connection>>,
) {
    loop {
        if let Some(incoming) = endpoint.accept().await {
            tokio::spawn(quic_handle_incoming(incoming, conn_sender.clone()));
        }
    }
}

async fn quic_handle_incoming(
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
