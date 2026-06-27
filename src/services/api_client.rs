// ─────────────────────────────────────────────────────────────────────────────
// services/api_client.rs — el cliente HTTP para hablar con el servidor, con reintentos y firma
// ─────────────────────────────────────────────────────────────────────────────

use crate::services::Identity;
use anyhow::Result;
use serde_json::Value;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 500;

pub struct ApiClient {
    pub server_url: String,
    pub identity: Identity,
    pub device_id: String,
}

impl ApiClient {
    pub fn new(server_url: &str, identity: Identity, device_id: &str) -> Self {
        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            identity,
            device_id: device_id.to_string(),
        }
    }

    // Órale, aquí va la magia: manda la petición firmada y si falla reintenta hasta 3 veces.
    // Los errores 4xx (auth/cliente) no se reintentan — ya cagaste, no hay vuelta atrás.
    pub fn send_request(&self, method: &str, path: &str, body: &str) -> Result<String> {
        let mut last_err = anyhow::anyhow!("No attempts made");
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(
                    RETRY_DELAY_MS * (1 << (attempt - 1)),
                ));
            }
            match self.send_request_once(method, path, body) {
                Ok(response) => return Ok(response),
                Err(e) => {
                    let msg = e.to_string();
                    // No reintentes errores de auth (401/403) ni errores del cliente (4xx)
                    if msg.contains("401") || msg.contains("403") || msg.contains("400") || msg.contains("422") {
                        return Err(e);
                    }
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }

    // Pues aquí se arma todo: timestamp, nonce, firma y la URL con el route. Qué rollo de headers.
    fn send_request_once(&self, method: &str, path: &str, body: &str) -> Result<String> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let nonce = uuid::Uuid::new_v4().to_string();

        // Encodeamos el body en base64 para que el WAF/ModSecurity no nos bloquee (error 406)
        let body_to_send = if method == "POST" && !body.is_empty() {
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            STANDARD.encode(body.as_bytes())
        } else {
            body.to_string()
        };

        let message = format!("{}.{}.{}", timestamp, nonce, body_to_send);
        let signature = self.identity.sign(message.as_bytes())?;

        let url = {
            let (route_path, extra_query) = if let Some(pos) = path.find('?') {
                (&path[..pos], Some(&path[pos + 1..]))
            } else {
                (path, None)
            };

            let mut constructed =
                if self.server_url.ends_with("index.php") || self.server_url.ends_with('/') {
                    format!("{}?route={}", self.server_url, route_path)
                } else {
                    format!("{}/?route={}", self.server_url, route_path)
                };

            if let Some(query) = extra_query {
                constructed.push('&');
                constructed.push_str(query);
            }
            constructed
        };

        let req = match method {
            "GET" => ureq::get(&url),
            "POST" => {
                let mut r = ureq::post(&url);
                r = r.set("Content-Type", "application/json");
                r
            }
            _ => return Err(anyhow::anyhow!("Unsupported HTTP method")),
        };

        let response = req
            .set("X-User-Id", &self.identity.user_uuid.to_string())
            .set("X-Identity", &self.identity.public_key)
            .set("X-Device-Id", &self.device_id)
            .set("X-Timestamp", &timestamp)
            .set("X-Nonce", &nonce)
            .set("X-Signature", &signature)
            .send_string(&body_to_send)?;

        Ok(response.into_string()?)
    }

    pub fn lookup_username(&self, public_key_hex: &str) -> Option<String> {
        let path = format!("user/lookup?key={}", public_key_hex);
        let resp = self.send_request("GET", &path, "").ok()?;
        let v: Value = serde_json::from_str(&resp).ok()?;
        v["username"].as_str().map(|s| s.to_string())
    }

    // Jala el progreso del capítulo activo desde el server
    pub fn fetch_chapter_progress(&self, chapter_id: &str) -> Result<Value> {
        let path = format!("chapter/active?chapter_id={}", chapter_id);
        let resp = self.send_request("GET", &path, "")?;
        Ok(serde_json::from_str(&resp)?)
    }

    // Manda las contribuciones incrementales al servidor — esto es lo que mueve el capítulo global
    pub fn submit_chapter_contribution(
        &self,
        chapter_id: &str,
        contributions: &std::collections::HashMap<String, u64>,
    ) -> Result<Value> {
        let body = serde_json::json!({
            "chapter_id": chapter_id,
            "contributions": contributions,
        }).to_string();
        let resp = self.send_request("POST", "chapter/contribute", &body)?;
        Ok(serde_json::from_str(&resp)?)
    }

    // Trae el historial de capítulos completados con tus aportaciones personales
    pub fn fetch_chapter_history(&self) -> Result<Value> {
        let resp = self.send_request("GET", "chapter/history", "")?;
        Ok(serde_json::from_str(&resp)?)
    }

    pub fn fetch_my_chapter_contributions(&self, chapter_id: &str) -> Result<std::collections::HashMap<String, u64>> {
        let path = format!("chapter/my-contributions?chapter_id={}", chapter_id);
        let resp = self.send_request("GET", &path, "")?;
        let val: Value = serde_json::from_str(&resp)?;
        let mut map = std::collections::HashMap::new();
        if let Some(totals) = val["totals"].as_object() {
            for (k, v) in totals {
                if let Some(n) = v.as_u64() {
                    map.insert(k.clone(), n);
                }
            }
        }
        Ok(map)
    }
}
