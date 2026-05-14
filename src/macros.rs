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
