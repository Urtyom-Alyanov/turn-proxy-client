use crate::configuration::configuration::{AppConfiguration, DefaultProvider, ProviderConfiguration, ProviderDetails};

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use webrtc_util::Conn;
use crate::providers::vk::{get_vk_call_id_from_link, get_vk_calls_turn_credentials};
use crate::providers::yandex::{get_yandex_call_id_from_link, get_yandex_telebridge_turn_credentials};
use crate::proxy_process::proxy_flow::proxy_flow;
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
  let mut cancel_set = JoinSet::new();

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
    for provider in &providers {
      info!("Trying provider with priority {:?}", provider.priority);

      let remote_conn: Arc<dyn Conn + Send + Sync> = setup_and_run_provider(provider, &listen_socket).await?;

      let final_conn = if provider.using_dtls_obfuscation {
        remote_conn
      } else {
        remote_conn
      };

      let token = CancellationToken::new();

      let threads = provider.threads.unwrap_or(1) as usize;
      let mut handles = vec![];

      for i in 0..threads {
        let flow_id = format!("PR-{}-THR-{}", provider.priority.unwrap_or(99), i);

        let up = proxy_flow(
          format!("{}-UP", flow_id),
          listen_socket.local_addr()?,
          final_conn.local_addr()?,
          token.clone(),
          listen_socket.clone(),
          final_conn.clone(),
          peer_addr.into()
        );

        handles.push(up);
      }

      match final_conn {
        Ok(_) => {
          break;
        }
        Err(e) => {
          warn!("Provider failed: {}. Moving to next priority...", e);
          continue;
        }
      }
    }

    error!("Not available providers. Retry after 5 seconds...");
    tokio::time::sleep(Duration::from_secs(5)).await;
  }

  Ok(())
}

fn get_call_id_from_link(kind: &DefaultProvider, link: &str) -> &str {
  match kind {
    DefaultProvider::VKCalls => {
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
      let call_id = get_call_id_from_link(kind, link);

      match kind {
        DefaultProvider::VKCalls => {
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

async fn setup_and_run_provider(provider: &ProviderConfiguration, local_socket: &Arc<UdpSocket>) -> Result<Arc<dyn Conn + Send + Sync>> {
  match &provider.details {
    ProviderDetails::Direct => {
      info!("Try to direct connection with server...");
      Ok(local_socket as Arc<dyn Conn + Send + Sync>)
    }
    _ => {
      info!("Try to connection via TURN...");

      let creds = fetch_creds(&provider.details).await?;
      let turn = turn_configure(creds).await?;

      info!("TURN connection established successfully");

      Ok(turn.conn)
    }
  }
}