fn uni_freecam_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::uni_freecam_glue::click(game, mod_id, button);
}
