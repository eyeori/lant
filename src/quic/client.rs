use std::net::SocketAddr;
use std::sync::Arc;

use crate::message::{
    build_message, deconstruct_message, MessagePayloadRef, MessageType, RecvMessage,
    ToMessagePayload,
};
use crate::quic::cert::{LTS_CERT, SERVER_NAME};
use crate::utils::error::Result;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rustls::pki_types::CertificateDer;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Stage<T> {
    Processing(T),
    Finish,
}

impl<T> Default for Stage<T> {
    fn default() -> Self {
        Self::Finish
    }
}

pub struct Client {
    quic_client: QuicClient,
}

impl Client {
    pub fn new(addr: &str) -> Result<Self> {
        let quic_client = QuicClient::new(SERVER_NAME.to_string(), addr.parse()?, LTS_CERT)?;
        Ok(Self { quic_client })
    }

    pub fn connecting(&self) -> Result<quinn::Connecting> {
        self.quic_client.connecting()
    }

    pub async fn request(
        &self,
        conn: &quinn::Connection,
        msg_type: MessageType,
        payload: impl ToMessagePayload,
    ) -> Result<RecvMessage> {
        // build request message
        let mut msg = build_message(msg_type, payload);

        // connect & send request
        let (mut ss, mut rs) = conn.open_bi().await?;
        ss.write_all_chunks(msg.as_mut_slice()).await?;
        ss.finish()?;

        // receive response
        let response = rs.read_to_end(usize::MAX).await?;
        Ok(response.into())
    }

    pub fn unwrap_message<'a>(
        &self,
        msg: &'a RecvMessage,
        msg_type_expect: MessageType,
    ) -> Result<Option<MessagePayloadRef<'a>>> {
        let (msg_type, msg_payload) = deconstruct_message(msg)?;
        match msg_type {
            msg_type if msg_type == msg_type_expect => Ok(msg_payload),
            MessageType::Error => {
                if let Some(payload) = msg_payload {
                    println!("[ERR]{}", std::str::from_utf8(payload)?);
                }
                Ok(None)
            }
            msg_type => {
                println!("[ERR]{msg_type:?} not fit");
                Ok(None)
            }
        }
    }

    pub async fn wait(&self) {
        self.quic_client.wait().await;
    }
}

pub struct QuicClient {
    server_name: String,
    addr: SocketAddr,
    endpoint: quinn::Endpoint,
}

impl QuicClient {
    pub fn new(server_name: String, addr: SocketAddr, lts_cert: &str) -> Result<Self> {
        let endpoint = Self::build_endpoint(lts_cert)?;
        Ok(Self {
            server_name,
            addr,
            endpoint,
        })
    }

    pub fn connecting(&self) -> Result<quinn::Connecting> {
        Ok(self.endpoint.connect(self.addr, &self.server_name)?)
    }

    pub async fn wait(&self) {
        self.endpoint.wait_idle().await;
    }

    fn build_endpoint(lts_cert: &str) -> Result<quinn::Endpoint> {
        let config = Self::build_client_config(lts_cert)?;
        let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(config);
        Ok(endpoint)
    }

    fn build_client_config(lts_cert: &str) -> Result<quinn::ClientConfig> {
        let tls_cert = CertificateDer::from(STANDARD.decode(lts_cert)?);
        let mut roots = rustls::RootCertStore::empty();
        roots.add(tls_cert)?;

        let config = quinn::ClientConfig::with_root_certificates(Arc::new(roots))?;
        Ok(config)
    }
}
