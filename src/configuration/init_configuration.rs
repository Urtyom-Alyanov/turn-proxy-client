use crate::configuration::args::Args;

use clap::Parser;
use anyhow::{Result};

pub fn init_config() -> Result<()> {
  let args = Args::parse();
  
  Ok(())
}