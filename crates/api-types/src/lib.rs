use fluxer_neptunium::model::{
    gateway::payload::incoming::UserPrivateResponse, guild::permissions::Permissions,
};

pub mod ws;

#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub splash: Option<String>,
    pub vanity_url_code: Option<String>,
    pub owner_id: String,
    // #[serde(rename = "permissions")]
    // pub current_user_permissions: Option<Permissions>,
    pub member_count: Option<usize>,
    pub online_count: Option<usize>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct FluxerUser {
    pub avatar: Option<String>,
    pub discriminator: String,
    pub id: String,
    pub is_staff: bool,
    pub username: String,
    pub global_name: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToResponse)]
pub struct ApiErrorBody {
    pub error: String,
}

impl From<UserPrivateResponse> for FluxerUser {
    fn from(value: UserPrivateResponse) -> Self {
        Self {
            avatar: value.avatar,
            discriminator: value.discriminator,
            id: value.id.to_string(),
            is_staff: value.is_staff,
            username: value.username,
            global_name: value.global_name,
        }
    }
}

/*
pub mod pg_notifications {
    use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
    use serde::{Deserialize, Serialize};

    pub const NOTIFICATION_GUILD_PREFIXES_UPDATE: &str = "guild_prefixes_update";

    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct GuildPrefixesUpdate(pub Id<GuildMarker>);
}
*/

pub fn is_guild_manager_permissions(permissions: Permissions) -> bool {
    permissions.intersects(Permissions::ADMINISTRATOR)
        || permissions.intersects(Permissions::MANAGE_GUILD)
}
