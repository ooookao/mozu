use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use rand::RngCore;
use reqwest::blocking::{Client, Response};
use security_framework::passwords::{get_generic_password, set_generic_password};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

const SERVICE: &str = "io.github.ooookao.mozu.chatgpt";
const ISSUER: &str = "https://auth.openai.com";
const API: &str = "https://api.openai.com/v1";
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";

pub(crate) type Result<T> = std::result::Result<T, String>;

/// One validated OAuth registration; credentials are stored only in macOS Keychain.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Account {
    pub(crate) client_id: String,
    pub(crate) subject: String,
    pub(crate) email: String,
    access_token: String,
    refresh_token: String,
    id_token: String,
    expires_at: u64,
    scope: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Vault {
    host_id: String,
    pub(crate) accounts: Vec<Account>,
    pub(crate) active: Option<String>,
}

impl Vault {
    pub(crate) fn load() -> Result<Self> {
        match get_generic_password(SERVICE, "accounts") {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| format!("ChatGPT連携情報の保存形式を読めません: {e}")),
            Err(e) if e.code() == -25300 => {
                let vault = Self {
                    host_id: format!("urn:uuid:{}", uuid::Uuid::new_v4()),
                    accounts: Vec::new(),
                    active: None,
                };
                vault.save()?;
                Ok(vault)
            }
            Err(e) => Err(format!("キーチェーンのChatGPT連携情報を読めません: {e}")),
        }
    }

    pub(crate) fn save(&self) -> Result<()> {
        let bytes =
            serde_json::to_vec(self).map_err(|e| format!("連携情報を保存できません: {e}"))?;
        set_generic_password(SERVICE, "accounts", &bytes)
            .map_err(|e| format!("ChatGPT連携情報をキーチェーンへ保存できません: {e}"))
    }

    pub(crate) fn selected(&self) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|a| Some(&a.client_id) == self.active.as_ref())
    }

    fn store_account(&self, account: Account) -> Result<Self> {
        let mut next = self.clone();
        next.active = Some(account.client_id.clone());
        if let Some(index) = next
            .accounts
            .iter()
            .position(|a| a.client_id == account.client_id)
        {
            next.accounts[index] = account;
        } else {
            next.accounts.push(account);
        }
        next.save()?;
        Ok(next)
    }
}

#[derive(Clone, Deserialize)]
pub(crate) struct Model {
    pub(crate) slug: String,
    pub(crate) display_name: String,
    visibility: String,
}

#[derive(Deserialize)]
struct ModelCatalog {
    models: Vec<Model>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: u64,
    scope: Option<String>,
    token_type: String,
}

#[derive(Clone, Deserialize)]
struct Claims {
    sub: String,
    nonce: Option<String>,
    email: Option<String>,
}

pub(crate) fn client() -> Result<Client> {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("OpenAI接続を準備できません: {e}"))
}

fn checked(response: Response, operation: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    // OAuth bodies may contain tokens. Report only typed error fields, never raw bodies.
    #[derive(Deserialize)]
    struct ApiError {
        code: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ErrorField {
        OAuth(String),
        Api(ApiError),
    }
    #[derive(Deserialize)]
    struct ErrorBody {
        error: ErrorField,
    }
    let code = match response.json::<ErrorBody>() {
        Ok(ErrorBody {
            error: ErrorField::OAuth(code),
        }) => code,
        Ok(ErrorBody {
            error: ErrorField::Api(error),
        }) => error.code.unwrap_or_else(|| "unspecified".into()),
        Err(_) => "invalid_error_response".into(),
    };
    Err(format!("{operation}: HTTP {status}, code={code}"))
}

fn clock() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| format!("端末の時刻を確認してください: {e}"))
}

