use std::path::PathBuf;

pub async fn pick_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Abrir pasta")
        .pick_folder()
        .await
        .map(|handle| handle.path().to_path_buf())
}
