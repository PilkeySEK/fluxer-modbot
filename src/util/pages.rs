use std::{sync::Arc, time::Duration};

use enumset::{EnumSet, EnumSetType};
use fluxer_neptunium::{
    cache::{Cached, CachedMessage},
    cached_payload::CachedMessageReactionAdd,
    exts::MessageExt,
    model::{
        guild::Emoji,
        id::{Id, marker::UserMarker},
    },
};
use tokio::sync::mpsc::unbounded_channel;

use crate::{
    commands::{CommandContext, CommandError},
    macros::debug_panic,
    util::{MaybeExpired, MaybeExpiringResult},
};

#[derive(EnumSetType)]
pub enum PageAction {
    Back,
    Continue,
}

pub async fn pages(
    ctx: &CommandContext<'_>,
    pages_message: Cached<CachedMessage>,
    allowed_reactor: Id<UserMarker>,
    allowed_actions: EnumSet<PageAction>,
    add_reactions: bool,
) -> MaybeExpiringResult<PageAction, CommandError> {
    const BACK: &str = "⬅️";
    const CONTINUE: &str = "➡️";

    enum PageMessage {
        Ok(PageAction),
        Expired,
    }

    if add_reactions {
        if let Err(e) = pages_message.add_reaction(ctx.ctx, BACK).await {
            return MaybeExpiringResult::Err(e.into());
        }
        if let Err(e) = pages_message.add_reaction(ctx.ctx, CONTINUE).await {
            return MaybeExpiringResult::Err(e.into());
        }
    }

    let (handler_tx, mut rx) = unbounded_channel();
    let expiry_handler_tx = handler_tx.clone();
    let ctx_clone = ctx.ctx.clone();
    ctx.register_reaction_handler(
        pages_message.id,
        move |event: Arc<CachedMessageReactionAdd>| {
            if event.user_id != allowed_reactor {
                return (false, Ok(()));
            }
            let Emoji::Default(emoji) = &event.emoji else {
                return (false, Ok(()));
            };
            if emoji == BACK {
                let ctx_clone = ctx_clone.clone();
                let user_id = event.user_id;
                let pages_message = pages_message.clone();
                tokio::spawn(async move {
                    let _ = pages_message
                        .delete_reaction(&ctx_clone, BACK, user_id)
                        .await;
                });
                if !allowed_actions.contains(PageAction::Back) {
                    return (false, Ok(()));
                }
                let _ = handler_tx.send(PageMessage::Ok(PageAction::Back));
                (true, Ok(()))
            } else if emoji == CONTINUE {
                let ctx_clone = ctx_clone.clone();
                let user_id = event.user_id;
                let pages_message = pages_message.clone();
                tokio::spawn(async move {
                    let result = pages_message
                        .delete_reaction(&ctx_clone, CONTINUE, user_id)
                        .await;
                    tracing::info!(?result);
                });
                if !allowed_actions.contains(PageAction::Continue) {
                    return (false, Ok(()));
                }
                let _ = handler_tx.send(PageMessage::Ok(PageAction::Continue));
                (true, Ok(()))
            } else {
                (false, Ok(()))
            }
        },
        Some((
            Box::new(move || {
                Box::pin(async move {
                    let _ = expiry_handler_tx.send(PageMessage::Expired);
                    Ok(())
                })
            }),
            Duration::from_mins(10),
        )),
    );

    if let Some(message) = rx.recv().await {
        match message {
            PageMessage::Expired => Ok(MaybeExpired::Expired),
            PageMessage::Ok(action) => {
                if !allowed_actions.contains(action) {
                    debug_panic!("allowed_actions does not contain the action.");
                }
                Ok(MaybeExpired::NotExpired(action))
            }
        }
    } else {
        debug_panic!("The channel should not close.");
        Ok(MaybeExpired::Expired)
    }
}
