// ─────────────────────────────────────────────────────────────────────────────
// audio/spotify.rs — cliente de Spotify Connect via Web API con PKCE OAuth
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{anyhow, Result};
use std::time::{SystemTime, UNIX_EPOCH};

const SCOPES: &str =
    "user-read-playback-state user-modify-playback-state user-read-currently-playing playlist-read-private playlist-read-collaborative";

// construye el redirect URI a partir de la URL base del servidor
pub fn redirect_uri(server_url: &str) -> String {
    let base = server_url.trim_end_matches('/');
    format!("{}/spotify/callback", base)
}

// jala el client_id del servidor — así los usuarios no tienen que configurar nada
pub fn fetch_client_id(server_url: &str) -> Result<String> {
    let url = format!("{}/spotify/config", server_url.trim_end_matches('/'));
    let resp = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(8))
        .call()
        .map_err(|e| anyhow!("Could not reach Questline server: {}", e))?;
    let json: serde_json::Value = resp.into_json()?;
    json["client_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Server returned no Spotify client_id"))
}

#[derive(Debug, Clone)]
pub struct SpotifyPlaylist {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub track_count: u32,
}

#[derive(Debug, Clone)]
pub struct SpotifyNowPlaying {
    pub track: String,
    pub artist: String,
    pub is_playing: bool,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// genera verifier (43 bytes URL-safe) y challenge (SHA-256 base64url del verifier)
pub fn generate_pkce() -> (String, String) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // semilla combinando pid + tiempo — no necesitamos crypto-rand para el verifier
    let seed = {
        let mut h = DefaultHasher::new();
        std::process::id().hash(&mut h);
        unix_now().hash(&mut h);
        h.finish()
    };

    // verifier: 64 chars URL-safe a partir del seed
    let chars: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut state = seed;
    let verifier: String = (0..64)
        .map(|_| {
            // xorshift para no repetir el mismo carácter
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            chars[(state as usize) % chars.len()] as char
        })
        .collect();

    // challenge: SHA-256 del verifier codificado en base64url
    // Rust stdlib no trae SHA-256, así que implementamos el S256 usando una versión simple
    let challenge = sha256_base64url(verifier.as_bytes());
    (verifier, challenge)
}

// SHA-256 puro en Rust sin dependencias externas
fn sha256_base64url(data: &[u8]) -> String {
    let hash = sha256(data);
    // base64url sin padding
    let b64 = base64url_encode(&hash);
    b64
}

fn base64url_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        out.push(TABLE[((b0 >> 2) & 0x3F) as usize] as char);
        out.push(TABLE[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < data.len() {
            out.push(TABLE[(((b1 & 0xF) << 2) | (b2 >> 6)) as usize] as char);
        }
        if i + 2 < data.len() {
            out.push(TABLE[(b2 & 0x3F) as usize] as char);
        }
        i += 3;
    }
    out
}

// SHA-256 estándar — constantes y lógica del RFC 4634
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, b) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] =
            [h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let tmp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let tmp2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e;
            e = d.wrapping_add(tmp1);
            d = c; c = b; b = a;
            a = tmp1.wrapping_add(tmp2);
        }
        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, &word) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub fn build_auth_url(client_id: &str, challenge: &str, state: &str, redirect_uri: &str) -> String {
    format!(
        "https://accounts.spotify.com/authorize?client_id={}&response_type=code\
         &redirect_uri={}&code_challenge_method=S256&code_challenge={}\
         &scope={}&state={}",
        url_encode(client_id),
        url_encode(redirect_uri),
        url_encode(challenge),
        url_encode(SCOPES),
        url_encode(state),
    )
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", byte));
            }
        }
    }
    out
}

// intercambia el código de autorización por tokens de acceso y refresco
pub fn exchange_code(
    client_id: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<(String, String, u64)> {
    let body = format!(
        "client_id={}&grant_type=authorization_code&code={}&redirect_uri={}&code_verifier={}",
        url_encode(client_id),
        url_encode(code),
        url_encode(redirect_uri),
        url_encode(verifier),
    );

    let resp = ureq::post("https://accounts.spotify.com/api/token")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .timeout(std::time::Duration::from_secs(10))
        .send_string(&body)
        .map_err(|e| anyhow!("Token exchange failed: {}", e))?;

    let json: serde_json::Value = resp.into_json()?;
    let access_token = json["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("No access_token in response"))?
        .to_string();
    let refresh_token = json["refresh_token"]
        .as_str()
        .ok_or_else(|| anyhow!("No refresh_token in response"))?
        .to_string();
    let expires_in = json["expires_in"].as_u64().unwrap_or(3600);
    let expiry = unix_now() + expires_in - 60;

    Ok((access_token, refresh_token, expiry))
}

// renueva el access token usando el refresh token
pub fn refresh_access_token(client_id: &str, refresh_tok: &str) -> Result<(String, u64)> {
    let body = format!(
        "client_id={}&grant_type=refresh_token&refresh_token={}",
        url_encode(client_id),
        url_encode(refresh_tok),
    );

    let resp = ureq::post("https://accounts.spotify.com/api/token")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .timeout(std::time::Duration::from_secs(10))
        .send_string(&body)
        .map_err(|e| anyhow!("Token refresh failed: {}", e))?;

    let json: serde_json::Value = resp.into_json()?;
    let access_token = json["access_token"]
        .as_str()
        .ok_or_else(|| anyhow!("No access_token in refresh response"))?
        .to_string();
    let expires_in = json["expires_in"].as_u64().unwrap_or(3600);
    let expiry = unix_now() + expires_in - 60;

    Ok((access_token, expiry))
}

// error especial para indicar token inválido — el caller debe limpiar los tokens y re-autenticar
pub const ERR_UNAUTHORIZED: &str = "SPOTIFY_UNAUTHORIZED";

pub fn get_playlists(access_token: &str) -> Result<Vec<SpotifyPlaylist>> {
    let resp = match ureq::get("https://api.spotify.com/v1/me/playlists?limit=50")
        .set("Authorization", &format!("Bearer {}", access_token))
        .timeout(std::time::Duration::from_secs(10))
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(401, _)) => return Err(anyhow!(ERR_UNAUTHORIZED)),
        // 403 = el app de Spotify no tiene "Web API" habilitado en developer.spotify.com
        Err(ureq::Error::Status(403, _)) => {
            return Err(anyhow!("Spotify 403: enable 'Web API' in your Spotify Developer App settings"));
        }
        Err(e) => return Err(anyhow!("Network error fetching playlists: {}", e)),
    };

    let json: serde_json::Value = resp.into_json()?;
    let items = json["items"].as_array().cloned().unwrap_or_default();

    let playlists = items
        .iter()
        .filter_map(|item| {
            let id = item["id"].as_str()?.to_string();
            let name = item["name"].as_str().unwrap_or("Untitled").to_string();
            let uri = item["uri"].as_str().unwrap_or("").to_string();
            let track_count = item["tracks"]["total"].as_u64().unwrap_or(0) as u32;
            Some(SpotifyPlaylist { id, name, uri, track_count })
        })
        .collect();

    Ok(playlists)
}

