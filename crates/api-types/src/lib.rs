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
