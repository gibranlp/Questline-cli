use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::services::Identity;

pub const SYNC_VERSION: u8 = 2;
pub const KEY_ID: &str = "account-v1";

/// Canonical, language-independent bytes signed for every durable sync event.
/// Length prefixes prevent delimiter ambiguity and allow PHP/JS to reproduce it.
pub fn event_signature_message(fields: &[&str]) -> Vec<u8> {
    let mut message = b"questline-sync-event-v1\n".to_vec();
    for field in fields {
        message.extend_from_slice(field.as_bytes().len().to_string().as_bytes());
        message.push(b':');
        message.extend_from_slice(field.as_bytes());
    }
    message
}
pub const CIPHER_NAME: &str = "AES-256-GCM";
const KEY_INFO: &[u8] = b"questline/sync/v2";
const FELLOWSHIP_IDENTITY_INFO: &[u8] = b"questline/fellowship/x25519/v1";

pub fn derive_sync_key(identity: &Identity) -> Result<[u8; 32]> {
    let secret = decode_hex(&identity.secret_key)?;
    if secret.len() != 32 {
        return Err(anyhow!("invalid Ed25519 private identity length"));
    }
    let hkdf = Hkdf::<Sha256>::new(None, &secret);
    let mut key = [0u8; 32];
    hkdf.expand(KEY_INFO, &mut key)
        .map_err(|_| anyhow!("failed to derive sync encryption key"))?;
    Ok(key)
}

pub fn associated_data(
    id: &str,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    timestamp: &str,
) -> String {
    associated_data_for(id, entity_type, entity_id, operation, timestamp, KEY_ID)
}

pub fn associated_data_for(
    id: &str,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    timestamp: &str,
    key_id: &str,
) -> String {
    format!("questline-sync-v2|{id}|{entity_type}|{entity_id}|{operation}|{timestamp}|{key_id}")
}

pub fn associated_data_for_route(
    id: &str,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    timestamp: &str,
    key_id: &str,
    scope: &str,
    routing_id: &str,
) -> String {
    format!(
        "{}|{}|{}",
        associated_data_for(id, entity_type, entity_id, operation, timestamp, key_id),
        scope,
        routing_id
    )
}

pub fn encrypt(identity: &Identity, plaintext: &str, aad: &str) -> Result<(String, String)> {
    let key = derive_sync_key(identity)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("invalid encryption key"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| anyhow!("sync encryption failed"))?;
    Ok((STANDARD.encode(nonce_bytes), STANDARD.encode(ciphertext)))
}

pub fn decrypt(identity: &Identity, nonce: &str, ciphertext: &str, aad: &str) -> Result<String> {
    let key = derive_sync_key(identity)?;
    let nonce = STANDARD
        .decode(nonce)
        .map_err(|_| anyhow!("invalid sync nonce"))?;
    if nonce.len() != 12 {
        return Err(anyhow!("invalid sync nonce length"));
    }
    let ciphertext = STANDARD
        .decode(ciphertext)
        .map_err(|_| anyhow!("invalid sync ciphertext"))?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow!("invalid encryption key"))?;
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| anyhow!("sync decryption failed: wrong identity or tampered event"))?;
    String::from_utf8(plaintext).map_err(|_| anyhow!("decrypted sync payload is not UTF-8"))
}

pub fn encrypt_with_project_key(
    key: &[u8; 32],
    plaintext: &str,
    aad: &str,
) -> Result<(String, String)> {
    encrypt_with_key(key, plaintext.as_bytes(), aad.as_bytes())
}

pub fn decrypt_with_project_key(
    key: &[u8; 32],
    nonce: &str,
    ciphertext: &str,
    aad: &str,
) -> Result<String> {
    String::from_utf8(decrypt_with_key(key, nonce, ciphertext, aad.as_bytes())?)
        .map_err(|_| anyhow!("decrypted project event is not UTF-8"))
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    if value.len() % 2 != 0 {
        return Err(anyhow!("invalid hex identity"));
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(Into::into))
        .collect()
}

pub fn fellowship_secret(identity: &Identity) -> Result<StaticSecret> {
    let secret = decode_hex(&identity.secret_key)?;
    let hkdf = Hkdf::<Sha256>::new(None, &secret);
    let mut bytes = [0u8; 32];
    hkdf.expand(FELLOWSHIP_IDENTITY_INFO, &mut bytes)
        .map_err(|_| anyhow!("failed to derive Fellowship encryption identity"))?;
    Ok(StaticSecret::from(bytes))
}

