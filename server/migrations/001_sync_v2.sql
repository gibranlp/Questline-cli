-- Questline encrypted sync database bootstrap.
-- Apply this to gibranlp_QuestlineE; it never reads from or modifies the legacy database.

CREATE TABLE IF NOT EXISTS sync_v2_events (
    seq BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    id VARCHAR(128) NOT NULL,
    account_id VARCHAR(36) NOT NULL,
    version TINYINT UNSIGNED NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,
    operation VARCHAR(20) NOT NULL,
    event_timestamp VARCHAR(50) NOT NULL,
    key_id VARCHAR(64) NOT NULL,
    nonce VARCHAR(32) NOT NULL,
    ciphertext LONGTEXT NOT NULL,
    scope VARCHAR(20) NOT NULL DEFAULT 'account',
    routing_id VARCHAR(36) NULL,
    device_id VARCHAR(64) NOT NULL DEFAULT '',
    author_public_key VARCHAR(64) NULL,
    event_signature VARCHAR(128) NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uq_sync_v2_account_event (account_id, id),
    INDEX idx_sync_v2_pull (account_id, seq),
    INDEX idx_sync_v2_entity (account_id, entity_type, entity_id),
    INDEX idx_sync_v2_device (account_id, device_id),
    INDEX idx_sync_v2_routing (routing_id, seq)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS account_v2_security (
    account_id VARCHAR(36) PRIMARY KEY,
    signatures_required TINYINT(1) NOT NULL DEFAULT 0,
    cutover_at TIMESTAMP NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS project_v2_members (
    routing_id VARCHAR(36) NOT NULL,
    account_id VARCHAR(36) NOT NULL,
    role VARCHAR(50) NOT NULL,
    joined_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (routing_id, account_id),
    INDEX idx_project_v2_account (account_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS project_v2_retired_routes (
    routing_id VARCHAR(36) PRIMARY KEY,
    replacement_routing_id VARCHAR(36) NOT NULL,
    retired_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS project_v2_key_envelopes (
    id VARCHAR(36) PRIMARY KEY,
    old_routing_id VARCHAR(36) NOT NULL,
    new_routing_id VARCHAR(36) NOT NULL,
    recipient_account_id VARCHAR(36) NOT NULL,
    sender_encryption_key VARCHAR(64) NOT NULL,
    key_nonce VARCHAR(32) NOT NULL,
    key_ciphertext TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uq_project_v2_rotation_recipient (new_routing_id, recipient_account_id),
    INDEX idx_project_v2_envelope_recipient (recipient_account_id, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
