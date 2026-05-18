use fluxer_neptunium::model::id::{Id, marker::UserMarker};

pub fn parse_mention_or_id(input: &str) -> Option<Id<UserMarker>> {
    Id::try_from(
        input
            .trim_start()
            .strip_prefix("<@")
            .and_then(|input| input.strip_suffix('>'))
            .unwrap_or(input),
    )
    .ok()
}
