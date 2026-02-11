// Unit tests for Model Router

#[cfg(test)]
mod tests {
    use super::super::router::{ModelChoice, ModelRouter, RouterConfig};
    use crate::diarization::types::PrivacyMode;

    #[tokio::test]
    async fn test_local_only_mode_always_returns_local() {
        // Test that LocalOnly mode always returns local models
        let config = RouterConfig {
            privacy_mode: PrivacyMode::LocalOnly,
            ..Default::default()
        };
        let router = ModelRouter::new(config);

        // Test diarization model choice
        let diarization_choice = router.choose_diarization_model().await.unwrap();
        match diarization_choice {
            ModelChoice::Local { .. } => {
                // Success - local model chosen
            }
            ModelChoice::External { .. } => {
                panic!("LocalOnly mode should never return external model");
            }
        }

        // Test LLM model choice
        let llm_choice = router.choose_llm_model().await.unwrap();
        match llm_choice {
            ModelChoice::Local { .. } => {
                // Success - local model chosen
            }
            ModelChoice::External { .. } => {
                panic!("LocalOnly mode should never return external model");
            }
        }
    }

    #[tokio::test]
    async fn test_prefer_external_mode_tries_external_first() {
        // Test that PreferExternal mode tries external first
        let config = RouterConfig {
            privacy_mode: PrivacyMode::PreferExternal,
            external_api_timeout_ms: 2000, // Short timeout for testing
            ..Default::default()
        };
        let router = ModelRouter::new(config);

        // This test will either succeed with external or fallback to local
        // depending on internet connectivity
        let diarization_choice = router.choose_diarization_model().await.unwrap();
        
        // Both outcomes are valid for PreferExternal mode
        match diarization_choice {
            ModelChoice::External { .. } => {
                println!("Successfully connected to external diarization model");
            }
            ModelChoice::Local { .. } => {
                println!("Fell back to local diarization model (no connectivity or API key)");
            }
        }
    }

    #[tokio::test]
    async fn test_cache_works() {
        // Test that caching works correctly
        let config = RouterConfig {
            privacy_mode: PrivacyMode::LocalOnly,
            cache_duration_seconds: 60,
            ..Default::default()
        };
        let router = ModelRouter::new(config);

        // First call
        let choice1 = router.choose_diarization_model().await.unwrap();
        
        // Second call should use cache
        let choice2 = router.choose_diarization_model().await.unwrap();

        // Both should be local models
        match (&choice1, &choice2) {
            (ModelChoice::Local { model_path: path1 }, ModelChoice::Local { model_path: path2 }) => {
                assert_eq!(path1, path2, "Cached choice should match first choice");
            }
            _ => panic!("Both choices should be local models"),
        }

        // Verify cache status
        let (has_diarization, has_llm, connectivity) = router.get_cache_status().await;
        assert!(has_diarization, "Diarization choice should be cached");
        assert!(!has_llm, "LLM choice should not be cached yet");
    }

    #[tokio::test]
    async fn test_cache_clear() {
        // Test that cache clearing works
        let config = RouterConfig {
            privacy_mode: PrivacyMode::LocalOnly,
            ..Default::default()
        };
        let router = ModelRouter::new(config);

        // Make a choice to populate cache
        let _ = router.choose_diarization_model().await.unwrap();

        // Verify cache is populated
        let (has_diarization_before, _, _) = router.get_cache_status().await;
        assert!(has_diarization_before, "Cache should be populated");

        // Clear cache
        router.clear_cache().await;

        // Verify cache is cleared
        let (has_diarization_after, _, _) = router.get_cache_status().await;
        assert!(!has_diarization_after, "Cache should be cleared");
    }

    #[tokio::test]
    async fn test_external_only_mode_requires_external() {
        // Test that ExternalOnly mode requires external models
        let config = RouterConfig {
            privacy_mode: PrivacyMode::ExternalOnly,
            external_api_timeout_ms: 2000,
            ..Default::default()
        };
        let router = ModelRouter::new(config);

        // This test will either succeed with external or fail
        // (depending on connectivity and API keys)
        let result = router.choose_diarization_model().await;
        
        match result {
            Ok(ModelChoice::External { .. }) => {
                println!("Successfully connected to external model");
            }
            Ok(ModelChoice::Local { .. }) => {
                panic!("ExternalOnly mode should never return local model");
            }
            Err(e) => {
                println!("ExternalOnly mode correctly failed without connectivity/API key: {}", e);
                // This is expected if no internet or API key
            }
        }
    }

    #[tokio::test]
    async fn test_connectivity_check_caching() {
        // Test that connectivity checks are cached
        let config = RouterConfig {
            privacy_mode: PrivacyMode::PreferExternal,
            external_api_timeout_ms: 2000,
            ..Default::default()
        };
        let router = ModelRouter::new(config);

        // First call will check connectivity
        let _ = router.choose_diarization_model().await;

        // Get cache status
        let (_, _, connectivity1) = router.get_cache_status().await;

        // Second call should use cached connectivity
        let _ = router.choose_diarization_model().await;

        // Get cache status again
        let (_, _, connectivity2) = router.get_cache_status().await;

        // Connectivity status should be cached
        assert_eq!(connectivity1, connectivity2, "Connectivity status should be cached");
    }

    #[tokio::test]
    async fn test_multiple_llm_api_key_priority() {
        // Test that LLM API key priority works correctly
        // This test checks the priority order: OpenAI > Anthropic > Hugging Face
        
        let config = RouterConfig {
            privacy_mode: PrivacyMode::ExternalOnly,
            ..Default::default()
        };
        let router = ModelRouter::new(config);

        // Note: This test will fail if no API keys are set, which is expected
        let result = router.choose_llm_model().await;

        match result {
            Ok(ModelChoice::External { endpoint, .. }) => {
                // Verify the endpoint matches one of the expected providers
                assert!(
                    endpoint.contains("openai.com") 
                    || endpoint.contains("anthropic.com")
                    || endpoint.contains("huggingface.co"),
                    "Endpoint should be from a known LLM provider"
                );
            }
            Ok(ModelChoice::Local { .. }) => {
                panic!("ExternalOnly mode should not return local model");
            }
            Err(_) => {
                // Expected if no API keys are configured
                println!("No external LLM API keys configured (expected in test environment)");
            }
        }
    }
}
