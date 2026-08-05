-- Compound Fellowship IDs contain a UUID, "__", and a 64-character identity
-- (102 characters total). VARCHAR(64) truncated them after signature validation,
-- poisoning the recipient stream with an envelope that could never verify.
ALTER TABLE sync_v2_events
    MODIFY COLUMN entity_id VARCHAR(255) NOT NULL;

-- These rows are cryptographically unrecoverable: both the durable signature and
-- AES-GCM associated data bind the original full ID. Remove only known compound
-- types that hit the old column boundary. The authoritative profile must perform
-- one full sync after this migration to republish them with fresh event IDs.
DELETE FROM sync_v2_events
WHERE entity_type IN ('project_member', 'task_assignment', 'task_dependency')
  AND CHAR_LENGTH(entity_id) = 64
  AND LOCATE('__', entity_id) > 0;
