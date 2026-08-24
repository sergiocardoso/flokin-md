use std::path::PathBuf;

pub fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Abrir pasta")
        .pick_folder()
}
