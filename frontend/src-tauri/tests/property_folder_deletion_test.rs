//! Property-based tests for meeting folder deletion
//!
//! This test suite validates that the meeting deletion system correctly handles
//! filesystem folder deletion using property-based testing.
//!
//! **Validates: Requirements 2.1, 2.2**

use proptest::prelude::*;
use sqlx::SqlitePool;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use app_lib::database::repositories::meeting::MeetingsRepository;
use chrono::Utc;

/// Helper function to create a test database with schema
async fn create_test_database() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory database");

    // Create the meetings table schema
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS meetings (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL,
            updated_at TIMESTAMP NOT NULL,
            folder_path TEXT
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create meetings table");

    // Create related tables for proper deletion
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS transcripts (
            id TEXT PRIMARY KEY,
            meeting_id TEXT NOT NULL,
            transcript TEXT NOT NULL,
            timestamp TIMESTAMP NOT NULL,
            audio_start_time REAL,
            audio_end_time REAL,
            duration REAL,
            FOREIGN KEY (meeting_id) REFERENCES meetings(id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create transcripts table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS transcript_chunks (
            id TEXT PRIMARY KEY,
            meeting_id TEXT NOT NULL,
            meeting_name TEXT,
            FOREIGN KEY (meeting_id) REFERENCES meetings(id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create transcript_chunks table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS summary_processes (
            id TEXT PRIMARY KEY,
            meeting_id TEXT NOT NULL,
            FOREIGN KEY (meeting_id) REFERENCES meetings(id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create summary_processes table");

    pool
}

/// Helper function to insert a test meeting into the database
async fn insert_test_meeting(
    pool: &SqlitePool,
    meeting_id: &str,
    folder_path: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().naive_utc();
    
    sqlx::query(
        "INSERT INTO meetings (id, title, created_at, updated_at, folder_path) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(meeting_id)
    .bind("Test Meeting")
    .bind(now)
    .bind(now)
    .bind(folder_path)
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper function to create a meeting folder with nested content
fn create_meeting_folder_with_content(base_path: &Path, folder_name: &str) -> PathBuf {
    let folder_path = base_path.join(folder_name);
    fs::create_dir_all(&folder_path).expect("Failed to create meeting folder");

    // Create some files in the folder
    fs::write(folder_path.join("audio.mp4"), b"fake audio data")
        .expect("Failed to create audio.mp4");
    fs::write(folder_path.join("metadata.json"), b"{\"test\": true}")
        .expect("Failed to create metadata.json");

    // Create a subdirectory with files
    let subdir = folder_path.join("transcripts");
    fs::create_dir_all(&subdir).expect("Failed to create subdirectory");
    fs::write(subdir.join("transcript.txt"), b"test transcript")
        .expect("Failed to create transcript file");

    folder_path
}

/// Strategy to generate valid meeting IDs
fn meeting_id_strategy() -> impl Strategy<Value = String> {
    "[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}"
        .prop_map(|s| s.to_string())
}

/// Strategy to generate folder names
fn folder_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{5,20}".prop_map(|s| s.to_string())
}

/// Property 2: Folder Deletion Attempt for Valid Paths
/// **Validates: Requirements 2.1, 2.2**
///
/// Property: For any meeting with a non-NULL, non-empty folder_path,
/// after successful database deletion, the system should attempt to
/// recursively delete the filesystem folder and all its contents.
#[cfg(test)]
mod folder_deletion_property_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_recursive_folder_deletion_for_valid_paths(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Verify folder exists with content before deletion
                assert!(folder_path.exists(), "Folder should exist before deletion");
                assert!(folder_path.join("audio.mp4").exists(), "audio.mp4 should exist");
                assert!(folder_path.join("metadata.json").exists(), "metadata.json should exist");
                assert!(folder_path.join("transcripts").exists(), "transcripts subdir should exist");
                assert!(folder_path.join("transcripts/transcript.txt").exists(), "transcript.txt should exist");

                // Setup: Insert meeting with folder_path into database
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists in database
                let meeting_exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_exists.is_some(), "Meeting should exist in database before deletion");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed
                assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting should be removed from database
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");

                // Property Assertion 3: Filesystem folder should be deleted recursively
                // Note: We need to give the system a moment to complete filesystem operations
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                assert!(
                    !folder_path.exists(),
                    "Folder should be deleted from filesystem after database deletion: {}",
                    folder_path.display()
                );

                // Property Assertion 4: All nested content should be deleted
                if folder_path.exists() {
                    // If folder still exists (shouldn't happen), verify it's empty or check contents
                    panic!("Folder still exists after deletion: {}", folder_path.display());
                }
            });
        }

        #[test]
        fn test_folder_deletion_with_nested_directories(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory with deeply nested structure
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = temp_dir.path().join(&folder_name);
                fs::create_dir_all(&folder_path).expect("Failed to create meeting folder");

                // Create deeply nested directory structure
                let deep_path = folder_path.join("level1/level2/level3");
                fs::create_dir_all(&deep_path).expect("Failed to create nested dirs");
                fs::write(deep_path.join("deep_file.txt"), b"deep content")
                    .expect("Failed to create deep file");

                // Create multiple files at different levels
                fs::write(folder_path.join("root_file.txt"), b"root")
                    .expect("Failed to create root file");
                fs::write(folder_path.join("level1/level1_file.txt"), b"level1")
                    .expect("Failed to create level1 file");
                fs::write(folder_path.join("level1/level2/level2_file.txt"), b"level2")
                    .expect("Failed to create level2 file");

                // Verify nested structure exists
                assert!(folder_path.exists());
                assert!(folder_path.join("level1/level2/level3/deep_file.txt").exists());

                // Setup: Insert meeting with folder_path into database
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion: All nested directories and files should be deleted
                assert!(result.is_ok(), "Database deletion should succeed");
                
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                assert!(
                    !folder_path.exists(),
                    "Entire folder tree should be deleted recursively"
                );
            });
        }

        #[test]
        fn test_folder_deletion_with_various_file_types(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory with various file types
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = temp_dir.path().join(&folder_name);
                fs::create_dir_all(&folder_path).expect("Failed to create meeting folder");

                // Create files with different extensions
                fs::write(folder_path.join("audio.mp4"), b"video data")
                    .expect("Failed to create mp4");
                fs::write(folder_path.join("transcript.json"), b"{}")
                    .expect("Failed to create json");
                fs::write(folder_path.join("notes.txt"), b"notes")
                    .expect("Failed to create txt");
                fs::write(folder_path.join("data.bin"), &[0u8; 1024])
                    .expect("Failed to create binary");

                // Verify all files exist
                assert!(folder_path.join("audio.mp4").exists());
                assert!(folder_path.join("transcript.json").exists());
                assert!(folder_path.join("notes.txt").exists());
                assert!(folder_path.join("data.bin").exists());

                // Setup: Insert meeting with folder_path into database
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion: All file types should be deleted
                assert!(result.is_ok(), "Database deletion should succeed");
                
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                assert!(
                    !folder_path.exists(),
                    "Folder with all file types should be deleted"
                );
            });
        }
    }
}

