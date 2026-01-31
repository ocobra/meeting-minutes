#[cfg(test)]
mod checkpoint_creation_tests {
    use crate::audio::audio_processing::create_meeting_folder;
    use crate::audio::incremental_saver::IncrementalAudioSaver;
    use tempfile::tempdir;

    #[test]
    fn test_checkpoint_directory_creation_success() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let base_path = temp_dir.path().to_path_buf();

        // Test creating meeting folder with checkpoints
        let meeting_folder = create_meeting_folder(&base_path, "Test Meeting", true)
            .expect("Failed to create meeting folder");

        // Verify main folder exists
        assert!(meeting_folder.exists(), "Meeting folder should exist");

        // Verify checkpoints directory exists
        let checkpoints_dir = meeting_folder.join(".checkpoints");
        assert!(checkpoints_dir.exists(), "Checkpoints directory should exist");

        // Verify we can create an IncrementalAudioSaver
        let saver = IncrementalAudioSaver::new(meeting_folder, 48000);
        assert!(saver.is_ok(), "IncrementalAudioSaver should initialize successfully");
    }

    #[test]
    fn test_checkpoint_directory_creation_without_checkpoints() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let base_path = temp_dir.path().to_path_buf();

        // Test creating meeting folder without checkpoints
        let meeting_folder = create_meeting_folder(&base_path, "Test Meeting", false)
            .expect("Failed to create meeting folder");

        // Verify main folder exists
        assert!(meeting_folder.exists(), "Meeting folder should exist");

        // Verify checkpoints directory does NOT exist
        let checkpoints_dir = meeting_folder.join(".checkpoints");
        assert!(!checkpoints_dir.exists(), "Checkpoints directory should not exist when create_checkpoints_dir=false");
    }

    #[test]
    fn test_incremental_saver_creates_missing_directory() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let meeting_folder = temp_dir.path().join("test_meeting");
        std::fs::create_dir_all(&meeting_folder).expect("Failed to create meeting folder");

        // Don't create .checkpoints directory initially
        let checkpoints_dir = meeting_folder.join(".checkpoints");
        assert!(!checkpoints_dir.exists(), "Checkpoints directory should not exist initially");

        // IncrementalAudioSaver should create the directory as fallback
        let saver = IncrementalAudioSaver::new(meeting_folder, 48000);
        assert!(saver.is_ok(), "IncrementalAudioSaver should create missing directory and initialize successfully");

        // Verify directory was created
        assert!(checkpoints_dir.exists(), "Checkpoints directory should be created by IncrementalAudioSaver");
    }

    #[test]
    fn test_permission_error_handling() {
        // This test is platform-specific and may not work on all systems
        // It's mainly for documentation of expected behavior
        
        // Create a temporary directory for testing
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let base_path = temp_dir.path().to_path_buf();

        // Create a meeting folder first
        let meeting_folder = create_meeting_folder(&base_path, "Permission Test", false)
            .expect("Failed to create meeting folder");

        // Try to create IncrementalAudioSaver without checkpoints directory
        // This should fail gracefully with a clear error message
        let saver_result = IncrementalAudioSaver::new(meeting_folder, 48000);
        
        // The saver should either succeed (by creating the directory) or fail with a clear error
        match saver_result {
            Ok(_) => {
                // Success - the saver created the directory as fallback
                println!("✅ IncrementalAudioSaver successfully created missing directory");
            }
            Err(e) => {
                // Failure - should have a clear error message
                let error_msg = format!("{}", e);
                assert!(
                    error_msg.contains("checkpoints") || error_msg.contains("directory"),
                    "Error message should mention checkpoints or directory: {}",
                    error_msg
                );
                println!("✅ IncrementalAudioSaver failed with clear error: {}", error_msg);
            }
        }
    }
}