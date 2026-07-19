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
