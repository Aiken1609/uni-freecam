fn uni_freecam_turn(xo: f64, yo: f64) -> core::ops::ControlFlow<()> {
    crate::game::uni_freecam_glue::turn(xo, yo)
}
