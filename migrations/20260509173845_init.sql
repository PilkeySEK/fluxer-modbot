-- Add migration script here

CREATE TABLE guilds (
    guild_id BIGINT PRIMARY KEY,
    -- NULL if default prefix
    command_prefixes TEXT[] NOT NULL
);

CREATE UNIQUE INDEX idx_guilds ON guilds (guild_id);

-- CREATE TABLE guild_permissions (
--     guild_id BIGINT NOT NULL,
--     entity_id BIGINT NOT NULL,
--     -- "user" or "role"
--     entity_type TEXT NOT NULL,
--     permissions TEXT[] NOT NULL
-- );

CREATE TABLE guild_command_configuration (
    guild_id BIGINT NOT NULL,
    command_name TEXT NOT NULL,
    -- list of role IDs
    roles TEXT[] NOT NULL,
    -- permissions bitflags as a string
    permissions TEXT NOT NULL,
    -- list of channel IDs
    channels TEXT[] NOT NULL
);

CREATE UNIQUE INDEX idx_guild_command_configuration ON guild_command_configuration (guild_id, command_name);

CREATE TABLE guild_moderation_cases (
    case_id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    -- The user that this moderation case applies to.
    target_id BIGINT NOT NULL,
    -- NULL if it was an automated action.
    moderator_id BIGINT,
    -- "Warn", "Mute", "Kick", "Ban".
    moderation_kind TEXT NOT NULL,
    -- NULL if permanent
    expires_at TIMESTAMPTZ,
    -- NULL if no reason was given.
    reason TEXT,
    -- Whether this moderation case is closed.
    -- A moderation case is closed either when the expires_at time is reached (it expired)
    -- or when it is closed manually by a moderator.
    closed BOOLEAN NOT NULL DEFAULT FALSE,
    -- The initially set duration in seconds
    -- NULL if permanent
    duration BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    close_reason TEXT,
    -- NULL if the case automatically expired
    closed_by BIGINT
);

CREATE UNIQUE INDEX idx_guild_moderation_cases_by_guild_id_and_case_id ON guild_moderation_cases (case_id, guild_id);
CREATE INDEX idx_guild_moderation_cases_by_guild_id_and_target_id ON guild_moderation_cases (guild_id, target_id);
CREATE INDEX idx_guild_moderation_cases_by_guild_id_and_moderator_id ON guild_moderation_cases (guild_id, moderator_id);

CREATE TABLE guild_errors (
    log_entry_id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_guild_errors_by_log_entry_id ON guild_errors (log_entry_id);
CREATE INDEX idx_guild_errors_by_guild_id ON guild_errors (guild_id);