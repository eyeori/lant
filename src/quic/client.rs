use std::net::SocketAddr;
use std::sync::Arc;

use crate::utils::error::Result;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rustls::pki_types::CertificateDer;

use crate::quic::cert::{LTS_CERT, SERVER_NAME};

pub fn build_client_config() -> Result<quinn::ClientConfig> {
    let tls_cert = CertificateDer::from(STANDARD.decode(LTS_CERT).unwrap());
    let mut roots = rustls::RootCertStore::empty();
    roots.add(tls_cert)?;

    let config = quinn::ClientConfig::with_root_certificates(Arc::new(roots))?;
    Ok(config)
}

pub fn build_endpoint(config: quinn::ClientConfig) -> Result<quinn::Endpoint> {
    let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
    endpoint.set_default_client_config(config);
    Ok(endpoint)
}

pub fn build_connecting(
    endpoint: Arc<quinn::Endpoint>,
    addr: SocketAddr,
) -> Result<quinn::Connecting> {
    Ok(endpoint.connect(addr, SERVER_NAME)?)
}
