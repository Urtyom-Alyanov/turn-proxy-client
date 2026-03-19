use crate::dtls::dtls_configure::dtls_configure;
use crate::proxy_process::run_bridge_group::run_bridge_group;
use crate::proxy_process::target_conn::TargetedConn;
use crate::{
  configuration::configuration::AppConfiguration,
  proxy_process::setup_and_run_provider::setup_and_run_provider,
};

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use webrtc_util::Conn;

pub async fn listening(config: AppConfiguration) -> Result<()> {
  let listen_addr: SocketAddr = config
    .common
    .listening_on
    .parse()
    .context("'listening-on' is not a valid socket address")?;
  let peer_addr: SocketAddr = config
    .common
    .peer_addr
    .parse()
    .context("'proxy-into' is not a valid socket address")?;

  info!("Listening on: {} UDP", listen_addr);
  info!("Proxying to: {} DTLS UDP", peer_addr);

  let listen_socket: Arc<UdpSocket> = Arc::new(UdpSocket::bind(listen_addr).await?);

  let cancel_token = CancellationToken::new();

  let ct = cancel_token.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c().await.ok();
    info!("Shutdown signal received. Closing connections...");
    ct.cancel();
  });

  info!("Sorting providers with priorities...");
  let mut providers = config.providers.clone();
  providers.sort_by_key(|p| p.priority.unwrap_or(u32::MAX));

  loop {
    let mut provider_active = false;

    for provider in &providers {
      info!("Trying provider with priority {:?}", provider.priority);

      let outbound = UdpSocket::bind("0.0.0.0:0").await?;

      let base_conn = Arc::new(outbound) as Arc<dyn Conn + Send + Sync>;

      let remote_conn = match setup_and_run_provider(provider, base_conn).await {
        Ok(conn) => conn,
        Err(e) => {
          warn!("Failed to setup provider: {}. Trying next...", e);
          continue;
        }
      };

      let targeted_conn = Arc::new(TargetedConn {
        inner: remote_conn,
        remote_addr: peer_addr,
      });

      let secure_conn = if provider.using_dtls_obfuscation {
        dtls_configure(targeted_conn)
          .await
          .context("Failed to configure DTLS connection")?
      } else {
        targeted_conn
      };

      let provider_token = cancel_token.child_token();
      if let Err(e) = run_bridge_group(
        provider,
        listen_socket.clone(),
        secure_conn,
        peer_addr,
        provider_token,
      )
      .await
      {
        error!("Bridge failed: {}. Switching to next provider...", e);
        continue;
      }

      provider_active = true;
      break;
    }

    if !provider_active && !cancel_token.is_cancelled() {
      error!("Not available providers. Retry after 5 seconds...");
      tokio::select! {
        _ = cancel_token.cancelled() => break,
        _ = tokio::time::sleep(Duration::from_secs(5)) => {}
      }
    }
  }

  Ok(())
}
