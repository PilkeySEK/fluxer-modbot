use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::Context;
use enum_map::Enum;
use fluxer_neptunium::model::{
    guild::permissions::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
    },
    user::PartialUser,
};

mod manager;
pub use manager::*;
use serde::Deserialize;
use sqlx::prelude::FromRow;
use utoipa::{
    ToSchema,
    openapi::{ObjectBuilder, schema::SchemaType},
};

use crate::DurationSchema;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct CaseId(pub i64);

impl utoipa::ToSchema for CaseId {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("CaseId")
    }
}

impl utoipa::PartialSchema for CaseId {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        ObjectBuilder::new()
            .schema_type(SchemaType::Type(utoipa::openapi::Type::String))
            .build()
            .into()
    }
}

impl serde::Serialize for CaseId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

#[derive(serde::Serialize)]
pub struct UserInfo {
    pub user_id: Id<UserMarker>,
    pub avatar: Option<String>,
    pub username: String,
    pub discriminator: String,
    pub global_name: Option<String>,
}

#[derive(utoipa::ToSchema)]
pub struct UserInfoSchema {
    pub user_id: String,
    pub avatar: Option<String>,
    pub username: String,
    pub discriminator: String,
    pub global_name: Option<String>,
}

impl From<PartialUser> for UserInfo {
    fn from(value: PartialUser) -> Self {
        Self {
            user_id: value.id,
            avatar: value.avatar,
            username: value.username,
            discriminator: value.discriminator,
            global_name: value.global_name,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct RawGuildModerationCase {
    pub case_id: i64,
    pub guild_id: i64,
    pub target_id: i64,
    pub moderator_id: Option<i64>,
    pub moderation_kind: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reason: Option<String>,
    pub closed: bool,
    pub duration: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub close_reason: Option<String>,
    pub closed_by: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct GuildModerationCaseCloseData {
    pub reason: Option<String>,
    pub closed_by: Option<Id<UserMarker>>,
}

#[derive(utoipa::ToSchema)]
pub struct GuildModerationCaseCloseDataResponse {
    pub reason: Option<String>,
    pub closed_by: Option<String>,
}

#[derive(serde::Serialize, ToSchema)]
pub struct GuildModerationCaseExpiry {
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[schema(value_type = DurationSchema)]
    pub duration: Duration,
}

#[derive(utoipa::ToSchema)]
pub struct GuildModerationCaseResponse {
    pub case_id: CaseId,
    pub guild_id: String,
    pub target_id: String,
    pub moderator_id: Option<String>,
    pub moderation_kind: ModerationKind,
    pub expiry: Option<GuildModerationCaseExpiry>,
    pub reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// `None` if the case is not closed.
    pub close_data: Option<GuildModerationCaseCloseDataResponse>,
}

#[derive(serde::Serialize)]
pub struct GuildModerationCase {
    pub case_id: CaseId,
    pub guild_id: Id<GuildMarker>,
    pub target_id: Id<UserMarker>,
    pub moderator_id: Option<Id<UserMarker>>,
    pub moderation_kind: ModerationKind,
    pub expiry: Option<GuildModerationCaseExpiry>,
    pub reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// `None` if the case is not closed.
    pub close_data: Option<GuildModerationCaseCloseData>,
}

#[derive(strum::Display, strum::EnumString, PartialEq, Eq, utoipa::ToSchema, serde::Serialize)]
pub enum ModerationKind {
    Warn,
    Mute,
    Kick,
    Ban,
}

impl ModerationKind {
    /// Whether the associated case should be sent to the case expiration actor.
    pub fn manual_expiration(&self) -> bool {
        match self {
            Self::Warn | Self::Mute => true,
            Self::Kick | Self::Ban => false,
        }
    }
}

impl GuildModerationCase {
    pub fn from_raw(raw: RawGuildModerationCase) -> Option<Self> {
        Some(Self {
            case_id: CaseId(raw.case_id),
            guild_id: raw.guild_id.cast_unsigned().into(),
            target_id: raw.target_id.cast_unsigned().into(),
            moderator_id: raw.moderator_id.map(|id| id.cast_unsigned().into()),
            moderation_kind: ModerationKind::from_str(&raw.moderation_kind).ok()?,
            expiry: if let Some(expires_at) = raw.expires_at {
                let duration = raw.duration?;
                Some(GuildModerationCaseExpiry {
                    expires_at,
                    duration: Duration::from_secs(u64::try_from(duration).ok()?),
                })
            } else {
                None
            },
            reason: raw.reason,
            created_at: raw.created_at,
            close_data: if raw.closed {
                Some(GuildModerationCaseCloseData {
                    reason: raw.close_reason,
                    closed_by: raw.closed_by.map(|id| id.cast_unsigned().into()),
                })
            } else {
                None
            },
        })
    }
}

impl std::fmt::Display for CaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Try to encode it using SQIDS with the blocklist, which is very unlikely to fail.
        // It only fails when it has reached the maximum number of tries for getting around the
        // blocklist, in which case we use sqids without a blocklist, which may contain a bad
        // word but this is probably fine in practice. Either way, it would be better than
        // panicking if sqids fails.
        let sqids_encoded = match crate::SQIDS.encode(&[self.0.cast_unsigned()]) {
            Ok(encoded) => encoded,
            #[expect(
                clippy::unwrap_used,
                reason = "There is no blocklist so this can't fail."
            )]
            Err(_) => crate::SQIDS_NO_BLOCKLIST
                .encode(&[self.0.cast_unsigned()])
                .unwrap(),
        };
        f.write_str(&sqids_encoded)
    }
}

