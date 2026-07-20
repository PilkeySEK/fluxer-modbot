use fluxer_neptunium::{
    exts::MessageExt,
    model::{
        gateway::payload::outgoing::{RequestGuildMembers, RequestGuildMembersQuery},
        id::{Id, marker::UserMarker},
    },
};

use crate::{
    commands::{CommandContext, CommandError},
    macros::embed_default_footer,
    util::{MaybeExpired, MaybeExpiringResult, confirmation::confirmation, parse_mention_or_id},
};

/// Expect the input to already be split from the rest of the args, so have no spaces.
///
/// May also return expired when the user rejected the confirmation dialog that may happen.
pub async fn parse_user_arg(
    ctx: &CommandContext<'_>,
    input: &str,
) -> MaybeExpiringResult<Option<Id<UserMarker>>, CommandError> {
    // Result<Option<Id<UserMarker>>, CommandError> {
    if let Some(user_id) = parse_mention_or_id(input) {
        Ok(MaybeExpired::NotExpired(Some(user_id)))
    } else {
        let users = ctx
            .ctx
            .request_guild_members(RequestGuildMembers {
                guild_ids: vec![ctx.guild_id],
                query: RequestGuildMembersQuery::Text(input.to_owned()),
                limit: Some(1),
                nonce: None,
                presences: None,
            })
            .await?;

        let Some(member) = users.first() else {
            return Ok(MaybeExpired::NotExpired(None));
        };

        let confirmation_reply = ctx
            .message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: format!(
                            "Is <@{}> (`{}#{}`) the user you're looking for?",
                            member.id,
                            member.user.username,
                            member.user.discriminator,
                        ),
                        color: 0xffffff,
                    }
                ),
            )
            .await?;
        let confirmation_result =
            confirmation(ctx, confirmation_reply, ctx.message.author.id).await?;
        match confirmation_result {
            MaybeExpired::NotExpired(true) => Ok(MaybeExpired::NotExpired(Some(member.id))),
            MaybeExpired::Expired | MaybeExpired::NotExpired(false) => Ok(MaybeExpired::Expired),
        }
    }
}
