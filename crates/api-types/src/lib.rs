use std::{collections::HashSet, sync::LazyLock};

use fluxer_neptunium::model::{
    gateway::payload::incoming::UserPrivateResponse, guild::permissions::Permissions,
};
use sqids::{Sqids, SqidsBuilder};
use utoipa::ToSchema;

pub mod db;
pub mod ws;

const SQIDS_MIN_LENGTH: u8 = 5;
static SQIDS: LazyLock<Sqids> = LazyLock::new(|| {
    #[expect(
        clippy::unwrap_used,
        reason = "There is a test for the initialization being successful."
    )]
    SqidsBuilder::new()
        .min_length(SQIDS_MIN_LENGTH)
        .build()
        .unwrap()
});
static SQIDS_NO_BLOCKLIST: LazyLock<Sqids> = LazyLock::new(|| {
    #[expect(
        clippy::unwrap_used,
        reason = "There is a test for the initialization being successful."
    )]
    SqidsBuilder::new()
        .blocklist(HashSet::new())
        .min_length(SQIDS_MIN_LENGTH)
        .build()
        .unwrap()
});

#[derive(ToSchema)]
pub(crate) struct DurationSchema {
    #[expect(unused)]
    pub secs: u64,
    #[expect(unused)]
    pub nanos: u32,
}

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

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Default)]
pub struct DashboardUserSettings {
    pub show_duration_zeroes: bool,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DashboardUser {
    pub avatar: Option<String>,
    pub discriminator: String,
    pub id: String,
    pub is_staff: bool,
    pub username: String,
    pub global_name: Option<String>,
    pub settings: DashboardUserSettings,
}

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToResponse)]
pub struct ApiErrorBody {
    pub error: String,
}

impl From<(UserPrivateResponse, DashboardUserSettings)> for DashboardUser {
    fn from(value: (UserPrivateResponse, DashboardUserSettings)) -> Self {
        Self {
            avatar: value.0.avatar,
            discriminator: value.0.discriminator,
            id: value.0.id.to_string(),
            is_staff: value.0.is_staff,
            username: value.0.username,
            global_name: value.0.global_name,
            settings: value.1,
        }
    }
}

pub fn is_guild_manager_permissions(permissions: Permissions) -> bool {
    permissions.intersects(Permissions::ADMINISTRATOR)
        || permissions.intersects(Permissions::MANAGE_GUILD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqids_initialization() {
        let id_to_encode = 123;
        let encoded_id = SQIDS.encode(&[id_to_encode]);
        let encoded_id_no_blocklist = SQIDS.encode(&[id_to_encode]);
        assert!(encoded_id.is_ok());
        assert!(encoded_id_no_blocklist.is_ok());
    }
}
