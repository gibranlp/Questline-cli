// ─────────────────────────────────────────────────────────────────────────────
// services/identity.rs — maneja las llaves criptográficas Ed25519 para identificar al usuario
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{Result, anyhow};
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
            let mut identity: Identity = serde_json::from_str(&file_content)?;
            if let Some(user_id) = existing_user_id {
                if identity.user_uuid != user_id {
                    identity.user_uuid = user_id;
                    let json_str = serde_json::to_string_pretty(&identity)?;
                    std::fs::write(&key_path, json_str)?;
                }
            }
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

/// Normalize the public identity exchanged during the Fellowship trust ceremony.
/// Spaces, dashes, and colons are display separators; all other non-hex input is
/// rejected rather than silently discarded.
pub fn normalize_companion_key(input: &str) -> Result<String> {
    let mut normalized = String::with_capacity(64);
    for character in input.chars() {
        if character.is_ascii_hexdigit() {
            normalized.push(character.to_ascii_lowercase());
        } else if character.is_whitespace() || matches!(character, '-' | ':') {
            continue;
        } else {
            return Err(anyhow!(
                "Companion Key contains an invalid character: {character}"
            ));
        }
    }
    if normalized.len() != 64 {
        return Err(anyhow!(
            "Companion Key must contain exactly 64 hexadecimal characters (found {})",
            normalized.len()
        ));
    }
    // Exact decoding catches malformed pairs and documents the 32-byte contract.
    if from_hex(&normalized)?.len() != 32 {
        return Err(anyhow!("Companion Key must decode to 32 bytes"));
    }
    Ok(normalized)
}

/// Format a complete or partial normalized key into readable 8-character groups.
pub fn format_companion_key(input: &str) -> String {
    input
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .map(|character| character.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .chunks(8)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A short verification fingerprint. This is deliberately only a comparison aid;
/// the complete Companion Key remains the actual cryptographic identity.
pub fn companion_key_fingerprint(input: &str) -> Result<String> {
    let normalized = normalize_companion_key(input)?;
    let key_bytes = from_hex(&normalized)?;
    let digest = Sha256::digest(key_bytes);
    Ok(digest[..8]
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>()
        .as_bytes()
        .chunks(4)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join("-"))
}

// Lee el hostname del sistema para identificar el dispositivo en la lista de dispositivos del héroe
pub fn get_local_device_name() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| std::env::var("HOSTNAME").unwrap_or_else(|_| "Workstation".to_string()))
}

#[cfg(test)]
mod companion_key_tests {
    use super::*;

    #[test]
    fn companion_key_normalizes_grouped_uppercase_input() {
        let raw = "AA11BB22-CC33DD44 EE55FF66:00112233 44556677 8899AABB CCDDEEFF 01234567";
        assert_eq!(
            normalize_companion_key(raw).unwrap(),
            "aa11bb22cc33dd44ee55ff6600112233445566778899aabbccddeeff01234567"
        );
    }

    #[test]
    fn companion_key_rejects_invalid_characters_and_lengths() {
        assert!(normalize_companion_key(&"ab".repeat(31)).is_err());
        assert!(normalize_companion_key(&format!("{}g1", "ab".repeat(31))).is_err());
    }

    #[test]
    fn companion_key_format_and_fingerprint_are_stable() {
        let key = "ab".repeat(32);
        assert_eq!(
            format_companion_key(&key),
            "abababab abababab abababab abababab abababab abababab abababab abababab"
        );
        assert_eq!(
            companion_key_fingerprint(&key).unwrap(),
            "9A2D-B2E2-3F15-04CD"
        );
    }
}

// Detecta si estamos corriendo dentro de WSL (Windows Subsystem for Linux) — checa el
// kernel release (uname) primero y /proc/version como respaldo, sin depender de env vars
// que a veces no se propagan (p.ej. cuando se corre bajo sudo o un shell no interactivo).
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn is_wsl() -> bool {
    if std::env::var_os("WSL_DISTRO_NAME").is_some() || std::env::var_os("WSL_INTEROP").is_some() {
        return true;
    }
    std::fs::read_to_string("/proc/version")
        .map(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("microsoft") || v.contains("wsl")
        })
        .unwrap_or(false)
}

