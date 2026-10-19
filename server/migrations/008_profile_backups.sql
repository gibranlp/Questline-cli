-- Run against the encrypted (E2EE) database before deploying the new clients.
-- Profile backups must never delete or replace sync_v2_events.
CREATE TABLE IF NOT EXISTS profile_v2_backups (
    account_id VARCHAR(36) PRIMARY KEY,
    envelope LONGTEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
