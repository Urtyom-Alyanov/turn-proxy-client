use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
  #[arg(long, default_value = "127.0.0.1:51820", help = "Выходной поток UDP")]
  pub listening_on: Option<String>,

  #[arg(long, help = "Количество потоков (участников) в видеоконференции")]
  pub threads: Option<i32>,

  #[arg(long, help = "Сервер назначения")]
  pub peer_address: Option<String>,

  #[arg(long, help = "Поставщик TURN сервера")]
  pub provider: Option<String>,

  #[arg(long, help = "Использовать только аргументы строки")]
  pub no_config: bool,

  #[arg(
    long,
    help = "Использовать UDP для подключения к TURN серверу"
  )]
  pub not_using_udp: bool,

  #[arg(
    long,
    help = "Использовать DTLS для шифрования и обфускации трафика"
  )]
  pub not_using_dtls_obfuscation: bool,

  #[arg(
    long,
    default_value = "/etc/turn-proxy/client/config.toml",
    help = "Путь к конфигурационному файлу"
  )]
  pub config: String
}
