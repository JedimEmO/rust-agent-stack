use anyhow::Result;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub id: String,
    pub original_name: String,
    pub size: u64,
    pub content_type: Option<String>,
    pub stored_path: PathBuf,
}

pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    pub async fn save_file(
        &self,
        data: Vec<u8>,
        original_name: &str,
        content_type: Option<String>,
    ) -> Result<FileMetadata> {
        let file_id = Uuid::new_v4().to_string();
        let extension = std::path::Path::new(original_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("bin");

        let stored_name = format!("{}.{}", file_id, extension);
        let stored_path = self.base_path.join(&stored_name);

        // Save file
        tokio::fs::write(&stored_path, &data).await?;

        Ok(FileMetadata {
            id: file_id.clone(),
            original_name: original_name.to_string(),
            size: data.len() as u64,
            content_type,
            stored_path,
        })
    }

    pub async fn get_file(&self, file_id: &str) -> Result<(Vec<u8>, Option<FileMetadata>)> {
        // Find file with matching ID
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if file_name_str.starts_with(file_id) {
                let path = entry.path();
                let data = tokio::fs::read(&path).await?;
                let metadata = entry.metadata().await?;

                // Try to guess original name from extension
                let extension = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("bin");

                let file_metadata = FileMetadata {
                    id: file_id.to_string(),
                    original_name: format!("file.{}", extension),
                    size: metadata.len(),
                    content_type: mime_guess::from_path(&path)
                        .first()
                        .map(|mime| mime.to_string()),
                    stored_path: path,
                };

                return Ok((data, Some(file_metadata)));
            }
        }

        anyhow::bail!("File not found: {}", file_id)
    }

    pub async fn delete_file(&self, file_id: &str) -> Result<()> {
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if file_name_str.starts_with(file_id) {
                tokio::fs::remove_file(entry.path()).await?;
                return Ok(());
            }
        }

        anyhow::bail!("File not found: {}", file_id)
    }
}
