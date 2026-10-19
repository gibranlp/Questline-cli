-- Apply to the encrypted database. Incremented only by an explicit cloud reset.
CREATE TABLE IF NOT EXISTS sync_v2_generations (
    account_id VARCHAR(36) PRIMARY KEY,
    generation BIGINT UNSIGNED NOT NULL DEFAULT 0
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
