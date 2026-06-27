-- ─────────────────────────────────────────────────────────────────────────────
-- schema.sql — el esquema MySQL del servidor de Questline
-- ─────────────────────────────────────────────────────────────────────────────
-- Questline MySQL Database Schema
-- Production-ready schema for HostGator MySQL deployments

CREATE TABLE IF NOT EXISTS users (
    id VARCHAR(36) PRIMARY KEY,
    public_key VARCHAR(64) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS devices (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    device_name VARCHAR(255) NOT NULL,
    last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS sync_events (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id VARCHAR(36) NOT NULL,
    operation VARCHAR(20) NOT NULL,
    payload LONGTEXT NOT NULL,
    created_at VARCHAR(50) NOT NULL, -- Client timestamp string for Latest Edit Wins
    INDEX idx_sync_events_user_entity (user_id, entity_type, entity_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS project_members (
    project_id VARCHAR(36) NOT NULL,
    user_identity VARCHAR(64) NOT NULL, -- Public Key
    user_username VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL, -- Owner, Steward, Companion, Observer
    PRIMARY KEY (project_id, user_identity)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS project_invitations (
    id VARCHAR(36) PRIMARY KEY,
    project_id VARCHAR(36) NOT NULL,
    project_name VARCHAR(255) NOT NULL,
    inviter_identity VARCHAR(64) NOT NULL,
    inviter_username VARCHAR(255) NOT NULL,
    invitee_identity VARCHAR(64) NOT NULL,
    role VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'Pending', -- Pending, Accepted, Declined
    created_at VARCHAR(50) NOT NULL,
    INDEX idx_invitee (invitee_identity)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS chronicle_messages (
    id VARCHAR(36) PRIMARY KEY,
    project_id VARCHAR(36) NOT NULL,
    sender_identity VARCHAR(64) NOT NULL,
    sender_username VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    message_type VARCHAR(50) NOT NULL,
    timestamp VARCHAR(50) NOT NULL,
    INDEX idx_project_msg (project_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS message_reactions (
    message_id VARCHAR(36) NOT NULL,
    user_identity VARCHAR(64) NOT NULL,
    emoji VARCHAR(50) NOT NULL,
    PRIMARY KEY (message_id, user_identity, emoji),
    FOREIGN KEY (message_id) REFERENCES chronicle_messages(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS project_permissions (
    role VARCHAR(50) NOT NULL,
    permission_key VARCHAR(50) NOT NULL,
    allowed TINYINT(1) NOT NULL DEFAULT 0,
    PRIMARY KEY (role, permission_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS backups (
    user_id VARCHAR(36) PRIMARY KEY,
    backup_data LONGTEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS nonces (
    user_id VARCHAR(36) NOT NULL,
    nonce VARCHAR(128) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, nonce)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS global_chronicle (
    id VARCHAR(36) PRIMARY KEY,
    hero_name VARCHAR(255) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    description TEXT NOT NULL,
    timestamp VARCHAR(50) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_global_chronicle_ts (timestamp)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS api_logs (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id VARCHAR(36) NULL,
    device_id VARCHAR(36) NULL,
    log_type VARCHAR(50) NOT NULL, -- AUTH_FAILURE, SYNC_FAILURE, API_ERROR, INVITATION, CHRONICLE
    message TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Living Chapters: cooperative global chapter progress
CREATE TABLE IF NOT EXISTS chapter_progress (
    chapter_id VARCHAR(50) PRIMARY KEY,
    completed TINYINT(1) NOT NULL DEFAULT 0,
    completed_at TIMESTAMP NULL,
    last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS chapter_objectives (
    chapter_id VARCHAR(50) NOT NULL,
    objective_type VARCHAR(50) NOT NULL,
    current_value BIGINT UNSIGNED NOT NULL DEFAULT 0,
    target_value BIGINT UNSIGNED NOT NULL,
    PRIMARY KEY (chapter_id, objective_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS chapter_contributions (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    chapter_id VARCHAR(50) NOT NULL,
    objective_type VARCHAR(50) NOT NULL,
    total_contributed BIGINT UNSIGNED NOT NULL DEFAULT 0,
    UNIQUE KEY uq_chapter_contrib (user_id, chapter_id, objective_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Seed default permissions
INSERT IGNORE INTO project_permissions (role, permission_key, allowed) VALUES
('Owner', 'write', 1),
('Owner', 'invite', 1),
('Owner', 'chat', 1),
('Steward', 'write', 1),
('Steward', 'invite', 1),
('Steward', 'chat', 1),
('Companion', 'write', 1),
('Companion', 'invite', 0),
('Companion', 'chat', 1),
('Observer', 'write', 0),
('Observer', 'invite', 0),
('Observer', 'chat', 1);