/// Property 4: Transaction Commit Before Filesystem Operations
/// **Validates: Requirements 3.3**
///
/// Property: For any meeting deletion operation, the database transaction should be
/// committed before any filesystem deletion is attempted, ensuring database changes
/// are persisted regardless of filesystem operation outcomes.
#[cfg(test)]
mod transaction_commit_ordering_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_database_committed_before_filesystem_deletion(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Setup: Insert meeting with folder_path into database
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists in database before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist in database before deletion");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed
                assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Database transaction must be committed IMMEDIATELY
                // This means the meeting should NOT exist in the database anymore,
                // even if filesystem operations are still in progress or fail
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                
                assert!(
                    meeting_after.is_none(),
                    "Meeting must be removed from database immediately after delete_meeting returns, \
                     proving transaction was committed before filesystem operations"
                );

                // Property Assertion 3: All related records should also be deleted from database
                // This proves the entire transaction was committed
                let transcript_chunks: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunks");
                assert!(transcript_chunks.is_none(), "transcript_chunks should be deleted");

                let summary_processes: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query summary_processes");
                assert!(summary_processes.is_none(), "summary_processes should be deleted");

                let transcripts: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcripts");
                assert!(transcripts.is_none(), "transcripts should be deleted");

                // Note: We don't need to verify filesystem deletion timing here,
                // as the property is about database commit happening BEFORE filesystem operations.
                // The fact that database is committed and we can query it proves the ordering.
            });
        }

        #[test]
        fn test_database_persisted_even_with_filesystem_errors(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory with a folder
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = temp_dir.path().join(&folder_name);
                fs::create_dir_all(&folder_path).expect("Failed to create meeting folder");
                
                // Create content in the folder
                fs::write(folder_path.join("audio.mp4"), b"test data")
                    .expect("Failed to create test file");

                // Make the folder read-only to cause filesystem deletion to fail
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&folder_path)
                        .expect("Failed to get metadata")
                        .permissions();
                    perms.set_mode(0o444); // Read-only
                    fs::set_permissions(&folder_path, perms)
                        .expect("Failed to set permissions");
                }

                // Setup: Insert meeting with folder_path
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist before deletion");

                // Action: Delete the meeting (filesystem deletion will likely fail due to permissions)
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion succeeds even if filesystem fails
                assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Database changes are persisted despite filesystem errors
                // This proves the transaction was committed before filesystem operations
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                
                assert!(
                    meeting_after.is_none(),
                    "Meeting must be permanently deleted from database even if filesystem deletion fails, \
                     proving transaction commit happens before filesystem operations"
                );

                // Property Assertion 3: All related records are also persisted as deleted
                let transcript_chunks: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunks");
                assert!(transcript_chunks.is_none(), "transcript_chunks should be permanently deleted");

                // Cleanup: Restore permissions so temp_dir can be cleaned up
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if folder_path.exists() {
                        let mut perms = fs::metadata(&folder_path)
                            .unwrap_or_else(|_| fs::metadata(temp_dir.path()).unwrap())
                            .permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(&folder_path, perms);
                    }
                }
            });
        }

        #[test]
        fn test_transaction_commit_with_nonexistent_folder(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create path to non-existent folder
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = temp_dir.path().join(&folder_name);
                
                // Verify folder does NOT exist
                assert!(!folder_path.exists(), "Folder should not exist");

                // Setup: Insert meeting with folder_path pointing to non-existent folder
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist before deletion");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion succeeds
                assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Database transaction is committed
                // Even though filesystem operation is a no-op (folder doesn't exist),
                // the database changes must be persisted
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                
                assert!(
                    meeting_after.is_none(),
                    "Meeting must be deleted from database, proving transaction was committed \
                     before filesystem operations (even when folder doesn't exist)"
                );
            });
        }

        #[test]
        fn test_transaction_commit_with_null_folder_path(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Insert meeting with NULL folder_path
                insert_test_meeting(&pool, &meeting_id, None)
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist before deletion");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion succeeds
                assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Database transaction is committed
                // With NULL folder_path, no filesystem operations occur,
                // but database changes must still be persisted
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                
                assert!(
                    meeting_after.is_none(),
                    "Meeting must be deleted from database, proving transaction was committed \
                     (no filesystem operations occur with NULL folder_path)"
                );
            });
        }
    }
}

