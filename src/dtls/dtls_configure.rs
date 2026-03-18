use std::sync::Arc;
use anyhow::Result;
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use tracing::{error, info};
use dtls::cipher_suite::CipherSuiteId;
use dtls::config::{Config as DtlsConfig, ExtendedMasterSecretType};
use dtls::conn::DTLSConn;
use webrtc_util::Conn;

pub async fn dtls_configure(conn: Arc<dyn Conn + Send + Sync>) -> Result<Arc<dyn Conn + Send + Sync>> {
  let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
  let params = CertificateParams::default();
  let cert = params.self_signed(&key_pair)?;

  let dtls_cert = dtls::crypto::Certificate {
    certificate: vec![cert.der().to_vec().into()],
    private_key: dtls::crypto::CryptoPrivateKey::from_key_pair(
      &key_pair).map_err(|e| error!("DTLS key parsing error: {}", e)).unwrap(),
  };

  let config = DtlsConfig {
    certificates: vec![dtls_cert],
    extended_master_secret: ExtendedMasterSecretType::Request,
    cipher_suites: vec![CipherSuiteId::Tls_Ecdhe_Ecdsa_With_Aes_128_Gcm_Sha256],
    insecure_skip_verify: true,
    ..Default::default()
  };

  let dtls_conn = DTLSConn::new(conn, config, true, None).await?;

  info!("DTLS Handshake completed successfully");

  Ok(Arc::new(dtls_conn) as Arc<dyn Conn + Send + Sync>)
}