-- Apply to the legacy/current Questline database.
ALTER TABLE users ADD COLUMN encryption_public_key VARCHAR(64) NULL;
ALTER TABLE project_invitations ADD COLUMN routing_id VARCHAR(36) NULL;
ALTER TABLE project_invitations ADD COLUMN inviter_encryption_key VARCHAR(64) NULL;
ALTER TABLE project_invitations ADD COLUMN key_nonce VARCHAR(32) NULL;
ALTER TABLE project_invitations ADD COLUMN key_ciphertext TEXT NULL;
ALTER TABLE project_invitations ADD COLUMN project_name_nonce VARCHAR(32) NULL;
ALTER TABLE project_invitations ADD COLUMN project_name_ciphertext TEXT NULL;
ALTER TABLE project_invitations ADD COLUMN project_id_nonce VARCHAR(32) NULL;
ALTER TABLE project_invitations ADD COLUMN project_id_ciphertext TEXT NULL;
