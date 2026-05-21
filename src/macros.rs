macro_rules! embed_default_footer {
    ($ctx:expr, { $($tt:tt)* } $(,)?) => {
        ::fluxer_neptunium::create_embed!(
            $($tt)*,
            footer: {
                text: $ctx.bot_name,
                timestamp: ::fluxer_neptunium::model::time::OffsetDateTime::now_utc().into(),
            })
    };
}

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

/// Panic only if in debug mode, otherwise this is a no-op.
/// Useful for bugs that should be caught during development / testing, but are non-fatal.
macro_rules! debug_panic {
    ($($reason:tt)*) => {
        if cfg!(debug_assertions) {
            panic!($($reason)*);
        }
    };
}

pub(crate) use {debug_panic, embed_default_footer, try_parse_mention_or_id};
