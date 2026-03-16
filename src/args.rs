use clap::Parser;
#[derive(Parser, Debug)]
pub struct Args {
  #[arg(long, default_value = "127.0.0.1:51820", description = "Выходной поток UDP")]
  pub listening_on: Option<String>,

  #[arg(long, description = "Количество потоков (участников) в видеоконференции")]
  pub threads: Option<i32>,

  #[arg(long, description = "Сервер назначения")]
  pub peer_address: Option<String>,

  #[arg(long, description = "Поставщик TURN сервера")]
  pub provider: Option<String>,

  #[arg(long, description = "Использовать только аргументы строки")]
  pub no_config: bool,

  #[arg(long, default_value = true, description = "Использовать UDP для подключения к TURN серверу")]
  pub using_udp: bool,

  #[arg(long, default_value = true, description = "Использовать DTLS для шифрования и обфускации трафика")]
  pub using_dtls_obfuscation: bool,

  #[arg(long, default_value = "/etc/turn-proxy/client/config.toml", description = "Путь к конфигурационному файлу")]
  pub config: String,
}