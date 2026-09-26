-- Add migration script here

CREATE TABLE guilds (
    guild_id BIGINT PRIMARY KEY,
    -- NULL if default prefix
    command_prefixes TEXT[] NOT NULL,
    -- NULL if no modlog webhook is set
    modlog_webhook_id BIGINT,
    -- NULL if no modlog webhook is set
    modlog_webhook_token TEXT,
    -- NULL if no modlog webhook is set or the webhook was set manually
    modlog_webhook_channel_id BIGINT,
    moderation_hierarchy_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    moderation_hierarchy_excluded_roles BIGINT[] NOT NULL DEFAULT ARRAY[]::BIGINT[]
);

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

CREATE TABLE dash_sessions (
    session_token TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    data JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_dash_sessions_expires_at ON dash_sessions (expires_at);

CREATE TABLE user_info (
    user_id BIGINT PRIMARY KEY,
    -- NULL if no avatar is set
    avatar TEXT,
    username TEXT NOT NULL,
    discriminator TEXT NOT NULL,
    -- NULL if no global name is set
    global_name TEXT
);

CREATE TABLE dashboard_user_settings (
    user_id BIGINT PRIMARY KEY,
    settings JSONB NOT NULL
);

CREATE TABLE guild_commands (
    guild_id BIGINT NOT NULL,
    command_id TEXT NOT NULL,
    required_roles BIGINT[] NOT NULL DEFAULT ARRAY[]::BIGINT[],
    required_permissions BIGINT NOT NULL,
    required_channels BIGINT[] NOT NULL DEFAULT ARRAY[]::BIGINT[],
    names TEXT[] NOT NULL
);

CREATE UNIQUE INDEX idx_guild_commands_by_guild_id_and_command_id ON guild_commands (guild_id, command_id);

CREATE TABLE cached_guild_roles (
    id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    color INTEGER NOT NULL,
    permissions TEXT NOT NULL
);