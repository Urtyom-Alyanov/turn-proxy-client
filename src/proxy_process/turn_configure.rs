use anyhow::Result;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, info};
use turn::client::{ClientConfig as TurnClientConfig, Client as TurnClient};
use turn::relay::relay_none::RelayAddressGeneratorNone;
use webrtc_util::Conn;

pub struct TurnCredentials {
  pub username: String,
  pub password: String,
  pub realm: String,
  pub turn_addr: String,
  pub stun_addr: Option<String>
}

pub struct TurnConnection<C: Conn> {
  client: TurnClient,
  pub conn: C
}

/// Настройка подключения к TURN-серверу с полученными учётными данными от поставщика
pub async fn turn_configure(
  credentials: TurnCredentials,
) -> Result<TurnConnection<impl Conn>> {
  let turn_sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
  debug!("Socket {} initialised successfully", turn_sock.local_addr()?);

  debug!("Setting up connection with {}...", &credentials.turn_addr);

  let client_config = TurnClientConfig {
    stun_serv_addr: credentials.stun_addr.unwrap_or(credentials.turn_addr.clone()),
    turn_serv_addr: credentials.turn_addr,
    username: credentials.username,
    password: credentials.password,
    realm: credentials.realm,
    conn: turn_sock,
    ..Default::default()
  };

  let client = TurnClient::new(client_config).await?;

  client.listen().await?;

  debug!("Connected to TURN server.");

  let relay_conn = client.allocate().await?;

  info!("Relay connection at {} allocated!", relay_conn.local_addr()?);

  Ok(TurnConnection {
    client,
    conn: relay_conn
  })
}