impl FromStr for CaseId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // I think sqids doesn't take any blocklists into account when decoding so this is fine
        // even if the original ID was generated using SQIDS_NO_BLOCKLIST
        let id = crate::SQIDS.decode(s);
        match id.first() {
            Some(id) => {
                let result = Self(id.cast_signed());
                if result.to_string() != s {
                    Err(())
                } else {
                    Ok(result)
                }
            }
            None => Err(()),
        }
    }
}

/// ID of a default bot command.
#[derive(
    strum::Display, strum::EnumString, Enum, Hash, PartialEq, Eq, Copy, Clone, Deserialize,
)]
pub enum CommandId {
    ListCases,
    CaseInfo,
    AddPrefix,
    RemovePrefix,
    ListPrefixes,
    SetModlogWebhook,
    ClearModlogWebhook,
    SetModlogChannel,
    Ping,
    Warn,
    Unwarn,
    Mute,
    Unmute,
    Kick,
    Ban,
    Unban,
    DeleteCase,
}

pub struct GuildCommandPermissionConfig {
    pub required_roles: Vec<Id<RoleMarker>>,
    pub required_permissions: Permissions,
    pub required_channels: Vec<Id<ChannelMarker>>,
}

pub struct GuildCommandConfig {
    pub guild_id: Id<GuildMarker>,
    pub command_id: CommandId,
    pub perms: Arc<GuildCommandPermissionConfig>,
    pub names: Vec<String>,
}

impl TryFrom<GuildCommandConfigSchema> for GuildCommandConfig {
    type Error = anyhow::Error;
    fn try_from(value: GuildCommandConfigSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            guild_id: value.guild_id.cast_unsigned().into(),
            command_id: CommandId::from_str(&value.command_id)
                .context("Failed to convert command ID")?,
            perms: Arc::new(GuildCommandPermissionConfig {
                required_roles: value
                    .required_roles
                    .into_iter()
                    .map(|id| id.cast_unsigned().into())
                    .collect::<Vec<_>>(),
                required_permissions: Permissions::from_bits_truncate(
                    value.required_permissions.cast_unsigned(),
                ),
                required_channels: value
                    .required_channels
                    .into_iter()
                    .map(|id| id.cast_unsigned().into())
                    .collect::<Vec<_>>(),
            }),
            names: value.names,
        })
    }
}

#[derive(FromRow)]
struct GuildCommandConfigSchema {
    pub guild_id: i64,
    pub command_id: String,
    pub required_roles: Vec<i64>,
    pub required_permissions: i64,
    pub required_channels: Vec<i64>,
    pub names: Vec<String>,
}
