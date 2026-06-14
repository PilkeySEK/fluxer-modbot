use std::sync::Arc;

use fluxer_neptunium::{
    exts::UserExt,
    model::id::{Id, marker::UserMarker},
};

use crate::commands::CommandContext;

pub fn fetch_and_add_users_to_db<const N: usize>(
    ctx: &CommandContext<'_>,
    user_ids: [Id<UserMarker>; N],
) -> tokio::task::JoinHandle<()> {
    let db = Arc::clone(ctx.db_arc);
    let ctx = ctx.ctx.clone();
    tokio::spawn(async move {
        for user_id in user_ids {
            match user_id.get_user(&ctx).await {
                Ok(user) => {
                    if let Err(e) = db.set_user_info(user.clone_inner().into()).await {
                        tracing::error!("Error setting user info for {user_id}: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Error fetching user {user_id}: {e}");
                }
            }
        }
    })
}
