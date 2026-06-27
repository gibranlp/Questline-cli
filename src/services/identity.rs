// ─────────────────────────────────────────────────────────────────────────────
// services/identity.rs — maneja las llaves criptográficas Ed25519 para identificar al usuario
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{anyhow, Result};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage;

// La identidad del héroe: su UUID, llave pública y privada en hex, y fecha de creación
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Identity {
    pub user_uuid: Uuid,
    pub public_key: String, // Hex encoded
    pub secret_key: String, // Hex encoded
    pub created_at: String,
}

impl Identity {
    // Carga la identidad existente del disco o genera una nueva — no manches, no toques sin saber cripto
    pub fn load_or_create(existing_user_id: Option<Uuid>) -> Result<Self> {
        let storage_dir = storage::get_storage_dir()?;
        if !storage_dir.exists() {
            std::fs::create_dir_all(&storage_dir)?;
        }
        let key_path = storage_dir.join("identity.key");

        // Si ya existe el archivo, cárgalo y listo — no regeneres llaves innecesariamente
        if key_path.exists() {
            let file_content = std::fs::read_to_string(&key_path)?;
            let identity: Identity = serde_json::from_str(&file_content)?;
            return Ok(identity);
        }

        // Genera un par de llaves Ed25519 criptográficamente seguras — pura magia matemática
        let mut rng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let secret_bytes = signing_key.to_bytes();
        let public_bytes = verifying_key.to_bytes();

        let secret_hex = to_hex(&secret_bytes);
        let public_hex = to_hex(&public_bytes);

        // Si el usuario ya existe en la DB, conserva su ID para no perder su historial
        let user_uuid = existing_user_id.unwrap_or_else(Uuid::new_v4);

        let identity = Identity {
            user_uuid,
            public_key: public_hex,
            secret_key: secret_hex,
            created_at: Utc::now().to_rfc3339(),
        };

        // Guarda la identidad en disco — este archivo es el pasaporte del héroe
        let json_str = serde_json::to_string_pretty(&identity)?;
        std::fs::write(&key_path, json_str)?;

        Ok(identity)
    }

    // Firma un payload con la llave privada — así el servidor sabe que eres tú y no un impostor
    pub fn sign(&self, payload: &[u8]) -> Result<String> {
        let secret_bytes = from_hex(&self.secret_key)?;
        // El array debe ser exactamente 32 bytes para Ed25519, si no truena
        let secret_arr: [u8; 32] = secret_bytes
            .try_into()
            .map_err(|_| anyhow!("Invalid secret key bytes size"))?;
        let signing_key = SigningKey::from_bytes(&secret_arr);

        let signature = signing_key.sign(payload);
        Ok(to_hex(&signature.to_bytes()))
    }

    // Verifies an incoming request signature — rejects tampered payloads before they touch the DB
    pub fn verify(payload: &[u8], public_key_hex: &str, signature_hex: &str) -> Result<bool> {
        let public_bytes = from_hex(public_key_hex)?;
        // La llave pública también necesita ser exactamente 32 bytes
        let public_arr: [u8; 32] = public_bytes
            .try_into()
            .map_err(|_| anyhow!("Invalid public key bytes size"))?;
        let verifying_key = VerifyingKey::from_bytes(&public_arr)?;

        let sig_bytes = from_hex(signature_hex)?;
        // La firma Ed25519 siempre son 64 bytes — ni uno más, ni uno menos
        let sig_arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| anyhow!("Invalid signature bytes size"))?;
        let signature = Signature::from_bytes(&sig_arr);

        // Si la firma no cuadra, regresamos false en lugar de propagar error — el caller decide qué hacer
        match verifying_key.verify(payload, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

// Convierte bytes a string hex — cada byte se vuelve dos caracteres hexadecimales
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// Parsea hex de vuelta a bytes — si viene mal formado, revienta con error
fn from_hex(hex_str: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chars = hex_str.chars();
    while let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
        let s = format!("{}{}", c1, c2);
        let b = u8::from_str_radix(&s, 16)?;
        bytes.push(b);
    }
    Ok(bytes)
}

// Lee el hostname del sistema para identificar el dispositivo en la lista de dispositivos del héroe
pub fn get_local_device_name() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| std::env::var("HOSTNAME").unwrap_or_else(|_| "Workstation".to_string()))
}

// Copia texto al portapapeles usando las herramientas nativas de cada plataforma — órale, soporte multiplataforma
pub fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn pipe_to(cmd: &mut Command, text: &str) -> bool {
        if let Ok(mut child) = cmd.stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return true;
        }
        false
    }

    // macOS — pbcopy
    #[cfg(target_os = "macos")]
    if pipe_to(&mut Command::new("pbcopy"), text) {
        return Ok(());
    }

    // Windows — clip.exe (ships with every Windows installation)
    #[cfg(target_os = "windows")]
    if pipe_to(&mut Command::new("clip"), text) {
        return Ok(());
    }

    // Linux / BSD — intenta primero Wayland, luego X11, si no hay nada pues truena con error
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if pipe_to(&mut Command::new("wl-copy"), text) {
            return Ok(());
        }
        if pipe_to(Command::new("xclip").args(["-selection", "clipboard"]), text) {
            return Ok(());
        }
        if pipe_to(Command::new("xsel").args(["-b", "-i"]), text) {
            return Ok(());
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No clipboard utility found. Install wl-copy (Wayland) or xclip/xsel (X11).",
        ));
    }

    // Fallback for any other platform
    #[allow(unreachable_code)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Clipboard copy is not supported on this platform.",
    ))
}
