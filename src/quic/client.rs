use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::quic::cert::{LTS_CERT, SERVER_NAME};

pub fn build_client_config() -> Result<rustls::ClientConfig> {
    let tls_cert = rustls::Certificate(STANDARD.decode(LTS_CERT).unwrap());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(&tls_cert)?;

    let config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(config)
}

pub fn build_endpoint(config: rustls::ClientConfig) -> Result<quinn::Endpoint> {
    let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(config)));

    Ok(endpoint)
}

pub fn build_connecting(
    endpoint: Arc<quinn::Endpoint>,
    addr: SocketAddr,
) -> Result<quinn::Connecting> {
    Ok(endpoint.connect(addr, SERVER_NAME)?)
}
