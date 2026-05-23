macro_rules! embed_default_footer {
    ($ctx:expr, { $($tt:tt)* } $(,)?) => {
        $crate::macros::embed_default_footer_raw!(
            $ctx.bot_name,
            {
                $($tt)*
            }
        )
    };
}

macro_rules! embed_default_footer_raw {
    ($bot_name:expr, { $($tt:tt)* } $(,)?) => {
        ::fluxer_neptunium::create_embed!(
            $($tt)*,
            footer: {
                text: $bot_name,
                timestamp: ::fluxer_neptunium::model::time::OffsetDateTime::now_utc().into(),
            })
    };
}

/*
macro_rules! try_parse_mention_or_id {
    ($ctx:expr, $args:expr) => {{
        let Some(target_id) = parse_mention_or_id($args) else {
            ::fluxer_neptunium::exts::MessageExt::reply(
                $ctx.message,
                $ctx.ctx,
                $crate::macros::embed_default_footer!(
                    $ctx,
                    {
                        description: "Provide a target user ID or mention.",
                        color: 0xff0000,
                    }
                ),
            ).await?;
            return Ok(());
        };
        target_id
    }};
}
*/

/*
macro_rules! try_parse_user_arg {
    ($ctx:expr, $input:expr) => {{
        match $crate::util::user_arg::parse_user_arg(&$ctx, $input).await? {
            $crate::util::Expiry::Expired => return Ok(()),
            $crate::util::Expiry::NotExpired(Some(id)) => id,
            $crate::util::Expiry::NotExpired(None) => {
                ::fluxer_neptunium::exts::MessageExt::reply(
                    $ctx.message,
                    $ctx.ctx,
                    embed_default_footer!(
                        $ctx,
                        {
                            description: "Could not find a user matching your query.",
                            color: 0xff0000,
                        }
                    )
                ).await?;
                return Ok(());
            }
        }
    }};
}
*/

macro_rules! get_user_arg {
    ($ctx:expr, $input:expr, required) => {{
        let (user_arg_str, rest) = $input.split_once(' ').unwrap_or(($input, ""));
        let target_id = if user_arg_str.is_empty() {
            $ctx.message
                .reply(
                    $ctx.ctx,
                    $crate::macros::embed_default_footer!(
                        $ctx,
                        {
                            description: "Provide a target user ID or mention.",
                            color: 0xff0000,
                        }
                    ),
                ).await?;
            return Ok(());
        } else {
            match $crate::util::user_arg::parse_user_arg(&$ctx, user_arg_str).await? {
                $crate::util::Expiry::Expired => return Ok(()),
                $crate::util::Expiry::NotExpired(Some(id)) => id,
                $crate::util::Expiry::NotExpired(None) => {
                    ::fluxer_neptunium::exts::MessageExt::reply(
                        $ctx.message,
                        $ctx.ctx,
                        $crate::macros::embed_default_footer!(
                            $ctx,
                            {
                                description: "Could not find a user matching your query.",
                                color: 0xff0000,
                            }
                        )
                    ).await?;
                    return Ok(());
                }
            }
        };
        (target_id, rest)
    }};
    ($ctx:expr, $input:expr, not required) => {{
        let (user_arg_str, rest) = $input.split_once(' ').unwrap_or(($input, ""));
        let target_id = if user_arg_str.is_empty() {
            None
        } else {
            match $crate::util::user_arg::parse_user_arg(&$ctx, user_arg_str).await? {
                $crate::util::Expiry::Expired => return Ok(()),
                $crate::util::Expiry::NotExpired(Some(id)) => Some(id),
                $crate::util::Expiry::NotExpired(None) => {
                    ::fluxer_neptunium::exts::MessageExt::reply(
                        $ctx.message,
                        $ctx.ctx,
                        $crate::macros::embed_default_footer!(
                            $ctx,
                            {
                                description: "Could not find a user matching your query.",
                                color: 0xff0000,
                            }
                        )
                    ).await?;
                    return Ok(());
                }
            }
        };
        (target_id, if target_id.is_none() {
            $input
        } else {
            rest
        })
    }};
}

/// Panic only if in debug mode, otherwise this is a no-op.
/// Useful for bugs that should be caught during development / testing, but are non-fatal.
macro_rules! debug_panic {
    ($($reason:tt)*) => {
        if cfg!(debug_assertions) {
            panic!($($reason)*);
        }
    };
}

pub(crate) use {debug_panic, embed_default_footer, embed_default_footer_raw, get_user_arg};
