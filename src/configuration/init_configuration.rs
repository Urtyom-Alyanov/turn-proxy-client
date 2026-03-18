use std::fs;
use crate::configuration::args::{Args, ProviderType};
use crate::configuration::configuration::{AppConfiguration, DefaultProvider, ProviderConfiguration, ProviderDetails};

use clap::Parser;
use anyhow::{Context, Result};

pub fn init_config() -> Result<AppConfiguration> {
  let args = Args::parse();

  let mut config = if !args.no_config {
    let content = fs::read_to_string(&args.config)
      .context(format!("Read configuration file error: {}", args.config))?;
    toml::from_str::<AppConfiguration>(&content)
      .context(format!("TOML configuration parse error (path: {})", args.config))?
  } else {
    AppConfiguration::default()
  };

  if let Some(listening) = args.listening_on {
    config.common.listening_on = listening;
  }
  if let Some(peer) = args.peer_addr {
    config.common.peer_addr = peer;
  }

  if let Some(provider_type) = args.provider_type {
    let common_args = args.provider_common.unwrap_or_default();

    let details = match provider_type {
      ProviderType::Direct => ProviderDetails::Direct,
      ProviderType::Default { kind, link } => ProviderDetails::Default { kind, link },
      ProviderType::Custom { username, password, turn_address, stun_address, realm } => {
        ProviderDetails::Custom {
          username,
          password,
          turn_address,
          stun_address,
          realm
        }
      }
    };

    let provider_config = ProviderConfiguration {
      priority: 0.into(),
      using_udp: common_args.using_udp,
      using_dtls_obfuscation: common_args.using_dtls_obfuscation,
      details,
      threads: common_args.threads.map(|t| t as i32),
    };

    config.providers = vec![provider_config];
  }

  for provider in &mut config.providers {
    if provider.threads.is_none() {
      provider.threads = match &provider.details {
        ProviderDetails::Default { kind: DefaultProvider::VKCalls, .. } => Some(16),
        ProviderDetails::Direct => None,
        _ => Some(1),
      };
    }
  }

  Ok(config)
}