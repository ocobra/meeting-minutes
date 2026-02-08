//! Unit tests for meeting folder deletion edge cases
//!
//! This test suite validates specific edge cases for the folder deletion functionality,
//! ensuring proper handling of non-existent folders, empty folders, and folders with
//! nested content.
//!
//! **Validates: Requirements 2.3**

use sqlx::SqlitePool;
use std::fs;
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

#[cfg(test)]
mod folder_deletion_edge_cases {
    use super::*;

    /// Test: Folder doesn't exist (should log warning, not error)
    /// 
    /// This test verifies that when a meeting has a folder_path pointing to a
    /// non-existent location, the deletion operation succeeds and logs a warning
    /// rather than failing with an error.
    #[tokio::test]
    async fn test_delete_meeting_with_nonexistent_folder() {
        // Setup: Create test database
        let pool = create_test_database().await;

        // Setup: Create a path that doesn't exist
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let nonexistent_path = temp_dir.path().join("nonexistent_folder");
        
        // Verify the folder doesn't exist
        assert!(!nonexistent_path.exists(), "Folder should not exist before test");

        // Setup: Insert meeting with non-existent folder_path
        let meeting_id = "test-meeting-nonexistent";
        let folder_path_str = nonexistent_path.to_str().unwrap();
        insert_test_meeting(&pool, meeting_id, Some(folder_path_str))
            .await
            .expect("Failed to insert test meeting");

        // Verify meeting exists in database
        let meeting_exists: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM meetings WHERE id = ?"
        )
        .bind(meeting_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query meeting");
        assert!(meeting_exists.is_some(), "Meeting should exist in database before deletion");

        // Action: Delete the meeting
        let result = MeetingsRepository::delete_meeting(&pool, meeting_id).await;

        // Assertion 1: Database deletion should succeed despite non-existent folder
        assert!(result.is_ok(), "Database deletion should succeed even when folder doesn't exist: {:?}", result);
        assert_eq!(result.unwrap(), true, "delete_meeting should return true");

        // Assertion 2: Meeting should be removed from database
        let meeting_after: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM meetings WHERE id = ?"
        )
        .bind(meeting_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query meeting after deletion");
        assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");

        // Assertion 3: Folder still doesn't exist (no error occurred)
        assert!(!nonexistent_path.exists(), "Folder should still not exist after deletion");
    }

    /// Test: Empty folder deletion
    /// 
    /// This test verifies that an empty meeting folder is successfully deleted
    /// when the meeting is removed from the database.
    #[tokio::test]
    async fn test_delete_meeting_with_empty_folder() {
        // Setup: Create test database
        let pool = create_test_database().await;

        // Setup: Create an empty folder
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let empty_folder = temp_dir.path().join("empty_meeting_folder");
        fs::create_dir_all(&empty_folder).expect("Failed to create empty folder");

        // Verify folder exists and is empty
        assert!(empty_folder.exists(), "Empty folder should exist before deletion");
        let entries: Vec<_> = fs::read_dir(&empty_folder)
            .expect("Failed to read directory")
            .collect();
        assert_eq!(entries.len(), 0, "Folder should be empty");

        // Setup: Insert meeting with empty folder_path
        let meeting_id = "test-meeting-empty-folder";
        let folder_path_str = empty_folder.to_str().unwrap();
        insert_test_meeting(&pool, meeting_id, Some(folder_path_str))
            .await
            .expect("Failed to insert test meeting");

        // Action: Delete the meeting
        let result = MeetingsRepository::delete_meeting(&pool, meeting_id).await;

        // Assertion 1: Database deletion should succeed
        assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
        assert_eq!(result.unwrap(), true, "delete_meeting should return true");

        // Assertion 2: Meeting should be removed from database
        let meeting_after: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM meetings WHERE id = ?"
        )
        .bind(meeting_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query meeting after deletion");
        assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");

        // Assertion 3: Empty folder should be deleted from filesystem
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(!empty_folder.exists(), "Empty folder should be deleted from filesystem");
    }

    /// Test: Delete meeting with empty string folder_path
    /// 
    /// This test verifies that when a meeting has an empty string as folder_path,
    /// the deletion operation succeeds and skips filesystem operations, treating
    /// empty string the same as NULL.
    #[tokio::test]
    async fn test_delete_meeting_with_empty_string_folder_path() {
        // Setup: Create test database
        let pool = create_test_database().await;

        // Setup: Insert meeting with empty string folder_path
        let meeting_id = "test-meeting-empty-string";
        insert_test_meeting(&pool, meeting_id, Some(""))
            .await
            .expect("Failed to insert test meeting");

        // Verify meeting exists in database
        let meeting_exists: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM meetings WHERE id = ?"
        )
        .bind(meeting_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query meeting");
        assert!(meeting_exists.is_some(), "Meeting should exist in database before deletion");

        // Action: Delete the meeting
        let result = MeetingsRepository::delete_meeting(&pool, meeting_id).await;

        // Assertion 1: Database deletion should succeed
        assert!(result.is_ok(), "Database deletion should succeed with empty string folder_path: {:?}", result);
        assert_eq!(result.unwrap(), true, "delete_meeting should return true");

        // Assertion 2: Meeting should be removed from database
        let meeting_after: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM meetings WHERE id = ?"
        )
        .bind(meeting_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query meeting after deletion");
        assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");

        // Assertion 3: No filesystem operations should have been attempted
        // (This is implicitly tested by the fact that no errors occurred and
        // the operation completed successfully without any folder path to work with)
    }

