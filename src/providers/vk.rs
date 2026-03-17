use crate::proxy_process::turn_configure::TurnCredentials;
use crate::providers::USER_AGENT;

use anyhow::{anyhow, Context, Result, Ok};
use reqwest::Client;
use serde_json::Value;
use uuid::Uuid;

struct CallTokenCredentials {
  name: String,
  call_id: String,
}

const VK_CLIENT_SECRET: &str = "QbYic1K3lEV5kTGiqlq2";
const VK_CLIENT_ID: &str = "6287487";
const OKCDN_APPLICATION_KEY: &str = "CGMMEJLGDIHBABABA";
const VK_REALM: &str = "vk";
const VK_API_VERSION: &str = "5.264";

/// Входит в звонок VK с анонимной учётной записью
pub async fn get_vk_calls_turn_credentials(call_id: &str, with_name: Option<String>) -> Result<TurnCredentials> {
  let client = Client::builder()
    .user_agent(USER_AGENT)
    .build()?;

  let anonymous = CallTokenCredentials {
    call_id: call_id.to_owned(),
    name: with_name.unwrap_or("Гость".to_owned()),
  };

  let (call_token, okcdn_token) = tokio::join!(
    async {
      let token_without_payload = get_anonymous_token(&client, None).await?;
      let payload = get_call_payload(&client, token_without_payload).await?;
      let access_token = get_anonymous_token(&client, payload.into()).await?;
      let call_token = get_call_token(&client, access_token, anonymous.into()).await?;
      Ok::<_>(call_token)
    },
    get_okcdn_anonymous_token(&client)
  );

  let call_token = call_token?;
  let okcdn_token = okcdn_token?;

  Ok(join_into_video_conversation(&client, call_id, call_token, okcdn_token).await?)
}


/// Позволяет получить анонимный токен пользователя ВКонтакте. Есть два разных случая:
///
/// 1. Без `call_payload`, получает стандартный токен.
/// 2. С `call_payload`, с которым можно уже войти в звонок.
async fn get_anonymous_token(client: &Client, call_payload: Option<String>) -> Result<String> {
  let url = "https://login.vk.ru/?act=get_anonym_token";
  let mut body = vec![
    ("client_secret", VK_CLIENT_SECRET),
    ("client_id", VK_CLIENT_ID),
    ("app_id", VK_CLIENT_ID),
    ("version", "1"),
  ];

  if let Some(payload) = call_payload {
    body.push(("payload", payload.as_str()));
    body.push(("token_type", "messages"));
  } else {
    body.push(("scopes", "audio_anonymous,video_anonymous,photos_anonymous,profile_anonymous"));
    body.push(("isApiOauthAnonymEnabled", "false"));
  }

  let resp = client.post(url)
    .form(&body)
    .send()
    .await?
    .json::<Value>()
    .await?;

  let token = resp["data"]["access_token"].as_str().ok_or_else(|| anyhow!("Failed to get anonym token from response"))?;

  Ok(token.to_owned())
}

/// Позволяет получить `call_payload` для токена
async fn get_call_payload(client: &Client, access_token: String) -> Result<String> {
  let url = "https://api.vk.ru/method/calls.getAnonymousAccessTokenPayload";

  let body = vec![
    ("client_id", VK_CLIENT_ID),
    ("v", VK_API_VERSION),
    ("access_token", access_token.as_str()),
  ];

  let resp = client.post(url).form(&body).send().await?.json::<Value>().await?;

  Ok(resp["response"]["payload"].as_str()
    .ok_or_else(|| anyhow!("Failed to get call payload from response"))?.to_owned())
}

/// Позволяет получить ключ для подключения к звонку (`call_token`)
async fn get_call_token(client: &Client, access_token: String, credentials: CallTokenCredentials) -> Result<String> {
  let url = "https://api.vk.ru/method/calls.getAnonymousAccessTokenPayload";
  let join_link = format!("https://vk.com/call/join/{}", credentials.call_id);

  let body = vec![
    ("client_id", VK_CLIENT_ID),
    ("v", VK_API_VERSION),
    ("access_token", access_token.as_str()),
    ("name", &credentials.name),
    ("vk_join_link", &join_link),
  ];

  let resp = client.post(url)
    .form(&body)
    .send().await?.json::<Value>().await?;

  Ok(resp["response"]["token"].as_str()
    .ok_or_else(|| anyhow!("Failed to get call token from response"))?.to_owned())
}

/// Получает OKCDN токен для звонка
async fn get_okcdn_anonymous_token(client: &Client) -> Result<String> {
  let url = "https://calls.okcdn.ru/fb.do";
  let session_data = format!("{{\"version\":2,\"device_id\":\"{}\",\"client_version\":1.1,\"client_type\":\"SDK_JS\"}}", Uuid::new_v4());

  let body = vec![
    ("method", "auth.anonymLogin"),
    ("session_data", &session_data),
    ("format", "JSON"),
    ("application_key", OKCDN_APPLICATION_KEY)
  ];

  let resp = client.post(url).form(&body).send().await?.json::<Value>().await?;

  Ok(resp["session_key"].as_str().ok_or(anyhow!("Failed to get okcdn_token from response"))?.to_owned())
}

/// Входит в видео конференцию ВКонтакте, получая тем самым учётные данные для TURN-сервера
async fn join_into_video_conversation(
  client: &Client,
  call_id: &str,
  call_token: String,
  okcdn_token: String
) -> Result<TurnCredentials> {
  let url = "https://calls.okcdn.ru/fb.do";

  let body = vec![
    ("joinLink", call_id),
    ("isVideo", "false"),
    ("protocolVersion", "5"),
    ("anonymToken", call_token.as_str()),
    ("method", "vchat.joinConversationByLink"),
    ("format", "JSON"),
    ("application_key", OKCDN_APPLICATION_KEY),
    ("session_key", okcdn_token.as_str()),
  ];

  let resp = client.post(url)
    .form(&body)
    .send().await?.json::<Value>().await?;

  let turn_data = &resp["turn_server"];

  let turn_url = turn_data["urls"][0].as_str().context("No turn URL")?;

  let turn_addr = turn_url
    .trim_start_matches("turn:")
    .trim_start_matches("turns:")
    .split('?')
    .next()
    .unwrap_or("")
    .to_owned();

  Ok(TurnCredentials {
    username: turn_data["username"].as_str().context("`username` does not contained in received data")?.to_owned(),
    realm: VK_REALM.to_owned(),
    password: turn_data["credential"].as_str().context("`credential` (TURN password) does not contained in received data")?.to_string(),
    stun_addr: turn_addr.clone().into(),
    turn_addr,
  })
}