//! Google OAuth 2.0 with local HTTP redirect server.
//!
//! Flow:
//!   1. Start a tiny_http server on localhost:8080 waiting for /callback
//!   2. Open (or print) the Google auth URL
//!   3. On receiving the code, exchange it for tokens
//!   4. Store tokens in keyring

use anyhow::{Context, Result};
use tokio::sync::oneshot;

const KEYRING_SERVICE: &str = "khamoshchat";
const TOKEN_USER: &str = "google_access_token";

/// Minimal OAuth redirect server that captures the authorization code.
async fn run_redirect_server(
    port: u16,
    code_tx: oneshot::Sender<String>,
) -> Result<()> {
    use tiny_http::Response;
    let server = tiny_http::Server::http(format!("127.0.0.1:{port}"))
        .map_err(|e| anyhow::anyhow!("redirect server error: {e}"))?;
    for request in server.incoming_requests() {
        let url = request.url().to_string();
        if url.starts_with("/callback?code=") {
            let code = url
                .strip_prefix("/callback?code=")
                .and_then(|s| s.split('&').next())
                .unwrap_or("")
                .to_string();
            let _ = code_tx.send(code);
            let _ = request.respond(
                Response::from_string("Authenticated! You can close this tab.")
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                    ),
            );
            break;
        }
    }
    Ok(())
}

/// Perform the full Google OAuth flow.
/// If `no_open` is true, prints the URL instead of launching a browser.
pub async fn google_oauth(no_open: bool) -> Result<()> {
    let client_id = std::env::var("KH_GOOGLE_CLIENT_ID")
        .context("KH_GOOGLE_CLIENT_ID not set")?;
    let client_secret = std::env::var("KH_GOOGLE_CLIENT_SECRET")
        .context("KH_GOOGLE_CLIENT_SECRET not set")?;

    let port = 8080u16;
    let redirect_uri = format!("http://localhost:{port}/callback");
    let scope = "openid email profile";

    // Build the authorization URL
    let state = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        rand::random::<[u8; 16]>(),
    );

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?client_id={}\
         &redirect_uri={}\
         &scope={}\
         &state={}\
         &response_type=code\
         &access_type=offline",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(scope),
        &state,
    );

    if no_open {
        println!("Open this URL in your browser:\n  {auth_url}");
    } else {
        println!("Opening browser for OAuth...");
        open::that(&auth_url).ok();
    }

    // Start redirect server
    let (code_tx, code_rx) = oneshot::channel();
    tokio::spawn(async move {
        if let Err(e) = run_redirect_server(port, code_tx).await {
            tracing::error!("Redirect server error: {e}");
        }
    });

    // Wait for the code
    let code = code_rx.await.context("OAuth cancelled")?;

    // Exchange code for tokens
    let token_resp: TokenResponse = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", &code),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", &redirect_uri),
            ("grant_type", &"authorization_code".to_string()),
        ])
        .send()
        .await?
        .json()
        .await
        .context("Token exchange failed")?;

    // Store tokens in keyring
    crate::keyring::set(KEYRING_SERVICE, "google_access_token", &token_resp.access_token)?;
    if let Some(refresh) = token_resp.refresh_token {
        crate::keyring::set(KEYRING_SERVICE, "google_refresh_token", refresh.as_str())?;
    }
    if let Some(id_token) = token_resp.id_token {
        crate::keyring::set(KEYRING_SERVICE, "google_id_token", &id_token)?;
    }

    println!("Authentication complete. Tokens stored in keyring.");
    Ok(())
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
}
