-- Apply to gibranlp_QuestlineE.
ALTER TABLE sync_v2_events ADD COLUMN scope VARCHAR(20) NOT NULL DEFAULT 'account';
ALTER TABLE sync_v2_events ADD COLUMN routing_id VARCHAR(36) NULL;
ALTER TABLE sync_v2_events ADD INDEX idx_sync_v2_routing (routing_id, seq);

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
