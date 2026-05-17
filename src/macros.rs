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
                $ctx.message.reply($ctx.ctx, embed_default_footer!(
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