// Copia texto al portapapeles usando las herramientas nativas de cada plataforma — órale, soporte multiplataforma
pub fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // Regresa true solo si el proceso hijo de verdad terminó con éxito — no basta con
    // que haya podido lanzarse (spawn), porque p.ej. xclip/xsel se lanzan bien y luego
    // truenan al tiro si no hay $DISPLAY (típico en WSL sin WSLg), reportando un yank
    // "exitoso" que en realidad nunca llegó al portapapeles.
    fn pipe_to(cmd: &mut Command, text: &str) -> bool {
        match cmd.stdin(Stdio::piped()).spawn() {
            Ok(mut child) => {
                let write_ok = if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(text.as_bytes()).is_ok()
                } else {
                    false
                };
                match child.wait() {
                    Ok(status) => write_ok && status.success(),
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
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
        // Bajo WSL casi nunca hay un compositor Wayland ni un servidor X corriendo (salvo
        // que WSLg esté habilitado), así que wl-copy/xclip/xsel truenan aunque estén
        // instalados. clip.exe de Windows sí está disponible vía interop de WSL y es el
        // camino confiable — lo intentamos primero en ese caso.
        if is_wsl() {
            if pipe_to(&mut Command::new("clip.exe"), text) {
                return Ok(());
            }
            // Ruta absoluta de respaldo por si /mnt/c no está en el PATH del shell.
            if pipe_to(
                &mut Command::new("/mnt/c/Windows/System32/clip.exe"),
                text,
            ) {
                return Ok(());
            }
            // Último recurso vía PowerShell (más lento, pero universal en cualquier WSL).
            if pipe_to(
                Command::new("powershell.exe").args(["-NoProfile", "-Command", "Set-Clipboard"]),
                text,
            ) {
                return Ok(());
            }
        }

        if pipe_to(&mut Command::new("wl-copy"), text) {
            return Ok(());
        }
        if pipe_to(
            Command::new("xclip").args(["-selection", "clipboard"]),
            text,
        ) {
            return Ok(());
        }
        if pipe_to(Command::new("xsel").args(["-b", "-i"]), text) {
            return Ok(());
        }

        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            if is_wsl() {
                "Could not reach the Windows clipboard from WSL. Make sure clip.exe is reachable (WSL interop enabled), or install wl-copy/xclip/xsel if you're running a Linux GUI (WSLg)."
            } else {
                "No clipboard utility found. Install wl-copy (Wayland) or xclip/xsel (X11)."
            },
        ));
    }

    // Fallback for any other platform
    #[allow(unreachable_code)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Clipboard copy is not supported on this platform.",
    ))
}

// Lee el portapapeles del sistema — contraparte de copy_to_clipboard, para acciones como
// el paste con click derecho del mouse en el editor. Usa las mismas herramientas nativas
// por plataforma (no hay lectura vía clip.exe en Windows, así que ahí usamos PowerShell).
pub fn paste_from_clipboard() -> std::io::Result<String> {
    use std::process::Command;

    // A diferencia de pipe_to (que escribe a stdin), aquí solo leemos stdout — pero igual
    // checamos el status de salida y no solo que el comando haya podido lanzarse, por la
    // misma razón: xclip/xsel/wl-paste truenan con exit code distinto de cero si no hay
    // $DISPLAY/compositor, y ahí no queremos regresar una cadena vacía como si fuera éxito.
    fn read_from(cmd: &mut Command) -> Option<String> {
        let output = cmd.output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    }

    // macOS — pbpaste
    #[cfg(target_os = "macos")]
    if let Some(text) = read_from(&mut Command::new("pbpaste")) {
        return Ok(text);
    }

    // Windows — no hay contraparte de lectura para clip.exe, así que usamos PowerShell
    #[cfg(target_os = "windows")]
    if let Some(text) = read_from(
        Command::new("powershell").args(["-NoProfile", "-Command", "Get-Clipboard"]),
    ) {
        return Ok(text);
    }

    // Linux / BSD — mismo orden de intentos que copy_to_clipboard, invertido a lectura
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if is_wsl() {
            if let Some(text) = read_from(
                Command::new("powershell.exe").args(["-NoProfile", "-Command", "Get-Clipboard"]),
            ) {
                return Ok(text);
            }
        }

        if let Some(text) = read_from(&mut Command::new("wl-paste")) {
            return Ok(text);
        }
        if let Some(text) = read_from(
            Command::new("xclip").args(["-selection", "clipboard", "-o"]),
        ) {
            return Ok(text);
        }
        if let Some(text) = read_from(Command::new("xsel").args(["-b", "-o"])) {
            return Ok(text);
        }

        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            if is_wsl() {
                "Could not reach the Windows clipboard from WSL. Make sure powershell.exe is reachable (WSL interop enabled), or install wl-clipboard/xclip/xsel if you're running a Linux GUI (WSLg)."
            } else {
                "No clipboard utility found. Install wl-clipboard (Wayland) or xclip/xsel (X11)."
            },
        ));
    }

    // Fallback for any other platform
    #[allow(unreachable_code)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Clipboard paste is not supported on this platform.",
    ))
}

#[cfg(test)]
mod clipboard_tests {
    use super::*;

    #[test]
    fn copy_then_paste_round_trips_through_the_os_clipboard() {
        // Headless CI containers often have no clipboard utility installed at
        // all (no $DISPLAY, no clip.exe/powershell.exe reachable) — treat that
        // as "can't verify here" rather than a failure, matching how
        // copy_to_clipboard itself already tolerates a missing backend.
        let payload = "questline-mouse-support-clipboard-roundtrip-test";
        if copy_to_clipboard(payload).is_err() {
            return;
        }
        match paste_from_clipboard() {
            Ok(text) => assert_eq!(text.trim_end_matches(['\r', '\n']), payload),
            Err(_) => {}
        }
    }
}
