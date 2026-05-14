use fluxer_neptunium::events::EventError;

use crate::commands::CommandContext;

pub async fn set_prefix(ctx: CommandContext<'_>, _args: &str) -> Result<(), EventError> {
    Ok(())
}