pub fn fellowship_public_key(identity: &Identity) -> Result<String> {
    let public = X25519PublicKey::from(&fellowship_secret(identity)?);
    Ok(public
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

pub fn wrap_project_key(
    identity: &Identity,
    recipient_public_hex: &str,
    project_id: &str,
    project_key: &[u8; 32],
) -> Result<(String, String)> {
    let recipient: [u8; 32] = decode_hex(recipient_public_hex)?
        .try_into()
        .map_err(|_| anyhow!("invalid Fellowship public key"))?;
    let shared = fellowship_secret(identity)?.diffie_hellman(&X25519PublicKey::from(recipient));
    let hkdf = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut wrapping_key = [0u8; 32];
    hkdf.expand(
        format!("questline/fellowship/wrap/v1/{project_id}").as_bytes(),
        &mut wrapping_key,
    )
    .map_err(|_| anyhow!("failed to derive project key wrapping key"))?;
    encrypt_with_key(&wrapping_key, project_key, project_id.as_bytes())
}

pub fn unwrap_project_key(
    identity: &Identity,
    sender_public_hex: &str,
    project_id: &str,
    nonce: &str,
    ciphertext: &str,
) -> Result<[u8; 32]> {
    let sender: [u8; 32] = decode_hex(sender_public_hex)?
        .try_into()
        .map_err(|_| anyhow!("invalid Fellowship public key"))?;
    let shared = fellowship_secret(identity)?.diffie_hellman(&X25519PublicKey::from(sender));
    let hkdf = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut wrapping_key = [0u8; 32];
    hkdf.expand(
        format!("questline/fellowship/wrap/v1/{project_id}").as_bytes(),
        &mut wrapping_key,
    )
    .map_err(|_| anyhow!("failed to derive project key wrapping key"))?;
    decrypt_with_key(&wrapping_key, nonce, ciphertext, project_id.as_bytes())?
        .try_into()
        .map_err(|_| anyhow!("invalid unwrapped project key"))
}

fn encrypt_with_key(key: &[u8; 32], plaintext: &[u8], aad: &[u8]) -> Result<(String, String)> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow!("invalid encryption key"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| anyhow!("project key encryption failed"))?;
    Ok((STANDARD.encode(nonce_bytes), STANDARD.encode(ciphertext)))
}

fn decrypt_with_key(key: &[u8; 32], nonce: &str, ciphertext: &str, aad: &[u8]) -> Result<Vec<u8>> {
    let nonce = STANDARD
        .decode(nonce)
        .map_err(|_| anyhow!("invalid key-envelope nonce"))?;
    if nonce.len() != 12 {
        return Err(anyhow!("invalid key-envelope nonce length"));
    }
    let ciphertext = STANDARD
        .decode(ciphertext)
        .map_err(|_| anyhow!("invalid key envelope"))?;
    Aes256Gcm::new_from_slice(key)
        .map_err(|_| anyhow!("invalid encryption key"))?
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad,
            },
        )
        .map_err(|_| anyhow!("project key envelope authentication failed"))
}

pub fn encrypt_project_payload(
    key: &[u8; 32],
    plaintext: &str,
    aad: &str,
) -> Result<(String, String)> {
    encrypt_with_key(key, plaintext.as_bytes(), aad.as_bytes())
}