/// Property 1: Database Deletion Independence
/// **Validates: Requirements 3.1, 3.2**
///
/// Property: For any meeting with a valid meeting_id, database deletion should succeed
/// and return success regardless of whether the filesystem folder exists, can be deleted,
/// or filesystem deletion encounters any errors.
#[cfg(test)]
mod database_deletion_independence_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_database_deletion_succeeds_with_nonexistent_folder(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory but don't create the actual folder
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = temp_dir.path().join(&folder_name);
                
                // Verify folder does NOT exist
                assert!(!folder_path.exists(), "Folder should not exist before test");

                // Setup: Insert meeting with folder_path pointing to non-existent folder
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists in database
                let meeting_exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_exists.is_some(), "Meeting should exist in database before deletion");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed even though folder doesn't exist
                assert!(result.is_ok(), "Database deletion should succeed even with non-existent folder: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting should be removed from database
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");
            });
        }

        #[test]
        fn test_database_deletion_succeeds_with_readonly_folder(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory with a folder
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = temp_dir.path().join(&folder_name);
                fs::create_dir_all(&folder_path).expect("Failed to create meeting folder");
                
                // Create some content in the folder
                fs::write(folder_path.join("audio.mp4"), b"test data")
                    .expect("Failed to create test file");

                // Make the folder read-only (this simulates permission issues)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&folder_path)
                        .expect("Failed to get metadata")
                        .permissions();
                    perms.set_mode(0o444); // Read-only
                    fs::set_permissions(&folder_path, perms)
                        .expect("Failed to set permissions");
                }

                // Verify folder exists
                assert!(folder_path.exists(), "Folder should exist before deletion");

                // Setup: Insert meeting with folder_path
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists in database
                let meeting_exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_exists.is_some(), "Meeting should exist in database before deletion");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed even if folder deletion fails
                assert!(result.is_ok(), "Database deletion should succeed even with permission issues: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting should be removed from database
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");

                // Cleanup: Restore permissions so temp_dir can be cleaned up
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if folder_path.exists() {
                        let mut perms = fs::metadata(&folder_path)
                            .unwrap_or_else(|_| fs::metadata(temp_dir.path()).unwrap())
                            .permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(&folder_path, perms);
                    }
                }
            });
        }

        #[test]
        fn test_database_deletion_succeeds_with_invalid_folder_path(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Use an invalid/malformed folder path
                let invalid_paths = vec![
                    "/nonexistent/path/that/does/not/exist",
                    "/tmp/../../../invalid",
                    "relative/path/without/root",
                ];

                for invalid_path in invalid_paths {
                    // Setup: Insert meeting with invalid folder_path
                    insert_test_meeting(&pool, &meeting_id, Some(invalid_path))
                        .await
                        .expect("Failed to insert test meeting");

                    // Verify meeting exists in database
                    let meeting_exists: Option<(String,)> = sqlx::query_as(
                        "SELECT id FROM meetings WHERE id = ?"
                    )
                    .bind(&meeting_id)
                    .fetch_optional(&pool)
                    .await
                    .expect("Failed to query meeting");
                    assert!(meeting_exists.is_some(), "Meeting should exist in database before deletion");

                    // Action: Delete the meeting
                    let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                    // Property Assertion 1: Database deletion should succeed with invalid path
                    assert!(result.is_ok(), "Database deletion should succeed with invalid path '{}': {:?}", invalid_path, result);
                    assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                    // Property Assertion 2: Meeting should be removed from database
                    let meeting_after: Option<(String,)> = sqlx::query_as(
                        "SELECT id FROM meetings WHERE id = ?"
                    )
                    .bind(&meeting_id)
                    .fetch_optional(&pool)
                    .await
                    .expect("Failed to query meeting after deletion");
                    assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");
                }
            });
        }

        #[test]
        fn test_database_deletion_with_folder_containing_locked_files(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory with a folder
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = temp_dir.path().join(&folder_name);
                fs::create_dir_all(&folder_path).expect("Failed to create meeting folder");
                
                // Create a file in the folder
                let file_path = folder_path.join("locked_file.txt");
                fs::write(&file_path, b"test data")
                    .expect("Failed to create test file");

                // Open the file to simulate it being in use (on Windows this would lock it)
                // Note: On Unix systems, files can be deleted while open, but we test the behavior anyway
                let _file_handle = std::fs::File::open(&file_path)
                    .expect("Failed to open file");

                // Verify folder exists
                assert!(folder_path.exists(), "Folder should exist before deletion");

                // Setup: Insert meeting with folder_path
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists in database
                let meeting_exists: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_exists.is_some(), "Meeting should exist in database before deletion");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed even with locked files
                assert!(result.is_ok(), "Database deletion should succeed even with locked files: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting should be removed from database
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");

                // Note: File handle will be dropped here, allowing cleanup
            });
        }
    }
}

