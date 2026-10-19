//! Encrypted profile snapshots live separately from the incremental event log.
use crate::{
    database::Database,
    services::{ApiClient, Identity, encryption},
};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

const AAD: &str = "questline-profile-backup-v1";

#[derive(Serialize, Deserialize)]
pub struct Envelope {
    pub format: String,
    pub nonce: String,
    pub ciphertext: String,
    #[serde(default)]
    pub generation: Option<i64>,
}

pub fn seal(db: &Database, identity: &Identity) -> Result<Envelope> {
    seal_for_generation(db, identity, None)
}

fn seal_for_generation(
    db: &Database,
    identity: &Identity,
    generation: Option<i64>,
) -> Result<Envelope> {
    let json = db.export_to_recovery_json()?;
    let aad = generation
        .map(|value| format!("{AAD}|{value}"))
        .unwrap_or_else(|| AAD.into());
    let (nonce, ciphertext) = encryption::encrypt(identity, &json, &aad)?;
    Ok(Envelope {
        format: AAD.into(),
        nonce,
        ciphertext,
        generation,
    })
}

pub fn open(envelope: &Envelope, identity: &Identity) -> Result<String> {
    if envelope.format != AAD {
        bail!("Unsupported profile backup format");
    }
    let aad = envelope
        .generation
        .map(|value| format!("{AAD}|{value}"))
        .unwrap_or_else(|| AAD.into());
    encryption::decrypt(identity, &envelope.nonce, &envelope.ciphertext, &aad)
}

pub fn upload(db: &Database, identity: &Identity, device_id: &str, server: &str) -> Result<()> {
    let _lock = db.lock_sync()?;
    if let Some(json) = db.get_setting("_profile_identity")? {
        let active: Identity = serde_json::from_str(&json)?;
        if active.public_key != identity.public_key {
            bail!("Profile changed before the backup could start");
        }
    }
    let client = ApiClient::new(server, identity.clone(), device_id);
    let head: serde_json::Value =
        serde_json::from_str(&client.send_request("GET", "sync/v2/head", "")?)?;
    let generation = head["generation"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("Server did not report its backup generation"))?;
    let known = db
        .get_setting("sync_history_generation_v2")?
        .and_then(|value| value.parse::<i64>().ok());
    if known.is_some_and(|known| known != generation) || (known.is_none() && generation > 0) {
        bail!("Cloud history changed; sync or restore this profile before uploading a backup");
    }
    let envelope = seal_for_generation(db, identity, Some(generation))?;
    let response = client.send_request(
        "POST",
        "profile/v2/backup",
        &serde_json::to_string(&envelope)?,
    )?;
    let response: serde_json::Value = serde_json::from_str(&response)?;
    if response["status"] != "success" {
        bail!("Server did not acknowledge the encrypted backup");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    fn identity(secret: u8) -> Identity {
        Identity {
            user_uuid: Uuid::new_v4(),
            public_key: String::new(),
            secret_key: format!("{secret:02x}").repeat(32),
            created_at: String::new(),
        }
    }
    #[test]
    fn snapshot_is_encrypted_and_bound_to_identity() {
        let db = Database::new(std::path::Path::new(":memory:")).unwrap();
        db.set_setting("private-note", "private contents").unwrap();
        let owner = identity(1);
        let envelope = seal(&db, &owner).unwrap();
        assert!(
            !serde_json::to_string(&envelope)
                .unwrap()
                .contains("private contents")
        );
        assert!(
            open(&envelope, &owner)
                .unwrap()
                .contains("private contents")
        );
        assert!(open(&envelope, &identity(2)).is_err());
        assert!(db.get_pending_sync_logs().unwrap().is_empty());
    }
}
