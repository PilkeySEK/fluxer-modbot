use fluxer_neptunium::model::guild::permissions::Permissions;

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

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToResponse)]
pub struct ApiErrorBody {
    pub error: String,
}

pub mod pg_notifications {
    use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
    use serde::{Deserialize, Serialize};

    pub const NOTIFICATION_GUILD_PREFIXES_UPDATE: &str = "guild_prefixes_update";

    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct GuildPrefixesUpdate(pub Id<GuildMarker>);
}

pub fn is_guild_manager_permissions(permissions: Permissions) -> bool {
    permissions.intersects(Permissions::ADMINISTRATOR)
        || permissions.intersects(Permissions::MANAGE_GUILD)
}
