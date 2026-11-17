fn uni_freecam_camera(partial: f32, position: &mut [f64; 3], yaw: &mut f32, pitch: &mut f32) {
    crate::game::uni_freecam_glue::camera(partial, position, yaw, pitch);
}
