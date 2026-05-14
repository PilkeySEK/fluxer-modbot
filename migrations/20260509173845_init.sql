-- Add migration script here

CREATE TABLE guilds (
    guild_id BIGINT PRIMARY KEY,
    -- NULL if default prefix
    command_prefixes TEXT[]
);

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