fn random_secret() -> String {
    let mut bytes = [0; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Performs PKCE OAuth on a worker. Only a validated identity can replace a saved account.
pub(crate) fn sign_in(
    vault: &Vault,
    returning: Option<&str>,
    cancelled: &AtomicBool,
    open_browser: impl Fn(String) -> Result<()>,
) -> Result<Vault> {
    let selected = match returning {
        Some(id) => Some(
            vault
                .accounts
                .iter()
                .find(|a| a.client_id == id)
                .ok_or("再連携するChatGPTアカウントがありません")?,
        ),
        None => None,
    };
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("ChatGPT認証のコールバックを開始できません: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}/auth/callback");
    let state = random_secret();
    let nonce = random_secret();
    let verifier = random_secret();
    let client_id = selected
        .map(|a| a.client_id.as_str())
        .unwrap_or("dynamic_agent_client");
    let mut url =
        Url::parse("https://auth.openai.com/api/accounts/authorize").map_err(|e| e.to_string())?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("ext_agent_host_id", vault.host_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", redirect.as_str()),
        ("scope", SCOPE),
        ("resource", API),
        ("state", state.as_str()),
        ("nonce", nonce.as_str()),
        ("code_challenge_method", "S256"),
        ("code_challenge", challenge(&verifier).as_str()),
    ]);
    if let Some(account) = selected {
        if !account.id_token.is_empty() {
            url.query_pairs_mut()
                .append_pair("id_token_hint", &account.id_token);
        }
    } else {
        url.query_pairs_mut().append_pair("agent_name_hint", "Mozu");
    }
    open_browser(url.into())?;
    let callback = receive_callback(&listener, &state, cancelled)?;
    let issued = callback
        .client_id
        .as_deref()
        .or(selected.map(|a| a.client_id.as_str()))
        .ok_or("ChatGPTからclient_idが返らず、登録を完了できませんでした")?;
    if issued == "dynamic_agent_client" || selected.is_some_and(|a| a.client_id != issued) {
        return Err("ChatGPT認証のclient_idが一致しません".into());
    }
    let http = client()?;
    let tokens: TokenResponse = checked(
        http.post("https://auth.openai.com/api/accounts/oauth/token")
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", issued),
                ("code", callback.code.as_str()),
                ("code_verifier", verifier.as_str()),
                ("redirect_uri", redirect.as_str()),
                ("resource", API),
            ])
            .send()
            .map_err(|e| format!("ChatGPT認証コードの交換に失敗しました: {}", e.without_url()))?,
        "ChatGPT認証コード交換",
    )?
    .json()
    .map_err(|e| format!("ChatGPTトークン応答を読めません: {e}"))?;
    validate_scope(&tokens)?;
    let id_token = tokens
        .id_token
        .ok_or("ChatGPTからIDトークンが返りませんでした")?;
    let claims = verify_identity(&http, &id_token, issued, &nonce)?;
    if selected.is_some_and(|a| a.subject != claims.sub) {
        return Err("再連携したChatGPTアカウントが元のアカウントと異なります".into());
    }
    if cancelled.load(Ordering::SeqCst) {
        return Err("ChatGPTへのログインをキャンセルしました".into());
    }
    vault.store_account(Account {
        client_id: issued.to_owned(),
        subject: claims.sub,
        email: claims.email.unwrap_or_else(|| "ChatGPTアカウント".into()),
        access_token: tokens.access_token,
        refresh_token: tokens
            .refresh_token
            .ok_or("ChatGPTから更新トークンが返りませんでした")?,
        id_token,
        expires_at: clock()? + tokens.expires_in,
        scope: tokens.scope.ok_or("ChatGPTの許可範囲がありません")?,
    })
}

struct Callback {
    code: String,
    client_id: Option<String>,
}

fn parse_callback(target: &str, state: &str) -> Result<Callback> {
    let url = Url::parse(&format!("http://127.0.0.1{target}")).map_err(|e| e.to_string())?;
    if url.path() != "/auth/callback" {
        return Err("ChatGPT認証のコールバックパスが異なります".into());
    }
    let pairs: Vec<_> = url.query_pairs().collect();
    if ["state", "code", "client_id", "error"]
        .iter()
        .any(|key| pairs.iter().filter(|(k, _)| k == key).count() > 1)
    {
        return Err("ChatGPT認証のコールバックパラメーターが重複しています".into());
    }
    let value = |key: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.to_string())
    };
    if value("state").as_deref() != Some(state) {
        return Err("ChatGPT認証のstateが一致しません".into());
    }
    if value("error").is_some() {
        return Err("ChatGPTへのログインが許可されませんでした".into());
    }
    Ok(Callback {
        code: value("code")
            .filter(|v| !v.is_empty())
            .ok_or("ChatGPT認証コードがありません")?,
        client_id: value("client_id"),
    })
}

