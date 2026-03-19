use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};

use tokio::{net::UdpSocket, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use webrtc_util::Conn;

use crate::{
  configuration::configuration::ProviderConfiguration, proxy_process::proxy_flow::ProxyBridge,
};

pub async fn run_bridge_group(
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

    let up = bridge
      .run_upstream(local_conn.clone(), remote_conn.clone(), peer_addr)
      .await?;
    let down = bridge
      .run_downstream(local_conn.clone(), remote_conn.clone())
      .await?;

    handles.push(up);
    handles.push(down);
  }

  if let Some(result) = futures_util::future::select_all(handles).await.0.ok() {
    result?;
  }

  token.cancel();
  Ok(())
}
