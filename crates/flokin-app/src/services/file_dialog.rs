use std::path::PathBuf;

pub fn pick_folder(title: String) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .pick_folder()
}