fn receive_callback(
    listener: &TcpListener,
    state: &str,
    cancelled: &AtomicBool,
) -> Result<Callback> {
    let deadline = Instant::now() + Duration::from_secs(300);
    while Instant::now() < deadline && !cancelled.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|e| e.to_string())?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .map_err(|e| e.to_string())?;
                let mut line = String::new();
                BufReader::new((&stream).take(8192))
                    .read_line(&mut line)
                    .map_err(|e| format!("認証応答を読めません: {e}"))?;
                let target = line
                    .strip_prefix("GET ")
                    .and_then(|v| v.split_once(" HTTP/1.").map(|(path, _)| path));
                let result = target
                    .ok_or_else(|| "認証のHTTPリクエストが不正です".to_owned())
                    .and_then(|target| parse_callback(target, state));
                let (status, body) = if result.is_ok() {
                    ("200 OK", "ChatGPT login completed. Return to Mozu.")
                } else {
                    (
                        "400 Bad Request",
                        "ChatGPT login could not be completed. Return to Mozu.",
                    )
                };
                write!(stream, "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).map_err(|e| format!("ブラウザへの認証結果送信に失敗しました: {e}"))?;
                let belongs_to_attempt = target
                    .and_then(|target| Url::parse(&format!("http://127.0.0.1{target}")).ok())
                    .is_some_and(|url| {
                        url.path() == "/auth/callback"
                            && url
                                .query_pairs()
                                .any(|(key, value)| key == "state" && value == state)
                    });
                if result.is_ok() || belongs_to_attempt {
                    return result;
                }
                // Unrelated loopback requests must not terminate the pending OAuth attempt.
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(e) => return Err(format!("ChatGPT認証の受信に失敗しました: {e}")),
        }
    }
    Err(if cancelled.load(Ordering::SeqCst) {
        "ChatGPTへのログインをキャンセルしました"
    } else {
        "ChatGPTへのログインがタイムアウトしました"
    }
    .into())
}

fn validate_scope(tokens: &TokenResponse) -> Result<()> {
    if !tokens.token_type.eq_ignore_ascii_case("bearer") {
        return Err("ChatGPTのトークン種別が不正です".into());
    }
    if !tokens
        .scope
        .as_ref()
        .is_some_and(|s| s.split_whitespace().any(|s| s == PLAN_SCOPE))
    {
        return Err("ChatGPTプランの利用が許可されていません。再ログインしてください".into());
    }
    Ok(())
}

fn verify_identity(http: &Client, token: &str, client_id: &str, nonce: &str) -> Result<Claims> {
    let header = decode_header(token).map_err(|_| "ChatGPTのIDトークンヘッダーが不正です")?;
    if header.alg != Algorithm::RS256 {
        return Err("ChatGPTのIDトークン署名方式が不正です".into());
    }
    let kid = header.kid.ok_or("ChatGPTの署名鍵IDがありません")?;
    let jwks: JwkSet = checked(
        http.get("https://auth.openai.com/.well-known/jwks.json")
            .send()
            .map_err(|e| format!("OpenAIの署名鍵を取得できません: {}", e.without_url()))?,
        "OpenAI署名鍵取得",
    )?
    .json()
    .map_err(|e| format!("OpenAIの公開鍵応答が不正です: {e}"))?;
    let jwk = jwks
        .find(&kid)
        .ok_or("ChatGPTの署名に対応する公開鍵がありません")?;
    let key = DecodingKey::from_jwk(jwk).map_err(|e| format!("OpenAIの署名鍵が不正です: {e}"))?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[client_id]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|e| format!("ChatGPTのIDトークン検証に失敗しました: {e}"))?
        .claims;
    if claims.nonce.as_deref() != Some(nonce) || claims.sub.is_empty() {
        return Err("ChatGPTの認証nonceまたはアカウントIDが不正です".into());
    }
    Ok(claims)
}

