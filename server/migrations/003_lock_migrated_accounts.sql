-- Apply to the legacy/current Questline database, not gibranlp_QuestlineE.
-- Existing accounts remain on protocol 1 until they successfully replace a
-- complete encrypted sync-v2 snapshot. Incremental writes do not lock migration.

ALTER TABLE users
    ADD COLUMN sync_protocol TINYINT UNSIGNED NOT NULL DEFAULT 1;
