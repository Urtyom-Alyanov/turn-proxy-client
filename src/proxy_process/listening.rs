use crate::configuration::configuration::{AppConfiguration, DefaultProvider, ProviderConfiguration, ProviderDetails};

use std::net::{SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use tokio::net::UdpSocket;
use tokio::task::{JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use webrtc_util::Conn;
use crate::dtls::dtls_configure::dtls_configure;
use crate::providers::vk::{get_vk_call_id_from_link, get_vk_calls_turn_credentials};
use crate::providers::yandex::{get_yandex_call_id_from_link, get_yandex_telebridge_turn_credentials};
use crate::proxy_process::proxy_flow::{ProxyBridge};
use crate::proxy_process::turn_configure::{turn_configure, TurnCredentials};

pub async fn listening(config: AppConfiguration) -> Result<()> {
  let listen_addr: SocketAddr = config.common.listening_on.parse()
    .context("'listening-on' is not a valid socket address")?;
  let peer_addr: SocketAddr = config.common.peer_addr.parse()
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

      let remote_conn = match setup_and_run_provider(provider).await {
        Ok(conn) => conn,
        Err(e) => {
          warn!("Failed to setup provider: {}. Trying next...", e);
          continue;
        }
      };

      let final_conn = if provider.using_dtls_obfuscation {
        dtls_configure(remote_conn).await.context("Failed to configure DTLS connection")?
      } else {
        remote_conn
      };

      let provider_token = cancel_token.child_token();
      if let Err(e) = run_bridge_group(provider, listen_socket.clone(), final_conn, peer_addr, provider_token).await {
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

async fn run_bridge_group(
  provider: &ProviderConfiguration,
  listen_socket: Arc<UdpSocket>,
  remote_conn: Arc<dyn Conn + Send + Sync>,
  peer_addr: SocketAddr,
  token: CancellationToken,
) -> Result<()> {
  let threads = provider.threads.unwrap_or(1) as usize;
  let mut handles: Vec<JoinHandle<Result<()>>> = vec![];
  let local_conn = listen_socket as Arc<dyn Conn + Send + Sync>;

  for i in 0..threads {
    let flow_id = format!("P-{}-T-{}", provider.priority.unwrap_or(0), i);
    let bridge = ProxyBridge::new(flow_id, token.clone());

    let up = bridge.run_upstream(local_conn.clone(), remote_conn.clone(), peer_addr).await?;
    let down = bridge.run_downstream(local_conn.clone(), remote_conn.clone()).await?;

    handles.push(up);
    handles.push(down);
  }

  if let Some(result) = futures_util::future::select_all(handles).await.0.ok() {
    result?;
  }

  token.cancel();
  Ok(())
}

fn get_call_id_from_link<'a>(kind: &DefaultProvider, link: &'a str) -> Result<&'a str> {
  match kind {
    DefaultProvider::VkCalls => {
      get_vk_call_id_from_link(link)
    }
    DefaultProvider::YandexTelemost => {
      get_yandex_call_id_from_link(link)
    }
  }
}

async fn fetch_creds(details: &ProviderDetails) -> Result<TurnCredentials> {
  match details {
    ProviderDetails::Default { kind, link } => {
      let call_id = get_call_id_from_link(kind, link)?;

      match kind {
        DefaultProvider::VkCalls => {
          get_vk_calls_turn_credentials(call_id.to_owned(), None).await
        }
        DefaultProvider::YandexTelemost => {
          get_yandex_telebridge_turn_credentials(call_id, None).await
        }
      }
    }
    ProviderDetails::Custom { realm, password, username, stun_address, turn_address } => {
      Ok(TurnCredentials {
        password: password.to_owned(),
        realm: realm.to_owned(),
        username: username.to_owned(),
        turn_addr: turn_address.to_owned(),
        stun_addr: stun_address.to_owned().into()
      })
    }
    ProviderDetails::Direct => {
      Err(anyhow!("Direct provider does not require TURN credentials fetching"))
    }
  }
}

async fn setup_and_run_provider(provider: &ProviderConfiguration) -> Result<Arc<dyn Conn + Send + Sync>> {
  match &provider.details {
    ProviderDetails::Direct => {
      info!("Try to direct connection with server...");
      let outbound = UdpSocket::bind("0.0.0.0:0").await?;
      Ok(Arc::new(outbound) as Arc<dyn Conn + Send + Sync>)
    }
    _ => {
      info!("Try to connection via TURN...");

      let creds = fetch_creds(&provider.details).await?;
      let turn = turn_configure(creds).await?;

      info!("TURN connection established successfully");

      Ok(Arc::new(turn.conn) as Arc<dyn Conn + Send + Sync>)
    }
  }
}