pub fn decrypt_project_payload(
    key: &[u8; 32],
    nonce: &str,
    ciphertext: &str,
    aad: &str,
) -> Result<String> {
    String::from_utf8(decrypt_with_key(key, nonce, ciphertext, aad.as_bytes())?)
        .map_err(|_| anyhow!("decrypted project payload is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use uuid::Uuid;

    fn identity() -> Identity {
        Identity {
            user_uuid: Uuid::nil(),
            public_key: "00".repeat(32),
            secret_key: "01".repeat(32),
            created_at: String::new(),
        }
    }

    #[test]
    fn round_trip_and_aad_tamper_detection() {
        let id = identity();
        let (nonce, ciphertext) = encrypt(&id, r#"{"title":"secret"}"#, "aad").unwrap();
        assert_eq!(
            decrypt(&id, &nonce, &ciphertext, "aad").unwrap(),
            r#"{"title":"secret"}"#
        );
        assert!(decrypt(&id, &nonce, &ciphertext, "changed").is_err());
    }

    #[test]
    fn stable_hkdf_test_vector() {
        assert_eq!(
            hex(&derive_sync_key(&identity()).unwrap()),
            "31218b3b3b68d4e9be0f8532c1601d1fd5b54bef07319cdb387904b8daa6cccf"
        );
    }

    #[test]
    fn decrypts_browser_produced_fellowship_envelope() {
        // Envelope generated by webapp/src/lib/crypto.js (wrapProjectKey +
        // encryptProjectPayload). Sender = identity(secret 01..), recipient =
        // secret 02.., routing_id fixed, project key = 03 x 32. Proves a browser
        // owner's invitation can be opened by the Rust CLI.
        let recipient = Identity {
            user_uuid: Uuid::nil(),
            public_key: "00".repeat(32),
            secret_key: "02".repeat(32),
            created_at: String::new(),
        };
        let sender_public = "4868ab2c01c594c811d6f5a7e224ad18b0b31b3bb76b0506ff4de4658ed2c429";
        let routing = "550e8400-e29b-41d4-a716-446655440000";

        let key = unwrap_project_key(
            &recipient,
            sender_public,
            routing,
            "I9rGHTQ9Um+eU61z",
            "LBg0MvCA5tmGChx19gHl5ypfmmd0Mdpcv/F11GCOrjEOWHeOoVerkuFfdQ7jgzD5",
        )
        .unwrap();
        assert_eq!(key, [3u8; 32]);

        let name = decrypt_project_payload(
            &key,
            "7dJn+p9H/lUgX1Oe",
            "pF1f7JKeOxtdS/CeDZBv+u3Hw3PzOdHv8Q==",
            &format!("questline/fellowship/name/v1/{routing}"),
        )
        .unwrap();
        assert_eq!(name, "Rivendell");
    }

    #[test]
    fn stable_fellowship_public_key_vector() {
        // Cross-implementation interop vector shared with the browser
        // (webapp/src/lib/crypto.js fellowshipPublicKeyHex). Do not change without
        // updating the JS side; a mismatch breaks CLI<->web encrypted Fellowship.
        assert_eq!(
            fellowship_public_key(&identity()).unwrap(),
            "4868ab2c01c594c811d6f5a7e224ad18b0b31b3bb76b0506ff4de4658ed2c429"
        );
    }

    #[test]
    fn durable_event_signature_detects_envelope_tampering() {
        let secret = [9u8; 32];
        let signing_key = SigningKey::from_bytes(&secret);
        let id = Identity {
            user_uuid: Uuid::nil(),
            public_key: hex(&signing_key.verifying_key().to_bytes()),
            secret_key: hex(&secret),
            created_at: String::new(),
        };
        let fields = [
            "2",
            "event",
            "task",
            "task-id",
            "upsert",
            "timestamp",
            "account-v1",
            "nonce",
            "ciphertext",
            "account",
            "",
            "device",
            &id.public_key,
        ];
        let message = event_signature_message(&fields);
        let signature = id.sign(&message).unwrap();
        assert!(Identity::verify(&message, &id.public_key, &signature).unwrap());

        let mut tampered = fields;
        tampered[8] = "different-ciphertext";
        assert!(
            !Identity::verify(
                &event_signature_message(&tampered),
                &id.public_key,
                &signature
            )
            .unwrap()
        );
    }

    #[test]
    fn fellowship_project_key_envelope_round_trip() {
        let sender = identity();
        let mut recipient = identity();
        recipient.secret_key = "02".repeat(32);
        let recipient_public = fellowship_public_key(&recipient).unwrap();
        let sender_public = fellowship_public_key(&sender).unwrap();
        let project_key = [7u8; 32];
        let (nonce, ciphertext) =
            wrap_project_key(&sender, &recipient_public, "project-id", &project_key).unwrap();
        assert_eq!(
            unwrap_project_key(
                &recipient,
                &sender_public,
                "project-id",
                &nonce,
                &ciphertext
            )
            .unwrap(),
            project_key
        );
    }

    #[test]
    fn wrong_project_key_cannot_decrypt_payload() {
        let aad = "questline-sync-v2|event|note|id|upsert|timestamp|project-v1|project|route";
        let (nonce, ciphertext) = encrypt_with_project_key(&[7u8; 32], "secret", aad).unwrap();
        assert!(decrypt_with_project_key(&[8u8; 32], &nonce, &ciphertext, aad).is_err());
        assert_eq!(
            decrypt_with_project_key(&[7u8; 32], &nonce, &ciphertext, aad).unwrap(),
            "secret"
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
