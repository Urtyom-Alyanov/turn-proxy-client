pub mod providers;
pub mod logging;
pub mod configuration;
pub mod dtls;
pub mod proxy_process;

use crate::configuration::init_configuration::init_config;
use crate::logging::init_logger;

use anyhow::Result;
use crate::dtls::dtls_configure::dtls_configure;
use crate::proxy_process::listening::listening;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let _guard = init_logger();
  let config = init_config()?;
  let dtls_config = dtls_configure()?;
  
  listening(config, dtls_config).await?;

  Ok(())
}