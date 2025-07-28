use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use {{SERVICE_NAME}}_domain::{DomainError, FileStorageService};

/// In-memory file storage service for testing and development
pub struct InMemoryFileStorageService {
    storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl InMemoryFileStorageService {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryFileStorageService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileStorageService for InMemoryFileStorageService {
    async fn upload_file(
        &self,
        key: &str,
        content: &[u8],
        _content_type: &str,
    ) -> Result<String, DomainError> {
        let mut storage = self.storage.lock().unwrap();
        storage.insert(key.to_string(), content.to_vec());
        Ok(format!("memory://{}", key))
    }

    async fn download_file(&self, key: &str) -> Result<Vec<u8>, DomainError> {
        let storage = self.storage.lock().unwrap();
        storage
            .get(key)
            .cloned()
            .ok_or_else(|| DomainError::entity_not_found("File", key))
    }

    async fn delete_file(&self, key: &str) -> Result<(), DomainError> {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(key);
        Ok(())
    }

    async fn generate_presigned_url(
        &self,
        key: &str,
        _expires_in_seconds: u32,
    ) -> Result<String, DomainError> {
        // In memory storage doesn't need presigned URLs
        Ok(format!("memory://{}", key))
    }
}

/// AWS S3 file storage service implementation
#[cfg(feature = "aws-sdk-s3")]
pub struct S3FileStorageService {
    // S3 client would go here
    bucket_name: String,
}

#[cfg(feature = "aws-sdk-s3")]
impl S3FileStorageService {
    pub async fn new(bucket_name: String) -> Result<Self, DomainError> {
        // Implementation would create S3 client
        Ok(Self { bucket_name })
    }
}

#[cfg(feature = "aws-sdk-s3")]
#[async_trait]
impl FileStorageService for S3FileStorageService {
    async fn upload_file(
        &self,
        _key: &str,
        _content: &[u8],
        _content_type: &str,
    ) -> Result<String, DomainError> {
        // Implementation would upload to S3
        todo!("Implement S3 file upload")
    }

    async fn download_file(&self, _key: &str) -> Result<Vec<u8>, DomainError> {
        // Implementation would download from S3
        todo!("Implement S3 file download")
    }

    async fn delete_file(&self, _key: &str) -> Result<(), DomainError> {
        // Implementation would delete from S3
        todo!("Implement S3 file deletion")
    }

    async fn generate_presigned_url(
        &self,
        _key: &str,
        _expires_in_seconds: u32,
    ) -> Result<String, DomainError> {
        // Implementation would generate S3 presigned URL
        todo!("Implement S3 presigned URL generation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_file_storage() {
        let service = InMemoryFileStorageService::new();
        let content = b"Hello, World!";
        let key = "test-file.txt";

        // Upload file
        let url = service
            .upload_file(key, content, "text/plain")
            .await
            .unwrap();
        assert_eq!(url, format!("memory://{}", key));

        // Download file
        let downloaded = service.download_file(key).await.unwrap();
        assert_eq!(downloaded, content);

        // Generate presigned URL
        let presigned_url = service.generate_presigned_url(key, 3600).await.unwrap();
        assert_eq!(presigned_url, format!("memory://{}", key));

        // Delete file
        service.delete_file(key).await.unwrap();

        // Verify file is deleted
        let result = service.download_file(key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_storage_multiple_files() {
        let service = InMemoryFileStorageService::new();

        // Upload multiple files
        service
            .upload_file("file1.txt", b"Content 1", "text/plain")
            .await
            .unwrap();
        service
            .upload_file("file2.txt", b"Content 2", "text/plain")
            .await
            .unwrap();
        service
            .upload_file("file3.txt", b"Content 3", "text/plain")
            .await
            .unwrap();

        // Download all files
        assert_eq!(
            service.download_file("file1.txt").await.unwrap(),
            b"Content 1"
        );
        assert_eq!(
            service.download_file("file2.txt").await.unwrap(),
            b"Content 2"
        );
        assert_eq!(
            service.download_file("file3.txt").await.unwrap(),
            b"Content 3"
        );

        // Delete one file and verify others remain
        service.delete_file("file2.txt").await.unwrap();
        assert_eq!(
            service.download_file("file1.txt").await.unwrap(),
            b"Content 1"
        );
        assert!(service.download_file("file2.txt").await.is_err());
        assert_eq!(
            service.download_file("file3.txt").await.unwrap(),
            b"Content 3"
        );
    }
} 