/// Refreshes and atomically replaces the rotating credential record before inference.
pub(crate) fn authorized(vault: &Vault) -> Result<(Vault, String)> {
    let account = vault.selected().ok_or("ChatGPTへログインしてください")?;
    if account.access_token.is_empty() {
        return Err("ChatGPTへ再ログインしてください".into());
    }
    if !account.scope.split_whitespace().any(|s| s == PLAN_SCOPE) {
        return Err("ChatGPTプランの利用許可がありません".into());
    }
    if account.expires_at > clock()? + 60 {
        return Ok((vault.clone(), account.access_token.clone()));
    }
    let tokens: TokenResponse = checked(
        client()?
            .post("https://auth.openai.com/api/accounts/oauth/token")
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", account.client_id.as_str()),
                ("refresh_token", account.refresh_token.as_str()),
                ("resource", API),
            ])
            .send()
            .map_err(|e| format!("ChatGPTの認証更新に失敗しました: {}", e.without_url()))?,
        "ChatGPT認証更新",
    )?
    .json()
    .map_err(|e| format!("ChatGPTの更新応答が不正です: {e}"))?;
    let next_account = Account {
        access_token: tokens.access_token,
        refresh_token: tokens
            .refresh_token
            .unwrap_or_else(|| account.refresh_token.clone()),
        id_token: tokens.id_token.unwrap_or_else(|| account.id_token.clone()),
        expires_at: clock()? + tokens.expires_in,
        scope: tokens.scope.unwrap_or_else(|| account.scope.clone()),
        ..account.clone()
    };
    if !tokens.token_type.eq_ignore_ascii_case("bearer")
        || !next_account
            .scope
            .split_whitespace()
            .any(|s| s == PLAN_SCOPE)
    {
        return Err("ChatGPTの再認証が必要です".into());
    }
    let token = next_account.access_token.clone();
    Ok((vault.store_account(next_account)?, token))
}

pub(crate) fn list_models(vault: &Vault) -> Result<(Vault, Vec<Model>)> {
    let (vault, token) = authorized(vault)?;
    let catalog: ModelCatalog = checked(
        client()?
            .get(format!("{API}/models"))
            .bearer_auth(token)
            .send()
            .map_err(|e| format!("ChatGPTモデル一覧の取得に失敗しました: {}", e.without_url()))?,
        "ChatGPTモデル一覧",
    )?
    .json()
    .map_err(|e| format!("ChatGPTモデル一覧の応答が不正です: {e}"))?;
    Ok((
        vault,
        catalog
            .models
            .into_iter()
            .filter(|m| m.visibility == "list")
            .collect(),
    ))
}

pub(crate) fn sign_out(vault: &Vault) -> Result<(Vault, Option<String>)> {
    let account = vault
        .selected()
        .ok_or("ログアウトするChatGPTアカウントがありません")?;
    #[derive(Deserialize)]
    struct Discovery {
        revocation_endpoint: Url,
    }
    let revoke = || -> Result<()> {
        let http = client()?;
        let discovery: Discovery = checked(
            http.get("https://auth.openai.com/.well-known/openid-configuration")
                .send()
                .map_err(|e| e.without_url().to_string())?,
            "ChatGPT認証設定取得",
        )?
        .json()
        .map_err(|e| e.to_string())?;
        if discovery.revocation_endpoint.scheme() != "https"
            || discovery.revocation_endpoint.host_str() != Some("auth.openai.com")
        {
            return Err("ChatGPTの失効先URLが不正です".into());
        }
        checked(
            http.post(discovery.revocation_endpoint)
                .form(&[
                    ("token", account.refresh_token.as_str()),
                    ("token_type_hint", "refresh_token"),
                    ("client_id", account.client_id.as_str()),
                ])
                .send()
                .map_err(|e| e.without_url().to_string())?,
            "ChatGPTログアウト",
        )?;
        Ok(())
    };
    let mut outcome = revoke();
    for delay in [200, 600] {
        if outcome.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(delay));
        outcome = revoke();
    }
    let mut next = vault.clone();
    if let Some(saved) = next
        .accounts
        .iter_mut()
        .find(|a| a.client_id == account.client_id)
    {
        saved.access_token.clear();
        saved.refresh_token.clear();
        saved.id_token.clear();
        saved.expires_at = 0;
    }
    next.save()?;
    Ok((next, outcome.err().map(|e| format!("端末ではログアウト済みですが、OpenAI側の失効を確認できませんでした。ChatGPTの設定から連携を解除してください。{e}"))))
}

pub(crate) fn is_connected(vault: &Vault) -> bool {
    vault.selected().is_some_and(|a| !a.access_token.is_empty())
}

pub(crate) fn response(response: Response) -> Result<Response> {
    checked(response, "ChatGPT整形")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc7636() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn callback_requires_matching_state_path_and_unique_parameters() {
        assert!(parse_callback("/auth/callback?state=s&code=c&client_id=oaiapp_x", "s").is_ok());
        for target in [
            "/auth/callback?state=other&code=c",
            "/callback?state=s&code=c",
            "/auth/callback?state=s&code=c&code=d",
            "/auth/callback?state=s&error=access_denied",
        ] {
            assert!(parse_callback(target, "s").is_err());
        }
    }
}
