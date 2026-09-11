use anyhow::{Result, anyhow};
use keyring::v1::{Entry, Error as KeyringError};
use sha2::{Digest, Sha256};

const CALENDAR_VAULT_SERVICE: &str = "com.questline.calendar";
const MAX_REFERENCE_COMPONENT_BYTES: usize = 512;
const MAX_SECRET_BYTES: usize = 16 * 1024;

/// Calendar secrets are stored separately so access-token rotation cannot
/// overwrite the durable refresh token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarCredentialKind {
    AccessToken,
    RefreshToken,
}

impl CalendarCredentialKind {
    fn wire_name(self) -> &'static str {
        match self {
            Self::AccessToken => "access-token",
            Self::RefreshToken => "refresh-token",
        }
    }
}

/// An opaque, non-secret handle suitable for local subscription metadata.
/// Provider account identifiers are deliberately hashed before they reach the
/// operating-system credential store or SQLite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarCredentialRef(String);

impl CalendarCredentialRef {
    pub fn new(
        profile: &str,
        provider: &str,
        provider_account_id: &str,
        kind: CalendarCredentialKind,
    ) -> Result<Self> {
        for (name, value) in [
            ("profile", profile),
            ("provider", provider),
            ("provider account", provider_account_id),
        ] {
            if value.is_empty()
                || value.len() > MAX_REFERENCE_COMPONENT_BYTES
                || value.chars().any(char::is_control)
            {
                return Err(anyhow!("Invalid calendar credential {name}"));
            }
        }

        let mut hasher = Sha256::new();
        for component in [
            "questline-calendar-credential-v1",
            profile,
            provider,
            provider_account_id,
            kind.wire_name(),
        ] {
            hasher.update((component.len() as u64).to_be_bytes());
            hasher.update(component.as_bytes());
        }
        Ok(Self(format!("calendar-v1-{:x}", hasher.finalize())))
    }

    pub fn opaque_id(&self) -> &str {
        &self.0
    }
}

/// Retrieved secret bytes are wiped before their allocation is released.
/// This limits their lifetime but does not claim protection from a compromised
/// process, debugger, or operating system.
pub struct SecretValue(Vec<u8>);

impl SecretValue {
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NativeCredentialVault;

impl NativeCredentialVault {
    /// Fails closed when the native secure store cannot be initialized. There
    /// is intentionally no environment-variable, SQLite, config, or file
    /// fallback.
    pub fn require_available(&self) -> Result<()> {
        Entry::store_status()
            .as_ref()
            .copied()
            .map_err(|error| redacted_vault_error("initialize", error))
    }

    pub fn store(&self, reference: &CalendarCredentialRef, secret: &[u8]) -> Result<()> {
        if secret.is_empty() || secret.len() > MAX_SECRET_BYTES {
            return Err(anyhow!(
                "Calendar credential must contain between 1 and {MAX_SECRET_BYTES} bytes"
            ));
        }
        let entry = self.entry(reference)?;
        entry
            .set_secret(secret)
            .map_err(|error| redacted_vault_error("store", &error))
    }

    pub fn read(&self, reference: &CalendarCredentialRef) -> Result<Option<SecretValue>> {
        let entry = self.entry(reference)?;
        match entry.get_secret() {
            Ok(secret) => Ok(Some(SecretValue(secret))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(redacted_vault_error("read", &error)),
        }
    }

    /// Deletion is idempotent so local disconnect can complete after an
    /// interrupted or partially successful provider-revocation attempt.
    pub fn delete(&self, reference: &CalendarCredentialRef) -> Result<()> {
        let entry = self.entry(reference)?;
        match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(redacted_vault_error("delete", &error)),
        }
    }

    fn entry(&self, reference: &CalendarCredentialRef) -> Result<Entry> {
        Entry::new(CALENDAR_VAULT_SERVICE, reference.opaque_id())
            .map_err(|error| redacted_vault_error("open", &error))
    }
}

/// Never forwards platform error text: a provider or vault may echo an account
/// identifier, request field, or secret-bearing value in its diagnostic.
fn redacted_vault_error(action: &str, error: &KeyringError) -> anyhow::Error {
    let reason = match error {
        KeyringError::NoDefaultStore | KeyringError::NotSupportedByStore(_) => {
            "no supported native credential vault is available"
        }
        KeyringError::NoStorageAccess(_) => "the native credential vault is locked or unavailable",
        KeyringError::NoEntry => "the calendar credential is not connected",
        _ => "the native credential vault rejected the operation",
    };
    anyhow!("Could not {action} calendar credential: {reason}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_references_are_stable_scoped_and_opaque() {
        let reference = CalendarCredentialRef::new(
            "work",
            "provider-a",
            "private-account@example.com",
            CalendarCredentialKind::RefreshToken,
        )
        .unwrap();
        let same = CalendarCredentialRef::new(
            "work",
            "provider-a",
            "private-account@example.com",
            CalendarCredentialKind::RefreshToken,
        )
        .unwrap();
        let other_profile = CalendarCredentialRef::new(
            "personal",
            "provider-a",
            "private-account@example.com",
            CalendarCredentialKind::RefreshToken,
        )
        .unwrap();
        let access_token = CalendarCredentialRef::new(
            "work",
            "provider-a",
            "private-account@example.com",
            CalendarCredentialKind::AccessToken,
        )
        .unwrap();

        assert_eq!(reference, same);
        assert_ne!(reference, other_profile);
        assert_ne!(reference, access_token);
        assert!(!reference.opaque_id().contains("private-account"));
        assert!(!reference.opaque_id().contains("provider-a"));
        assert_eq!(reference.opaque_id().len(), "calendar-v1-".len() + 64);
    }

    #[test]
    fn credential_reference_rejects_empty_oversized_and_control_text() {
        assert!(
            CalendarCredentialRef::new(
                "",
                "provider",
                "account",
                CalendarCredentialKind::RefreshToken
            )
            .is_err()
        );
        assert!(
            CalendarCredentialRef::new(
                "profile",
                "provider\nforged-log-entry",
                "account",
                CalendarCredentialKind::RefreshToken
            )
            .is_err()
        );
        assert!(
            CalendarCredentialRef::new(
                "profile",
                "provider",
                &"a".repeat(MAX_REFERENCE_COMPONENT_BYTES + 1),
                CalendarCredentialKind::RefreshToken
            )
            .is_err()
        );
    }

    #[test]
    fn vault_errors_never_include_platform_details() {
        let platform_error = KeyringError::Invalid(
            "refresh_token".to_string(),
            "secret-value-that-must-not-escape".to_string(),
        );
        let message = redacted_vault_error("store", &platform_error).to_string();
        assert!(!message.contains("refresh_token"));
        assert!(!message.contains("secret-value"));
        assert_eq!(
            message,
            "Could not store calendar credential: the native credential vault rejected the operation"
        );
    }
}
