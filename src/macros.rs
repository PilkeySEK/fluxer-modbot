#[macro_export]
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

#[macro_export]
macro_rules! try_db {
    ($ctx:expr, $operation:expr) => {
        match $operation.await {
            Ok(value) => value,
            Err(e) => {
                tracing::error!("Database error: {e}");
                ::fluxer_neptunium::exts::MessageExt::reply($ctx.message, $ctx.ctx, $crate::embed_default_footer!(
                    $ctx,
                    {
                        description: "Database error.",
                        color: 0xff0000,
                    }
                )).await?;
                return Ok(());
            }
        }
    };
}

#[macro_export]
macro_rules! try_parse_mention_or_id {
    ($ctx:expr, $args:expr) => {{
        let Some(target_id) = parse_mention_or_id($args) else {
            ::fluxer_neptunium::exts::MessageExt::reply(
                $ctx.message,
                $ctx.ctx,
                $crate::embed_default_footer!(
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
