use async_trait::async_trait;
use aws_sdk_s3::Client;
use notes_kit_core::error::StorageError;
use notes_kit_core::traits::StorageBackend;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct S3StorageBackend {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3StorageBackend {
    pub async fn new(bucket: impl Into<String>, prefix: Option<String>) -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Self::from_config(&config, bucket, prefix)
    }

    pub fn from_config(
        sdk_config: &aws_config::SdkConfig,
        bucket: impl Into<String>,
        prefix: Option<String>,
    ) -> Self {
        let s3_config = aws_sdk_s3::config::Builder::from(sdk_config)
            .force_path_style(true)
            .build();
        let client = Client::from_conf(s3_config);
        let prefix = match prefix {
            Some(p) if !p.is_empty() => {
                if p.ends_with('/') {
                    p
                } else {
                    format!("{p}/")
                }
            }
            _ => String::new(),
        };
        Self {
            client,
            bucket: bucket.into(),
            prefix,
        }
    }

    fn full_key(&self, path: &str) -> String {
        format!("{}{}", self.prefix, path)
    }

    async fn list_objects(
        &self,
        extension: &str,
        mut each: impl FnMut(&str, &aws_sdk_s3::types::Object),
    ) -> Result<(), StorageError> {
        let mut continuation_token: Option<String> = None;
        let suffix = format!(".{extension}");

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&self.prefix);

            if let Some(token) = continuation_token.take() {
                request = request.continuation_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| StorageError::Io(format!("S3 ListObjectsV2 failed: {e}")))?;

            for object in response.contents() {
                if let Some(key) = object.key() {
                    if key.ends_with(&suffix) {
                        let relative = key.strip_prefix(&self.prefix).unwrap_or(key);
                        each(relative, object);
                    }
                }
            }

            if response.is_truncated() == Some(true) {
                continuation_token =
                    response.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for S3StorageBackend {
    async fn list_files(&self, extension: &str) -> Result<Vec<String>, StorageError> {
        let mut files = Vec::new();
        self.list_objects(extension, |relative, _| {
            files.push(relative.to_string());
        })
        .await?;
        Ok(files)
    }

    async fn read_file(&self, path: &str) -> Result<String, StorageError> {
        let bytes = self.read_file_bytes(path).await?;
        String::from_utf8(bytes)
            .map_err(|e| StorageError::Io(format!("S3 object is not valid UTF-8: {e}")))
    }

    async fn read_file_bytes(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let key = self.full_key(path);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| match &e {
                aws_sdk_s3::error::SdkError::ServiceError(err)
                    if err.err().is_no_such_key() =>
                {
                    StorageError::NotFound(format!("S3 object not found: {key}"))
                }
                _ => StorageError::Io(format!("S3 GetObject failed for '{key}': {e}")),
            })?;

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::Io(format!("S3 body read failed: {e}")))?
            .into_bytes();

        Ok(bytes.to_vec())
    }

    async fn read_file_range(
        &self,
        path: &str,
        start: u64,
        max_len: u64,
    ) -> Result<notes_kit_core::traits::RangeRead, StorageError> {
        use notes_kit_core::traits::RangeRead;

        let key = self.full_key(path);
        let end_incl = start.saturating_add(max_len.max(1)) - 1;

        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .range(format!("bytes={start}-{end_incl}"))
            .send()
            .await;

        let response = match result {
            Ok(r) => r,
            Err(e) => {
                if let aws_sdk_s3::error::SdkError::ServiceError(err) = &e {
                    if err.err().is_no_such_key() {
                        return Err(StorageError::NotFound(format!(
                            "S3 object not found: {key}"
                        )));
                    }
                    // Start beyond end of object: report the total size so
                    // the caller can answer 416 with a Content-Range.
                    if err.err().meta().code() == Some("InvalidRange") {
                        let head = self
                            .client
                            .head_object()
                            .bucket(&self.bucket)
                            .key(&key)
                            .send()
                            .await
                            .map_err(|e| {
                                StorageError::Io(format!("S3 HeadObject failed for '{key}': {e}"))
                            })?;
                        return Ok(RangeRead {
                            bytes: Vec::new(),
                            total_size: head.content_length().unwrap_or(0).max(0) as u64,
                        });
                    }
                }
                return Err(StorageError::Io(format!(
                    "S3 ranged GetObject failed for '{key}': {e}"
                )));
            }
        };

        // "bytes start-end/total"
        let total_size = response
            .content_range()
            .and_then(|cr| cr.rsplit('/').next())
            .and_then(|t| t.parse::<u64>().ok());

        let bytes = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::Io(format!("S3 body read failed: {e}")))?
            .into_bytes()
            .to_vec();

        let total_size = total_size.unwrap_or(bytes.len() as u64);
        Ok(RangeRead { bytes, total_size })
    }

    async fn list_all_files(&self) -> Result<Vec<String>, StorageError> {
        let mut files = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&self.prefix);

            if let Some(token) = continuation_token.take() {
                request = request.continuation_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| StorageError::Io(format!("S3 ListObjectsV2 failed: {e}")))?;

            for object in response.contents() {
                if let Some(key) = object.key() {
                    let relative = key.strip_prefix(&self.prefix).unwrap_or(key);
                    if !relative.is_empty() && !relative.ends_with('/') {
                        files.push(relative.to_string());
                    }
                }
            }

            if response.is_truncated() == Some(true) {
                continuation_token =
                    response.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(files)
    }

    fn is_path_safe(&self, path: &str) -> bool {
        !path.contains("..") && !path.starts_with('/') && !path.contains('\0')
    }

    async fn listing_hash(&self, extension: &str) -> Result<Option<u64>, StorageError> {
        let mut entries: Vec<(String, String)> = Vec::new();
        self.list_objects(extension, |relative, object| {
            let etag = object.e_tag().unwrap_or("").to_string();
            entries.push((relative.to_string(), etag));
        })
        .await?;
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut hasher = DefaultHasher::new();
        for (path, etag) in &entries {
            path.hash(&mut hasher);
            etag.hash(&mut hasher);
        }
        Ok(Some(hasher.finish()))
    }
}