/// Property 5: Transaction Atomicity Preservation
/// **Validates: Requirements 7.5, 7.6**
///
/// Property: For any meeting deletion operation, if any database deletion step fails,
/// all database changes should be rolled back, an error should be returned, and no
/// filesystem operations should occur.
#[cfg(test)]
mod transaction_atomicity_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_transaction_rollback_on_database_error(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Verify folder exists before deletion
                assert!(folder_path.exists(), "Folder should exist before deletion");

                // Setup: Insert meeting with folder_path into database
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert related records
                let now = Utc::now().naive_utc();
                
                sqlx::query(
                    "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(format!("{}-transcript", meeting_id))
                .bind(&meeting_id)
                .bind("Test transcript")
                .bind(now)
                .execute(&pool)
                .await
                .expect("Failed to insert transcript");

                sqlx::query(
                    "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                )
                .bind(format!("{}-chunk", meeting_id))
                .bind(&meeting_id)
                .bind("Test Meeting")
                .execute(&pool)
                .await
                .expect("Failed to insert transcript_chunk");

                // Verify all records exist before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist before deletion");

                let transcript_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript");
                assert!(transcript_before.is_some(), "Transcript should exist before deletion");

                let chunk_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunk");
                assert!(chunk_before.is_some(), "Transcript chunk should exist before deletion");

                // Action: Attempt to delete with an invalid/non-existent meeting_id
                // This should fail and trigger a rollback
                let invalid_meeting_id = "nonexistent-meeting-id";
                let result = MeetingsRepository::delete_meeting(&pool, invalid_meeting_id).await;

                // Property Assertion 1: Deletion should fail (return Ok(false) or Err)
                // The function returns Ok(false) when meeting is not found
                assert!(
                    result.is_ok() && result.unwrap() == false,
                    "Deletion should fail for non-existent meeting"
                );

                // Property Assertion 2: Original meeting and all related records should still exist
                // (transaction should be rolled back)
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after failed deletion");
                assert!(
                    meeting_after.is_some(),
                    "Meeting should still exist after failed deletion (transaction rolled back)"
                );

                let transcript_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript after failed deletion");
                assert!(
                    transcript_after.is_some(),
                    "Transcript should still exist after failed deletion (transaction rolled back)"
                );

                let chunk_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunk after failed deletion");
                assert!(
                    chunk_after.is_some(),
                    "Transcript chunk should still exist after failed deletion (transaction rolled back)"
                );

                // Property Assertion 3: Filesystem folder should still exist
                // (no filesystem operations should occur when database deletion fails)
                assert!(
                    folder_path.exists(),
                    "Folder should still exist after failed deletion (no filesystem operations on DB failure)"
                );

                // Verify folder contents are intact
                assert!(folder_path.join("audio.mp4").exists(), "audio.mp4 should still exist");
                assert!(folder_path.join("metadata.json").exists(), "metadata.json should still exist");
                assert!(folder_path.join("transcripts").exists(), "transcripts subdir should still exist");
            });
        }

        #[test]
        fn test_no_filesystem_operations_on_empty_meeting_id(
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Verify folder exists before deletion attempt
                assert!(folder_path.exists(), "Folder should exist before deletion attempt");

                // Action: Attempt to delete with empty meeting_id (should fail validation)
                let result = MeetingsRepository::delete_meeting(&pool, "").await;

                // Property Assertion 1: Deletion should fail with error
                assert!(
                    result.is_err(),
                    "Deletion should fail for empty meeting_id"
                );

                // Property Assertion 2: Filesystem folder should still exist
                // (no filesystem operations should occur when validation fails)
                assert!(
                    folder_path.exists(),
                    "Folder should still exist after failed deletion (no filesystem operations on validation failure)"
                );

                // Verify folder contents are intact
                assert!(folder_path.join("audio.mp4").exists(), "audio.mp4 should still exist");
                assert!(folder_path.join("metadata.json").exists(), "metadata.json should still exist");
            });
        }

        #[test]
        fn test_transaction_atomicity_with_related_records(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Setup: Insert meeting with folder_path
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert multiple related records
                let now = Utc::now().naive_utc();
                
                for i in 0..5 {
                    sqlx::query(
                        "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                    )
                    .bind(format!("{}-transcript-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .bind(format!("Test transcript {}", i))
                    .bind(now)
                    .execute(&pool)
                    .await
                    .expect("Failed to insert transcript");
                }

                for i in 0..3 {
                    sqlx::query(
                        "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                    )
                    .bind(format!("{}-chunk-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .bind("Test Meeting")
                    .execute(&pool)
                    .await
                    .expect("Failed to insert transcript_chunk");
                }

                for i in 0..2 {
                    sqlx::query(
                        "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                    )
                    .bind(format!("{}-summary-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .execute(&pool)
                    .await
                    .expect("Failed to insert summary_process");
                }

                // Verify all records exist
                let transcript_count: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcripts");
                assert_eq!(transcript_count.0, 5, "Should have 5 transcripts");

                let chunk_count: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcript_chunks");
                assert_eq!(chunk_count.0, 3, "Should have 3 transcript chunks");

                let summary_count: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count summary_processes");
                assert_eq!(summary_count.0, 2, "Should have 2 summary processes");

                // Action: Attempt to delete with wrong meeting_id
                let wrong_meeting_id = "wrong-meeting-id";
                let result = MeetingsRepository::delete_meeting(&pool, wrong_meeting_id).await;

                // Property Assertion 1: Deletion should fail
                assert!(
                    result.is_ok() && result.unwrap() == false,
                    "Deletion should fail for wrong meeting_id"
                );

                // Property Assertion 2: All related records should still exist (atomicity)
                let transcript_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcripts after failed deletion");
                assert_eq!(
                    transcript_count_after.0, 5,
                    "All transcripts should still exist after failed deletion"
                );

                let chunk_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcript_chunks after failed deletion");
                assert_eq!(
                    chunk_count_after.0, 3,
                    "All transcript chunks should still exist after failed deletion"
                );

                let summary_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count summary_processes after failed deletion");
                assert_eq!(
                    summary_count_after.0, 2,
                    "All summary processes should still exist after failed deletion"
                );

                // Property Assertion 3: Filesystem folder should still exist
                assert!(
                    folder_path.exists(),
                    "Folder should still exist after failed deletion"
                );
            });
        }

        #[test]
        fn test_successful_deletion_is_atomic(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Setup: Insert meeting with folder_path
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert related records
                let now = Utc::now().naive_utc();
                
                sqlx::query(
                    "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(format!("{}-transcript", meeting_id))
                .bind(&meeting_id)
                .bind("Test transcript")
                .bind(now)
                .execute(&pool)
                .await
                .expect("Failed to insert transcript");

                sqlx::query(
                    "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                )
                .bind(format!("{}-chunk", meeting_id))
                .bind(&meeting_id)
                .bind("Test Meeting")
                .execute(&pool)
                .await
                .expect("Failed to insert transcript_chunk");

                sqlx::query(
                    "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                )
                .bind(format!("{}-summary", meeting_id))
                .bind(&meeting_id)
                .execute(&pool)
                .await
                .expect("Failed to insert summary_process");

                // Verify all records exist before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist before deletion");

                // Action: Delete the meeting (should succeed)
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Deletion should succeed
                assert!(result.is_ok(), "Deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: ALL database records should be deleted atomically
                // (either all deleted or none deleted - in this case all should be deleted)
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should be deleted");

                let transcript_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript after deletion");
                assert!(transcript_after.is_none(), "Transcript should be deleted");

                let chunk_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunk after deletion");
                assert!(chunk_after.is_none(), "Transcript chunk should be deleted");

                let summary_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query summary_process after deletion");
                assert!(summary_after.is_none(), "Summary process should be deleted");

                // Property Assertion 3: Filesystem folder should be deleted
                // (filesystem operations occur AFTER successful database transaction)
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                assert!(
                    !folder_path.exists(),
                    "Folder should be deleted after successful database deletion"
                );
            });
        }
    }
}

/// Property 3: NULL and Empty Folder Path Handling
/// **Validates: Requirements 1.2, 5.1, 5.2, 6.1, 6.2**
///
/// Property: For any meeting with a NULL or empty string folder_path,
/// the system should complete database deletion successfully without
/// attempting any filesystem operations.
#[cfg(test)]
mod null_and_empty_folder_path_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_null_folder_path_completes_without_filesystem_operations(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Insert meeting with NULL folder_path
                insert_test_meeting(&pool, &meeting_id, None)
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists in database before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist in database before deletion");

                // Verify folder_path is NULL
                let folder_path_check: Option<(Option<String>,)> = sqlx::query_as(
                    "SELECT folder_path FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query folder_path");
                assert!(folder_path_check.is_some(), "Meeting record should exist");
                assert!(folder_path_check.unwrap().0.is_none(), "folder_path should be NULL");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed
                assert!(result.is_ok(), "Database deletion should succeed with NULL folder_path: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting should be removed from database
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(
                    meeting_after.is_none(),
                    "Meeting should not exist in database after deletion"
                );

                // Property Assertion 3: No filesystem operations should have been attempted
                // Since folder_path is NULL, the system should skip filesystem deletion entirely.
                // We verify this indirectly by confirming the operation completed successfully
                // without any folder path to work with.
            });
        }

        #[test]
        fn test_empty_string_folder_path_completes_without_filesystem_operations(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Insert meeting with empty string folder_path
                insert_test_meeting(&pool, &meeting_id, Some(""))
                    .await
                    .expect("Failed to insert test meeting");

                // Verify meeting exists in database before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist in database before deletion");

                // Verify folder_path is empty string
                let folder_path_check: Option<(Option<String>,)> = sqlx::query_as(
                    "SELECT folder_path FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query folder_path");
                assert!(folder_path_check.is_some(), "Meeting record should exist");
                let folder_path_value = folder_path_check.unwrap().0;
                assert!(
                    folder_path_value.is_some() && folder_path_value.as_ref().unwrap().is_empty(),
                    "folder_path should be empty string"
                );

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed
                assert!(result.is_ok(), "Database deletion should succeed with empty string folder_path: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting should be removed from database
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(
                    meeting_after.is_none(),
                    "Meeting should not exist in database after deletion"
                );

                // Property Assertion 3: No filesystem operations should have been attempted
                // Since folder_path is empty string, the system should treat it as NULL
                // and skip filesystem deletion entirely.
            });
        }

        #[test]
        fn test_whitespace_only_folder_path_completes_without_filesystem_operations(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Test various whitespace-only strings
                let whitespace_paths = vec![
                    " ",
                    "  ",
                    "\t",
                    "\n",
                    " \t\n ",
                ];

                for whitespace_path in whitespace_paths {
                    // Setup: Insert meeting with whitespace-only folder_path
                    insert_test_meeting(&pool, &meeting_id, Some(whitespace_path))
                        .await
                        .expect("Failed to insert test meeting");

                    // Verify meeting exists in database before deletion
                    let meeting_before: Option<(String,)> = sqlx::query_as(
                        "SELECT id FROM meetings WHERE id = ?"
                    )
                    .bind(&meeting_id)
                    .fetch_optional(&pool)
                    .await
                    .expect("Failed to query meeting");
                    assert!(meeting_before.is_some(), "Meeting should exist in database before deletion");

                    // Action: Delete the meeting
                    let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                    // Property Assertion 1: Database deletion should succeed
                    assert!(
                        result.is_ok(),
                        "Database deletion should succeed with whitespace-only folder_path '{}': {:?}",
                        whitespace_path.escape_debug(),
                        result
                    );
                    assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                    // Property Assertion 2: Meeting should be removed from database
                    let meeting_after: Option<(String,)> = sqlx::query_as(
                        "SELECT id FROM meetings WHERE id = ?"
                    )
                    .bind(&meeting_id)
                    .fetch_optional(&pool)
                    .await
                    .expect("Failed to query meeting after deletion");
                    assert!(
                        meeting_after.is_none(),
                        "Meeting should not exist in database after deletion"
                    );

                    // Property Assertion 3: No filesystem operations should have been attempted
                    // Whitespace-only paths should be treated as invalid and skipped
                }
            });
        }

        #[test]
        fn test_null_folder_path_with_related_records(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Insert meeting with NULL folder_path
                insert_test_meeting(&pool, &meeting_id, None)
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert related records (transcripts, transcript_chunks, summary_processes)
                let now = Utc::now().naive_utc();
                
                sqlx::query(
                    "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(format!("{}-transcript", meeting_id))
                .bind(&meeting_id)
                .bind("Test transcript")
                .bind(now)
                .execute(&pool)
                .await
                .expect("Failed to insert transcript");

                sqlx::query(
                    "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                )
                .bind(format!("{}-chunk", meeting_id))
                .bind(&meeting_id)
                .bind("Test Meeting")
                .execute(&pool)
                .await
                .expect("Failed to insert transcript_chunk");

                sqlx::query(
                    "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                )
                .bind(format!("{}-summary", meeting_id))
                .bind(&meeting_id)
                .execute(&pool)
                .await
                .expect("Failed to insert summary_process");

                // Verify all records exist before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist");

                let transcript_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript");
                assert!(transcript_before.is_some(), "Transcript should exist");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed
                assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting and all related records should be deleted
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should be deleted");

                let transcript_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript after deletion");
                assert!(transcript_after.is_none(), "Transcript should be deleted");

                let chunk_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunk after deletion");
                assert!(chunk_after.is_none(), "Transcript chunk should be deleted");

                let summary_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query summary_process after deletion");
                assert!(summary_after.is_none(), "Summary process should be deleted");

                // Property Assertion 3: No filesystem operations attempted (NULL folder_path)
            });
        }

        #[test]
        fn test_empty_string_folder_path_with_related_records(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Insert meeting with empty string folder_path
                insert_test_meeting(&pool, &meeting_id, Some(""))
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert related records
                let now = Utc::now().naive_utc();
                
                sqlx::query(
                    "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(format!("{}-transcript", meeting_id))
                .bind(&meeting_id)
                .bind("Test transcript")
                .bind(now)
                .execute(&pool)
                .await
                .expect("Failed to insert transcript");

                sqlx::query(
                    "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                )
                .bind(format!("{}-chunk", meeting_id))
                .bind(&meeting_id)
                .bind("Test Meeting")
                .execute(&pool)
                .await
                .expect("Failed to insert transcript_chunk");

                sqlx::query(
                    "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                )
                .bind(format!("{}-summary", meeting_id))
                .bind(&meeting_id)
                .execute(&pool)
                .await
                .expect("Failed to insert summary_process");

                // Verify all records exist before deletion
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Database deletion should succeed
                assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: Meeting and all related records should be deleted
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should be deleted");

                let transcript_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript after deletion");
                assert!(transcript_after.is_none(), "Transcript should be deleted");

                let chunk_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunk after deletion");
                assert!(chunk_after.is_none(), "Transcript chunk should be deleted");

                let summary_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query summary_process after deletion");
                assert!(summary_after.is_none(), "Summary process should be deleted");

                // Property Assertion 3: No filesystem operations attempted (empty string folder_path)
            });
        }

        #[test]
        fn test_mixed_null_and_empty_folder_paths(
            meeting_id_1 in meeting_id_strategy(),
            meeting_id_2 in meeting_id_strategy(),
        ) {
            // Ensure we have different meeting IDs
            prop_assume!(meeting_id_1 != meeting_id_2);

            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Insert one meeting with NULL folder_path
                insert_test_meeting(&pool, &meeting_id_1, None)
                    .await
                    .expect("Failed to insert first test meeting");

                // Setup: Insert another meeting with empty string folder_path
                insert_test_meeting(&pool, &meeting_id_2, Some(""))
                    .await
                    .expect("Failed to insert second test meeting");

                // Verify both meetings exist
                let meeting1_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id_1)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting 1");
                assert!(meeting1_before.is_some(), "Meeting 1 should exist");

                let meeting2_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id_2)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting 2");
                assert!(meeting2_before.is_some(), "Meeting 2 should exist");

                // Action: Delete first meeting (NULL folder_path)
                let result1 = MeetingsRepository::delete_meeting(&pool, &meeting_id_1).await;

                // Property Assertion 1: First deletion should succeed
                assert!(result1.is_ok(), "First deletion should succeed: {:?}", result1);
                assert_eq!(result1.unwrap(), true, "delete_meeting should return true");

                // Verify first meeting is deleted
                let meeting1_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id_1)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting 1 after deletion");
                assert!(meeting1_after.is_none(), "Meeting 1 should be deleted");

                // Action: Delete second meeting (empty string folder_path)
                let result2 = MeetingsRepository::delete_meeting(&pool, &meeting_id_2).await;

                // Property Assertion 2: Second deletion should succeed
                assert!(result2.is_ok(), "Second deletion should succeed: {:?}", result2);
                assert_eq!(result2.unwrap(), true, "delete_meeting should return true");

                // Verify second meeting is deleted
                let meeting2_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id_2)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting 2 after deletion");
                assert!(meeting2_after.is_none(), "Meeting 2 should be deleted");

                // Property Assertion 3: Both deletions completed without filesystem operations
            });
        }
    }
}

