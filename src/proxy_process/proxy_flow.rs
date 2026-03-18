use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};
use webrtc_util::Conn;

/// Выделение потока с прокси-процессом, если указан `peer_addr`, то ожидается, что трафик идёт через
/// TURN, иначе - напрямую
pub fn proxy_flow(
  flow_name: String,
  from_addr: SocketAddr, // Local or TURN
  to_addr: SocketAddr, // Local or TURN
  cancellation_token: CancellationToken,
  from_flow: Arc<dyn Conn + Send + Sync>,
  to_flow: Arc<dyn Conn + Send + Sync>,
  peer_addr: Option<SocketAddr>,
) -> JoinHandle<anyhow::Result<()>> {
  tokio::spawn(async move {
    let mut buf = [0u8; 2048];

    loop {
      match from_flow.recv_from(&mut buf).await {
        Ok((n, src_addr)) if n > 0 => {
          debug!("[{}] Received {} bytes via {} from {}", flow_name, n, from_addr, src_addr);
          
          if n >= buf.len() {
            warn!("[{}] Packet from {} is too large for buffer ({})", flow_name, src_addr, n);
          }

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
        _ => break,
      }
    }

    cancellation_token.cancel();
    Ok(())
  })
}