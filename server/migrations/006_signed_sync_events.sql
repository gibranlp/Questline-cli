-- Deploy only after clients emit durable event signatures. Existing unsigned rows
-- require the separate account cutover policy before these columns become NOT NULL.
ALTER TABLE sync_v2_events
    ADD COLUMN author_public_key VARCHAR(64) NULL AFTER device_id,
    ADD COLUMN event_signature VARCHAR(128) NULL AFTER author_public_key;

CREATE TABLE IF NOT EXISTS account_v2_security (
    account_id VARCHAR(36) PRIMARY KEY,
    signatures_required TINYINT(1) NOT NULL DEFAULT 0,
    cutover_at TIMESTAMP NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
