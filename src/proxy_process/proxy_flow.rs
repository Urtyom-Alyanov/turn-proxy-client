use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use webrtc_util::Conn;

type SharedPeerAddr = Arc<RwLock<Option<SocketAddr>>>;
pub struct ProxyBridge {
  pub flow_name: String,
  pub cancellation_token: CancellationToken,
  last_client_addr: SharedPeerAddr,
}

impl ProxyBridge {
  pub fn new(flow_name: String, cancellation_token: CancellationToken) -> Self {
    Self {
      flow_name,
      cancellation_token,
      last_client_addr: Arc::new(RwLock::new(None)),
    }
  }

  pub async fn run_upstream(
    &self,
    local_conn: Arc<dyn Conn + Send + Sync>,
    remote_conn: Arc<dyn Conn + Send + Sync>,
    remote_peer_addr: SocketAddr,
  ) -> Result<JoinHandle<Result<()>>> {
    let flow_name = format!("{}-UP", &self.flow_name);
    let cancellation_token = self.cancellation_token.clone();
    let last_client_addr = self.last_client_addr.clone();
    let local_addr = local_conn.local_addr()?;
    let remote_addr = remote_conn.local_addr()?;

    Ok(proxy_flow(
      flow_name,

      local_addr,
      remote_addr,

      cancellation_token,

      local_conn,
      remote_conn,

      Some(remote_peer_addr),
      Some(last_client_addr),
      None
    ))
  }

  pub async fn run_downstream(
    &self,
    local_conn: Arc<dyn Conn + Send + Sync>,
    remote_conn: Arc<dyn Conn + Send + Sync>,
  ) -> Result<JoinHandle<Result<()>>> {
    let flow_name = format!("{}-DOWN", &self.flow_name);
    let cancellation_token = self.cancellation_token.clone();
    let last_client_addr = self.last_client_addr.clone();
    let local_addr = local_conn.local_addr()?;
    let remote_addr = remote_conn.local_addr()?;

    Ok(proxy_flow(
      flow_name,

      remote_addr,
      local_addr,

      cancellation_token,

      remote_conn,
      local_conn,

      None,
      None,
      Some(last_client_addr)
    ))
  }
}

/// Выделение потока с прокси-процессом, если указан `peer_addr`, то ожидается, что трафик идёт через
/// TURN, иначе - напрямую
fn proxy_flow(
  flow_name: String,

  from_addr: SocketAddr, // Local or TURN
  to_addr: SocketAddr, // Local or TURN

  cancellation_token: CancellationToken,

  from_flow: Arc<dyn Conn + Send + Sync>,
  to_flow: Arc<dyn Conn + Send + Sync>,

  static_remote_addr: Option<SocketAddr>,
  addr_to_store: Option<SharedPeerAddr>,
  addr_to_load: Option<SharedPeerAddr>,
) -> JoinHandle<Result<()>> {
  tokio::spawn(async move {
    let mut buf = [0u8; 2048];

    loop {
      tokio::select! {
        _ = cancellation_token.cancelled() => {
          debug!("[{}] Flow cancelled", flow_name);
          break;
        }
        res = from_flow.recv_from(&mut buf) => {
          match res {
            Ok((n, src_addr)) if n > 0 => {
              debug!("[{}] Received {} bytes via {} from {}", flow_name, n, from_addr, src_addr);

              if n >= buf.len() {
                warn!("[{}] Packet from {} is too large for buffer ({})", flow_name, src_addr, n);
              }

              if let Some(shared) = &addr_to_store {
                let mut guard = shared.write().await;
                if *guard != Some(src_addr) {
                  *guard = Some(src_addr);
                }
              }

              let peer_addr = if let Some(fixed) = static_remote_addr {
                Some(fixed)
              } else if let Some(shared) = &addr_to_load {
                *shared.read().await
              } else {
                None
              };

              let send_result = match peer_addr {
                Some(dest) => to_flow.send_to(&buf[..n], dest).await,
                None => to_flow.send(&buf[..n]).await,
              };

              if let Err(e) = send_result {
                warn!("[{}] Error sending to {} via {} from {}: {}", flow_name, peer_addr.unwrap_or(to_addr), to_addr, from_addr, e);
                break;
              }

              debug!("[{}] Send {} bytes to {} via {}", flow_name, n, peer_addr.unwrap_or(to_addr), to_addr);
            }
            Ok(_) => continue,
            Err(e) => {
              debug!("[{}] Receive error (likely closed): {}", flow_name, e);
              break;
            }
          }
        }
      }
    }

    cancellation_token.cancel();
    Ok(())
  })
}