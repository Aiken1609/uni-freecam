fn uni_freecam_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::uni_freecam_glue::key(game, key, pressed);
}
