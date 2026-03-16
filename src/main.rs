pub mod providers;
pub mod logging;
pub mod configuration;

use crate::configuration::init_configuration::init_config;
use crate::logging::init_logger;

use anyhow::Result;
use tracing::info;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let _guard = init_logger();
  let config = init_config()?;

  info!("Hello, world!");
  Ok(())
}