#[derive(ts_rs::TS)]
#[ts(export)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct ApiGuild {
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

#[derive(ts_rs::TS)]
#[ts(export)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ApiErrorBody {
    pub error: String,
}
