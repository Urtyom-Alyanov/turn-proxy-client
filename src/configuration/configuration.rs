use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CommonConfiguration {
  /// Адрес входа/выхода
  pub listening_on: String,
  /// Конечный сервер
  pub peer_addr: String,
}

#[derive(Deserialize, Debug)]
pub struct ProviderConfiguration {
  /// Не использовать UDP для TURN сервера поставщика (может понизить скорость), не знаю зачем
  /// это кому-то, на другие параметры не влияет
  pub not_using_udp: bool,
  /// Не использовать DTLS обфускацию для поставщика (может увеличить скорость, но также может
  /// увеличить шанс на блокировку
  ///
  /// НЕ РЕКОМЕНДУЕТСЯ ОТКЛЮЧАТЬ
  pub not_using_dtls_obfuscation: bool,
  /// Количество потоков, выглядит как количество участников в конференции, большие значения могут
  /// вызвать подозрения, так как с одного IP адреса идёт подключается одновременно к одному звонку
  /// условно 16 человек, что, довольно, странно.
  ///
  /// Не рекомендуется указывать большие значения, однако может существенно увеличить скорость, если
  /// со стороны поставщика имеется ограничение по скорости для участника конференции.
  pub threads: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct AppConfiguration {
  common: CommonConfiguration,
  provider: ProviderConfiguration,
}