    /// Test: Folder with nested subdirectories and files
    /// 
    /// This test verifies that a meeting folder containing nested subdirectories
    /// and multiple files is completely and recursively deleted when the meeting
    /// is removed from the database.
    #[tokio::test]
    async fn test_delete_meeting_with_nested_content() {
        // Setup: Create test database
        let pool = create_test_database().await;

        // Setup: Create a folder with nested subdirectories and files
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let meeting_folder = temp_dir.path().join("nested_meeting_folder");
        fs::create_dir_all(&meeting_folder).expect("Failed to create meeting folder");

        // Create files at root level
        fs::write(meeting_folder.join("audio.mp4"), b"fake audio data")
            .expect("Failed to create audio.mp4");
        fs::write(meeting_folder.join("metadata.json"), b"{\"test\": true}")
            .expect("Failed to create metadata.json");

        // Create first level subdirectory with files
        let transcripts_dir = meeting_folder.join("transcripts");
        fs::create_dir_all(&transcripts_dir).expect("Failed to create transcripts dir");
        fs::write(transcripts_dir.join("transcript1.txt"), b"transcript 1")
            .expect("Failed to create transcript1.txt");
        fs::write(transcripts_dir.join("transcript2.txt"), b"transcript 2")
            .expect("Failed to create transcript2.txt");

        // Create second level nested subdirectory with files
        let chunks_dir = transcripts_dir.join("chunks");
        fs::create_dir_all(&chunks_dir).expect("Failed to create chunks dir");
        fs::write(chunks_dir.join("chunk1.json"), b"{\"chunk\": 1}")
            .expect("Failed to create chunk1.json");
        fs::write(chunks_dir.join("chunk2.json"), b"{\"chunk\": 2}")
            .expect("Failed to create chunk2.json");

        // Create another top-level subdirectory
        let summaries_dir = meeting_folder.join("summaries");
        fs::create_dir_all(&summaries_dir).expect("Failed to create summaries dir");
        fs::write(summaries_dir.join("summary.txt"), b"meeting summary")
            .expect("Failed to create summary.txt");

        // Verify all files and directories exist
        assert!(meeting_folder.exists(), "Meeting folder should exist");
        assert!(meeting_folder.join("audio.mp4").exists(), "audio.mp4 should exist");
        assert!(meeting_folder.join("metadata.json").exists(), "metadata.json should exist");
        assert!(transcripts_dir.exists(), "transcripts dir should exist");
        assert!(transcripts_dir.join("transcript1.txt").exists(), "transcript1.txt should exist");
        assert!(transcripts_dir.join("transcript2.txt").exists(), "transcript2.txt should exist");
        assert!(chunks_dir.exists(), "chunks dir should exist");
        assert!(chunks_dir.join("chunk1.json").exists(), "chunk1.json should exist");
        assert!(chunks_dir.join("chunk2.json").exists(), "chunk2.json should exist");
        assert!(summaries_dir.exists(), "summaries dir should exist");
        assert!(summaries_dir.join("summary.txt").exists(), "summary.txt should exist");

        // Setup: Insert meeting with nested folder_path
        let meeting_id = "test-meeting-nested-content";
        let folder_path_str = meeting_folder.to_str().unwrap();
        insert_test_meeting(&pool, meeting_id, Some(folder_path_str))
            .await
            .expect("Failed to insert test meeting");

        // Action: Delete the meeting
        let result = MeetingsRepository::delete_meeting(&pool, meeting_id).await;

        // Assertion 1: Database deletion should succeed
        assert!(result.is_ok(), "Database deletion should succeed: {:?}", result);
        assert_eq!(result.unwrap(), true, "delete_meeting should return true");

        // Assertion 2: Meeting should be removed from database
        let meeting_after: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM meetings WHERE id = ?"
        )
        .bind(meeting_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query meeting after deletion");
        assert!(meeting_after.is_none(), "Meeting should not exist in database after deletion");

        // Assertion 3: Entire folder tree should be deleted from filesystem
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(!meeting_folder.exists(), "Meeting folder should be completely deleted from filesystem");
        
        // Verify nested content is also gone (if folder somehow still exists)
        if meeting_folder.exists() {
            panic!("Meeting folder still exists after deletion: {}", meeting_folder.display());
        }
    }
}
