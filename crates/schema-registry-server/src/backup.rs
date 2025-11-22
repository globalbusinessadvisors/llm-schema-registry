// Automated Backup Service
// Implements PostgreSQL backups with S3 storage, verification, and retention policies

use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub database_url: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub retention_daily: usize,  // Default: 30
    pub retention_monthly: usize, // Default: 12
    pub backup_dir: PathBuf,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/schema_registry".to_string()),
            s3_bucket: std::env::var("BACKUP_S3_BUCKET")
                .unwrap_or_else(|_| "schema-registry-backups".to_string()),
            s3_region: std::env::var("BACKUP_S3_REGION")
                .unwrap_or_else(|_| "us-east-1".to_string()),
            retention_daily: 30,
            retention_monthly: 12,
            backup_dir: PathBuf::from("/tmp/backups"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub id: Uuid,
    pub s3_key: String,
    pub size_bytes: u64,
    pub duration_seconds: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: Uuid,
    pub s3_key: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub verified: bool,
    pub backup_type: BackupType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupType {
    Daily,
    Monthly,
    Manual,
}

pub struct BackupService {
    config: BackupConfig,
    s3_client: S3Client,
}

impl BackupService {
    pub async fn new(config: BackupConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let aws_config = aws_config::load_from_env().await;
        let s3_client = S3Client::new(&aws_config);

        // Ensure backup directory exists
        fs::create_dir_all(&config.backup_dir).await?;

        Ok(Self { config, s3_client })
    }

    /// Run a complete backup job: dump, upload, verify, cleanup
    pub async fn run_backup_job(&self) -> Result<BackupResult, Box<dyn std::error::Error>> {
        let backup_id = Uuid::new_v4();
        let start_time = Utc::now();

        tracing::info!(
            backup_id = %backup_id,
            "Starting automated backup"
        );

        // Step 1: Create PostgreSQL dump
        let dump_path = self.create_pg_dump(backup_id).await?;
        tracing::info!(
            backup_id = %backup_id,
            path = ?dump_path,
            "PostgreSQL dump created"
        );

        // Step 2: Get file size
        let metadata = fs::metadata(&dump_path).await?;
        let size_bytes = metadata.len();

        // Step 3: Upload to S3
        let s3_key = self.generate_s3_key(start_time, backup_id);
        self.upload_to_s3(&dump_path, &s3_key).await?;
        tracing::info!(
            backup_id = %backup_id,
            s3_key = %s3_key,
            size_mb = size_bytes / 1024 / 1024,
            "Backup uploaded to S3"
        );

        // Step 4: Verify backup
        self.verify_backup(&s3_key).await?;
        tracing::info!(
            backup_id = %backup_id,
            "Backup verified successfully"
        );

        // Step 5: Record backup metadata
        self.record_backup_metadata(backup_id, &s3_key, size_bytes, start_time).await?;

        // Step 6: Cleanup old backups
        self.cleanup_old_backups().await?;

        // Step 7: Cleanup local files
        fs::remove_file(&dump_path).await?;

        let duration = Utc::now() - start_time;

        tracing::info!(
            backup_id = %backup_id,
            duration_seconds = duration.num_seconds(),
            size_mb = size_bytes / 1024 / 1024,
            "Backup completed successfully"
        );

        Ok(BackupResult {
            id: backup_id,
            s3_key,
            size_bytes,
            duration_seconds: duration.num_seconds(),
            created_at: start_time,
        })
    }

    /// Create PostgreSQL dump using pg_dump
    async fn create_pg_dump(&self, backup_id: Uuid) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dump_filename = format!("backup-{}.sql.gz", backup_id);
        let dump_path = self.config.backup_dir.join(&dump_filename);

        // Use pg_dump to create compressed backup
        let output = Command::new("pg_dump")
            .arg(&self.config.database_url)
            .arg("--format=plain")
            .arg("--no-owner")
            .arg("--no-acl")
            .arg("--verbose")
            .arg("--file")
            .arg(&dump_path)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pg_dump failed: {}", error).into());
        }

        // Compress the dump
        let output = Command::new("gzip")
            .arg("-f")
            .arg(&dump_path)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("gzip compression failed: {}", error).into());
        }

        Ok(PathBuf::from(format!("{}.gz", dump_path.display())))
    }

    /// Upload backup to S3
    async fn upload_to_s3(&self, local_path: &PathBuf, s3_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let body = ByteStream::from_path(local_path).await?;

        self.s3_client
            .put_object()
            .bucket(&self.config.s3_bucket)
            .key(s3_key)
            .body(body)
            .storage_class(aws_sdk_s3::types::StorageClass::StandardIa)
            .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::Aes256)
            .metadata("backup_type", "automated")
            .metadata("created_by", "schema-registry-backup-service")
            .send()
            .await?;

        Ok(())
    }

    /// Verify backup exists and is accessible
    async fn verify_backup(&self, s3_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.s3_client
            .head_object()
            .bucket(&self.config.s3_bucket)
            .key(s3_key)
            .send()
            .await?;

        // Verify content length is greater than 0
        if response.content_length().unwrap_or(0) == 0 {
            return Err("Backup file is empty".into());
        }

        Ok(())
    }

    /// Record backup metadata (in future: store in database)
    async fn record_backup_metadata(
        &self,
        backup_id: Uuid,
        s3_key: &str,
        size_bytes: u64,
        created_at: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let metadata = BackupMetadata {
            id: backup_id,
            s3_key: s3_key.to_string(),
            size_bytes,
            created_at,
            verified: true,
            backup_type: self.determine_backup_type(created_at),
        };

        // TODO: Store in database for tracking
        tracing::debug!(
            backup_metadata = ?metadata,
            "Backup metadata recorded"
        );

        Ok(())
    }

    /// Cleanup old backups according to retention policy
    async fn cleanup_old_backups(&self) -> Result<(), Box<dyn std::error::Error>> {
        // List all backups in S3
        let response = self.s3_client
            .list_objects_v2()
            .bucket(&self.config.s3_bucket)
            .prefix("backups/")
            .send()
            .await?;

        let mut daily_backups = Vec::new();
        let mut monthly_backups = Vec::new();

        if let Some(contents) = response.contents() {
            for object in contents {
                if let Some(key) = &object.key() {
                    if key.contains("/daily/") {
                        daily_backups.push(key.clone());
                    } else if key.contains("/monthly/") {
                        monthly_backups.push(key.clone());
                    }
                }
            }
        }

        // Sort by key (which includes date)
        daily_backups.sort();
        daily_backups.reverse();
        monthly_backups.sort();
        monthly_backups.reverse();

        // Delete old daily backups
        for key in daily_backups.iter().skip(self.config.retention_daily) {
            tracing::info!(key = %key, "Deleting old daily backup");
            self.s3_client
                .delete_object()
                .bucket(&self.config.s3_bucket)
                .key(key)
                .send()
                .await?;
        }

        // Delete old monthly backups
        for key in monthly_backups.iter().skip(self.config.retention_monthly) {
            tracing::info!(key = %key, "Deleting old monthly backup");
            self.s3_client
                .delete_object()
                .bucket(&self.config.s3_bucket)
                .key(key)
                .send()
                .await?;
        }

        Ok(())
    }

    /// Generate S3 key for backup
    fn generate_s3_key(&self, timestamp: DateTime<Utc>, backup_id: Uuid) -> String {
        let backup_type = self.determine_backup_type(timestamp);
        let type_str = match backup_type {
            BackupType::Daily => "daily",
            BackupType::Monthly => "monthly",
            BackupType::Manual => "manual",
        };

        format!(
            "backups/{}/{}/backup-{}.sql.gz",
            type_str,
            timestamp.format("%Y-%m-%d"),
            backup_id
        )
    }

    /// Determine if backup should be daily or monthly
    fn determine_backup_type(&self, timestamp: DateTime<Utc>) -> BackupType {
        // First day of month = monthly backup
        if timestamp.day() == 1 {
            BackupType::Monthly
        } else {
            BackupType::Daily
        }
    }

    /// Restore from backup
    pub async fn restore_from_backup(&self, backup_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!(backup_id = %backup_id, "Starting restore from backup");

        // Find backup in S3 (search all paths)
        let s3_key = self.find_backup_s3_key(backup_id).await?;

        // Download from S3
        let local_path = self.config.backup_dir.join(format!("restore-{}.sql.gz", backup_id));
        self.download_from_s3(&s3_key, &local_path).await?;

        // Decompress
        let output = Command::new("gunzip")
            .arg("-f")
            .arg(&local_path)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("gunzip decompression failed: {}", error).into());
        }

        // Restore to database
        let uncompressed_path = local_path.with_extension("");
        let output = Command::new("psql")
            .arg(&self.config.database_url)
            .arg("-f")
            .arg(&uncompressed_path)
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Database restore failed: {}", error).into());
        }

        // Cleanup
        fs::remove_file(&uncompressed_path).await?;

        tracing::info!(backup_id = %backup_id, "Restore completed successfully");

        Ok(())
    }

    /// Find backup S3 key by backup ID
    async fn find_backup_s3_key(&self, backup_id: Uuid) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.s3_client
            .list_objects_v2()
            .bucket(&self.config.s3_bucket)
            .prefix("backups/")
            .send()
            .await?;

        if let Some(contents) = response.contents() {
            for object in contents {
                if let Some(key) = &object.key() {
                    if key.contains(&backup_id.to_string()) {
                        return Ok(key.clone());
                    }
                }
            }
        }

        Err(format!("Backup {} not found", backup_id).into())
    }

    /// Download backup from S3
    async fn download_from_s3(&self, s3_key: &str, local_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.s3_client
            .get_object()
            .bucket(&self.config.s3_bucket)
            .key(s3_key)
            .send()
            .await?;

        let data = response.body.collect().await?;
        fs::write(local_path, data.into_bytes()).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_type_determination() {
        let config = BackupConfig::default();
        let service = BackupService {
            config,
            s3_client: S3Client::new(&aws_config::SdkConfig::builder().build()),
        };

        // First day of month
        let monthly = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(2, 0, 0)
            .unwrap();
        let monthly_utc = DateTime::<Utc>::from_naive_utc_and_offset(monthly, Utc);
        assert_eq!(service.determine_backup_type(monthly_utc), BackupType::Monthly);

        // Other day
        let daily = chrono::NaiveDate::from_ymd_opt(2025, 1, 15)
            .unwrap()
            .and_hms_opt(2, 0, 0)
            .unwrap();
        let daily_utc = DateTime::<Utc>::from_naive_utc_and_offset(daily, Utc);
        assert_eq!(service.determine_backup_type(daily_utc), BackupType::Daily);
    }
}
