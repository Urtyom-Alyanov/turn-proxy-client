pub mod providers;
pub mod args;
pub mod logging;

use tokio::net::{UdpSocket};
use std::sync::Arc;
use std::net::SocketAddr;

#[tokio::main(flavor = "current_thread")]
fn main() {
  let args = Args::parse();
}