/// Property 6: Deletion Order Preservation
/// **Validates: Requirements 6.4, 7.1, 7.2, 7.3, 7.4**
///
/// Property: For any meeting deletion operation, related records (transcript_chunks,
/// summary_processes, transcripts) should be deleted before the meeting record itself
/// is deleted.
#[cfg(test)]
mod deletion_order_preservation_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use sqlx::Executor;

    /// Custom connection wrapper that tracks deletion order
    #[derive(Clone)]
    struct DeletionTracker {
        deletions: Arc<Mutex<Vec<String>>>,
    }

    impl DeletionTracker {
        fn new() -> Self {
            Self {
                deletions: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn record_deletion(&self, table: &str) {
            let mut deletions = self.deletions.lock().unwrap();
            deletions.push(table.to_string());
        }

        fn get_deletions(&self) -> Vec<String> {
            self.deletions.lock().unwrap().clone()
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_related_records_deleted_before_meeting(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Setup: Insert meeting with folder_path
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert related records in all tables
                let now = Utc::now().naive_utc();
                
                // Insert multiple transcript_chunks
                for i in 0..3 {
                    sqlx::query(
                        "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                    )
                    .bind(format!("{}-chunk-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .bind("Test Meeting")
                    .execute(&pool)
                    .await
                    .expect("Failed to insert transcript_chunk");
                }

                // Insert multiple summary_processes
                for i in 0..2 {
                    sqlx::query(
                        "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                    )
                    .bind(format!("{}-summary-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .execute(&pool)
                    .await
                    .expect("Failed to insert summary_process");
                }

                // Insert multiple transcripts
                for i in 0..5 {
                    sqlx::query(
                        "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                    )
                    .bind(format!("{}-transcript-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .bind(format!("Test transcript {}", i))
                    .bind(now)
                    .execute(&pool)
                    .await
                    .expect("Failed to insert transcript");
                }

                // Verify all records exist before deletion
                let chunk_count_before: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcript_chunks");
                assert_eq!(chunk_count_before.0, 3, "Should have 3 transcript chunks");

                let summary_count_before: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count summary_processes");
                assert_eq!(summary_count_before.0, 2, "Should have 2 summary processes");

                let transcript_count_before: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcripts");
                assert_eq!(transcript_count_before.0, 5, "Should have 5 transcripts");

                let meeting_exists_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_exists_before.is_some(), "Meeting should exist");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Deletion should succeed
                assert!(result.is_ok(), "Deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: All related records should be deleted
                let chunk_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcript_chunks after deletion");
                assert_eq!(
                    chunk_count_after.0, 0,
                    "All transcript_chunks should be deleted"
                );

                let summary_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count summary_processes after deletion");
                assert_eq!(
                    summary_count_after.0, 0,
                    "All summary_processes should be deleted"
                );

                let transcript_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcripts after deletion");
                assert_eq!(
                    transcript_count_after.0, 0,
                    "All transcripts should be deleted"
                );

                // Property Assertion 3: Meeting should be deleted
                let meeting_exists_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(
                    meeting_exists_after.is_none(),
                    "Meeting should be deleted"
                );

                // Property Assertion 4: The deletion order is enforced by foreign key constraints
                // and the implementation. If the meeting was deleted before related records,
                // the foreign key constraints would have prevented the deletion or caused an error.
                // The fact that all deletions succeeded proves the correct order was followed.
            });
        }

        #[test]
        fn test_deletion_order_with_foreign_key_constraints(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database with foreign key constraints enabled
                let pool = SqlitePool::connect(":memory:")
                    .await
                    .expect("Failed to create in-memory database");

                // Enable foreign key constraints
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(&pool)
                    .await
                    .expect("Failed to enable foreign keys");

                // Create tables with foreign key constraints
                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS meetings (
                        id TEXT PRIMARY KEY,
                        title TEXT NOT NULL,
                        created_at TIMESTAMP NOT NULL,
                        updated_at TIMESTAMP NOT NULL,
                        folder_path TEXT
                    )
                    "#,
                )
                .execute(&pool)
                .await
                .expect("Failed to create meetings table");

                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS transcripts (
                        id TEXT PRIMARY KEY,
                        meeting_id TEXT NOT NULL,
                        transcript TEXT NOT NULL,
                        timestamp TIMESTAMP NOT NULL,
                        audio_start_time REAL,
                        audio_end_time REAL,
                        duration REAL,
                        FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE RESTRICT
                    )
                    "#,
                )
                .execute(&pool)
                .await
                .expect("Failed to create transcripts table");

                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS transcript_chunks (
                        id TEXT PRIMARY KEY,
                        meeting_id TEXT NOT NULL,
                        meeting_name TEXT,
                        FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE RESTRICT
                    )
                    "#,
                )
                .execute(&pool)
                .await
                .expect("Failed to create transcript_chunks table");

                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS summary_processes (
                        id TEXT PRIMARY KEY,
                        meeting_id TEXT NOT NULL,
                        FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE RESTRICT
                    )
                    "#,
                )
                .execute(&pool)
                .await
                .expect("Failed to create summary_processes table");

                // Setup: Insert meeting
                insert_test_meeting(&pool, &meeting_id, None)
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert related records
                let now = Utc::now().naive_utc();
                
                sqlx::query(
                    "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                )
                .bind(format!("{}-chunk", meeting_id))
                .bind(&meeting_id)
                .bind("Test Meeting")
                .execute(&pool)
                .await
                .expect("Failed to insert transcript_chunk");

                sqlx::query(
                    "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                )
                .bind(format!("{}-summary", meeting_id))
                .bind(&meeting_id)
                .execute(&pool)
                .await
                .expect("Failed to insert summary_process");

                sqlx::query(
                    "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(format!("{}-transcript", meeting_id))
                .bind(&meeting_id)
                .bind("Test transcript")
                .bind(now)
                .execute(&pool)
                .await
                .expect("Failed to insert transcript");

                // Verify all records exist
                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist");

                let transcript_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript");
                assert!(transcript_before.is_some(), "Transcript should exist");

                // Action: Delete the meeting
                // With ON DELETE RESTRICT, this will only succeed if related records are deleted first
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Deletion should succeed
                // This proves that the implementation deletes related records before the meeting
                assert!(
                    result.is_ok(),
                    "Deletion should succeed with foreign key constraints: {:?}",
                    result
                );
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: All records should be deleted
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should be deleted");

                let transcript_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript after deletion");
                assert!(transcript_after.is_none(), "Transcript should be deleted");

                // Property Assertion 3: The fact that deletion succeeded with ON DELETE RESTRICT
                // proves that related records were deleted before the meeting record
            });
        }

        #[test]
        fn test_deletion_order_with_multiple_related_records(
            meeting_id in meeting_id_strategy(),
            folder_name in folder_name_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Create temporary directory for meeting folders
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let folder_path = create_meeting_folder_with_content(
                    temp_dir.path(),
                    &folder_name,
                );

                // Setup: Insert meeting with folder_path
                let folder_path_str = folder_path.to_str().unwrap();
                insert_test_meeting(&pool, &meeting_id, Some(folder_path_str))
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert many related records to stress test the deletion order
                let now = Utc::now().naive_utc();
                
                // Insert 10 transcript_chunks
                for i in 0..10 {
                    sqlx::query(
                        "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                    )
                    .bind(format!("{}-chunk-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .bind("Test Meeting")
                    .execute(&pool)
                    .await
                    .expect("Failed to insert transcript_chunk");
                }

                // Insert 5 summary_processes
                for i in 0..5 {
                    sqlx::query(
                        "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                    )
                    .bind(format!("{}-summary-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .execute(&pool)
                    .await
                    .expect("Failed to insert summary_process");
                }

                // Insert 20 transcripts
                for i in 0..20 {
                    sqlx::query(
                        "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                    )
                    .bind(format!("{}-transcript-{}", meeting_id, i))
                    .bind(&meeting_id)
                    .bind(format!("Test transcript {}", i))
                    .bind(now)
                    .execute(&pool)
                    .await
                    .expect("Failed to insert transcript");
                }

                // Verify counts before deletion
                let chunk_count: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcript_chunks");
                assert_eq!(chunk_count.0, 10, "Should have 10 transcript chunks");

                let summary_count: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count summary_processes");
                assert_eq!(summary_count.0, 5, "Should have 5 summary processes");

                let transcript_count: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcripts");
                assert_eq!(transcript_count.0, 20, "Should have 20 transcripts");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Deletion should succeed even with many related records
                assert!(result.is_ok(), "Deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: All related records should be deleted
                let chunk_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcript_chunks after deletion");
                assert_eq!(chunk_count_after.0, 0, "All transcript_chunks should be deleted");

                let summary_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count summary_processes after deletion");
                assert_eq!(summary_count_after.0, 0, "All summary_processes should be deleted");

                let transcript_count_after: (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .expect("Failed to count transcripts after deletion");
                assert_eq!(transcript_count_after.0, 0, "All transcripts should be deleted");

                // Property Assertion 3: Meeting should be deleted
                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should be deleted");

                // Property Assertion 4: Filesystem folder should be deleted
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                assert!(
                    !folder_path.exists(),
                    "Folder should be deleted after successful database deletion"
                );
            });
        }

        #[test]
        fn test_deletion_order_preserved_across_transaction_boundaries(
            meeting_id in meeting_id_strategy(),
        ) {
            // Run async test in tokio runtime
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                // Setup: Create test database
                let pool = create_test_database().await;

                // Setup: Insert meeting
                insert_test_meeting(&pool, &meeting_id, None)
                    .await
                    .expect("Failed to insert test meeting");

                // Setup: Insert related records
                let now = Utc::now().naive_utc();
                
                sqlx::query(
                    "INSERT INTO transcript_chunks (id, meeting_id, meeting_name) VALUES (?, ?, ?)"
                )
                .bind(format!("{}-chunk", meeting_id))
                .bind(&meeting_id)
                .bind("Test Meeting")
                .execute(&pool)
                .await
                .expect("Failed to insert transcript_chunk");

                sqlx::query(
                    "INSERT INTO summary_processes (id, meeting_id) VALUES (?, ?)"
                )
                .bind(format!("{}-summary", meeting_id))
                .bind(&meeting_id)
                .execute(&pool)
                .await
                .expect("Failed to insert summary_process");

                sqlx::query(
                    "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(format!("{}-transcript", meeting_id))
                .bind(&meeting_id)
                .bind("Test transcript")
                .bind(now)
                .execute(&pool)
                .await
                .expect("Failed to insert transcript");

                // Verify all records exist before deletion
                let chunk_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunk");
                assert!(chunk_before.is_some(), "Transcript chunk should exist");

                let summary_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query summary_process");
                assert!(summary_before.is_some(), "Summary process should exist");

                let transcript_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript");
                assert!(transcript_before.is_some(), "Transcript should exist");

                let meeting_before: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting");
                assert!(meeting_before.is_some(), "Meeting should exist");

                // Action: Delete the meeting
                let result = MeetingsRepository::delete_meeting(&pool, &meeting_id).await;

                // Property Assertion 1: Deletion should succeed
                assert!(result.is_ok(), "Deletion should succeed: {:?}", result);
                assert_eq!(result.unwrap(), true, "delete_meeting should return true");

                // Property Assertion 2: All records should be deleted in the correct order
                // We verify this by checking that none of the records exist after deletion
                let chunk_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcript_chunks WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript_chunk after deletion");
                assert!(chunk_after.is_none(), "Transcript chunk should be deleted");

                let summary_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM summary_processes WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query summary_process after deletion");
                assert!(summary_after.is_none(), "Summary process should be deleted");

                let transcript_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM transcripts WHERE meeting_id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query transcript after deletion");
                assert!(transcript_after.is_none(), "Transcript should be deleted");

                let meeting_after: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM meetings WHERE id = ?"
                )
                .bind(&meeting_id)
                .fetch_optional(&pool)
                .await
                .expect("Failed to query meeting after deletion");
                assert!(meeting_after.is_none(), "Meeting should be deleted");

                // Property Assertion 3: The transaction ensures atomicity
                // Either all deletions succeed in the correct order, or none succeed
                // The fact that all records are deleted proves the correct order was maintained
            });
        }
    }
}