pub fn start_playlist(access_token: &str, playlist_uri: &str) -> Result<()> {
    let body = serde_json::json!({ "context_uri": playlist_uri });
    let resp = ureq::put("https://api.spotify.com/v1/me/player/play")
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(10))
        .send_json(body);

    match resp {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(404, _)) => Err(anyhow!(
            "No active Spotify device found. Open Spotify on any device first."
        )),
        Err(ureq::Error::Status(403, _)) => Err(anyhow!(
            "Playback requires Spotify Premium."
        )),
        Err(e) => Err(anyhow!("Playback failed: {}", e)),
    }
}

pub fn get_now_playing(access_token: &str) -> Result<Option<SpotifyNowPlaying>> {
    let resp = ureq::get("https://api.spotify.com/v1/me/player/currently-playing")
        .set("Authorization", &format!("Bearer {}", access_token))
        .timeout(std::time::Duration::from_secs(8))
        .call();

    match resp {
        Ok(r) => {
            if r.status() == 204 {
                return Ok(None); // nada reproduciéndose
            }
            let json: serde_json::Value = r.into_json()?;
            let is_playing = json["is_playing"].as_bool().unwrap_or(false);
            let track = json["item"]["name"]
                .as_str()
                .unwrap_or("Unknown Track")
                .to_string();
            let artist = json["item"]["artists"][0]["name"]
                .as_str()
                .unwrap_or("Unknown Artist")
                .to_string();
            Ok(Some(SpotifyNowPlaying { track, artist, is_playing }))
        }
        Err(ureq::Error::Status(204, _)) => Ok(None),
        // 401 = token expirado, hay que re-autenticar
        Err(ureq::Error::Status(401, _)) => Err(anyhow!(ERR_UNAUTHORIZED)),
        // 403 = sin Premium o sin dispositivo activo — no es error de token
        Err(ureq::Error::Status(403, _)) => Ok(None),
        Err(e) => Err(anyhow!("Failed to get now playing: {}", e)),
    }
}

pub fn pause_playback(access_token: &str) -> Result<()> {
    ureq::put("https://api.spotify.com/v1/me/player/pause")
        .set("Authorization", &format!("Bearer {}", access_token))
        .timeout(std::time::Duration::from_secs(8))
        .call()
        .map(|_| ())
        .map_err(|e| anyhow!("Pause failed: {}", e))
}

pub fn resume_playback(access_token: &str) -> Result<()> {
    ureq::put("https://api.spotify.com/v1/me/player/play")
        .set("Authorization", &format!("Bearer {}", access_token))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(8))
        .send_string("{}")
        .map(|_| ())
        .map_err(|e| anyhow!("Resume failed: {}", e))
}

pub fn skip_to_next(access_token: &str) -> Result<()> {
    ureq::post("https://api.spotify.com/v1/me/player/next")
        .set("Authorization", &format!("Bearer {}", access_token))
        .timeout(std::time::Duration::from_secs(8))
        .call()
        .map(|_| ())
        .map_err(|e| anyhow!("Skip failed: {}", e))
}

// consulta el servidor de Questline cada segundo hasta recibir el código OAuth
// — el navegador ya entregó el código al callback del servidor, aquí solo lo retiramos
pub fn start_code_poller(server_url: String, state: String) -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let poll_url = format!(
            "{}/spotify/token?state={}",
            server_url.trim_end_matches('/'),
            state
        );
        // timeout de 5 minutos — si el user no autoriza en ese tiempo, se cancela
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);

        loop {
            if std::time::Instant::now() >= deadline {
                crate::services::log_structured(
                    "WARNING",
                    "spotify_poll",
                    "Spotify auth timed out after 5 minutes",
                    None,
                );
                break;
            }

            match ureq::get(&poll_url).timeout(std::time::Duration::from_secs(5)).call() {
                Ok(resp) => {
                    if let Ok(json) = resp.into_json::<serde_json::Value>() {
                        if let Some(code) = json["code"].as_str() {
                            let _ = tx.send(code.to_string());
                            break;
                        }
                    }
                }
                Err(ureq::Error::Status(404, _)) => {
                    // aún pendiente — el usuario no ha completado el login en el browser
                }
                Err(e) => {
                    crate::services::log_structured(
                        "ERROR",
                        "spotify_poll",
                        "Error polling Spotify token endpoint",
                        Some(&e.to_string()),
                    );
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
    rx
}
