use anyhow::Result;
use std::{sync::Arc};

use tokio::{net::UdpSocket, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use webrtc_util::Conn;

use crate::{
  configuration::configuration::ProviderConfiguration, proxy_process::proxy_flow::ProxyBridge,
};
use crate::proxy_process::target_conn::TargetedConn;

pub async fn run_bridge_thread(
  provider: &ProviderConfiguration,
  thread_num: usize,
  listen_socket: Arc<UdpSocket>,
  remote_conn: Arc<dyn Conn + Send + Sync>,
  token: CancellationToken,
) -> Result<()> {
  let mut handles: Vec<JoinHandle<Result<()>>> = vec![];
  let local_conn = Arc::new(TargetedConn {
    inner: listen_socket.clone(),
    remote_addr: listen_socket.local_addr()?,
  });

  let flow_id = format!("P-{}-T-{}", provider.priority.unwrap_or(0), thread_num);
  let bridge = ProxyBridge::new(flow_id, token.clone());

  let up = bridge
    .run_upstream(local_conn.clone(), remote_conn.clone())
    .await?;
  let down = bridge
    .run_downstream(local_conn.clone(), remote_conn.clone())
    .await?;

  handles.push(up);
  handles.push(down);

  if let Some(result) = futures_util::future::select_all(handles).await.0.ok() {
    result?;
  }

  token.cancel();
  Ok(())
}
