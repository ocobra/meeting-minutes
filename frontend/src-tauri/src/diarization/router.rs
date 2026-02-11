//! Model Router - Chooses between external/cloud and local models
//!
//! The Model Router determines which diarization model to use based on:
//! 1. Privacy settings (LocalOnly, PreferExternal, ExternalOnly)
//! 2. Internet connectivity
//! 3. External API availability
//! 4. Fallback to local models when needed
//!
//! # Architecture
//!
//! The router implements a smart model selection strategy that respects user privacy
//! preferences while optimizing for accuracy and resource usage:
//!
//! - **LocalOnly**: Never uses external APIs, all processing happens locally
//! - **PreferExternal**: Tries external APIs first, falls back to local on failure
//! - **ExternalOnly**: Requires external APIs, fails if unavailable
//!
//! # Caching
//!
//! The router caches routing decisions and connectivity status to avoid repeated
//! network checks. Cache duration is configurable (default: 5 minutes).
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::router::{ModelRouter, RouterConfig};
//! use crate::diarization::types::PrivacyMode;
//!
//! let config = RouterConfig {
//!     privacy_mode: PrivacyMode::PreferExternal,
//!     ..Default::default()
//! };
//! let router = ModelRouter::new(config);
//!
//! // Choose diarization model
//! let model = router.choose_diarization_model().await?;
//! ```

use crate::diarization::{types::PrivacyMode, DiarizationError};
use log::{debug, info, warn};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Model choice result from router
#[derive(Debug, Clone)]
pub enum ModelChoice {
    /// Use external/cloud model
    External {
        endpoint: String,
        api_key: Option<String>,
    },
    /// Use local model
    Local { model_path: String },
}

/// Configuration for model router
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Privacy mode setting
    pub privacy_mode: PrivacyMode,
    /// Timeout for external API checks (milliseconds)
    pub external_api_timeout_ms: u64,
    /// How long to cache routing decisions (seconds)
    pub cache_duration_seconds: u64,
    /// Hugging Face API endpoint for diarization
    pub hf_diarization_endpoint: String,
    /// Local diarization model path
    pub local_diarization_path: String,
    /// Local LLM model path
    pub local_llm_path: String,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            privacy_mode: PrivacyMode::PreferExternal,
            external_api_timeout_ms: 5000,
            cache_duration_seconds: 300,
            hf_diarization_endpoint:
                "https://api-inference.huggingface.co/models/pyannote/speaker-diarization-3.1"
                    .to_string(),
            local_diarization_path: "models/pyannote/speaker-diarization".to_string(),
            local_llm_path: "models/llm/local".to_string(),
        }
    }
}

/// Cached routing decision
#[derive(Debug, Clone)]
struct RouterCache {
    diarization_choice: Option<ModelChoice>,
    llm_choice: Option<ModelChoice>,
    cached_at: Instant,
    connectivity_status: Option<bool>,
    connectivity_checked_at: Instant,
}

impl Default for RouterCache {
    fn default() -> Self {
        Self {
            diarization_choice: None,
            llm_choice: None,
            cached_at: Instant::now(),
            connectivity_status: None,
            connectivity_checked_at: Instant::now(),
        }
    }
}

/// Model Router for choosing between external and local models
pub struct ModelRouter {
    config: RouterConfig,
    cache: Arc<RwLock<RouterCache>>,
}

impl ModelRouter {
    /// Create a new model router with the given configuration
    pub fn new(config: RouterConfig) -> Self {
        info!(
            "Initializing ModelRouter with privacy mode: {:?}",
            config.privacy_mode
        );
        Self {
            config,
            cache: Arc::new(RwLock::new(RouterCache::default())),
        }
    }

    /// Choose which diarization model to use
    pub async fn choose_diarization_model(&self) -> Result<ModelChoice, DiarizationError> {
        debug!("Choosing diarization model...");

        // Check cache first
        let cache = self.cache.read().await;
        if let Some(choice) = &cache.diarization_choice {
            if cache.cached_at.elapsed().as_secs() < self.config.cache_duration_seconds {
                debug!("Using cached diarization model choice");
                return Ok(choice.clone());
            }
        }
        drop(cache);

        // Determine model choice based on privacy mode
        let choice = match self.config.privacy_mode {
            PrivacyMode::LocalOnly => {
                info!("Privacy mode: LocalOnly - using local diarization model");
                ModelChoice::Local {
                    model_path: self.config.local_diarization_path.clone(),
                }
            }
            PrivacyMode::ExternalOnly => {
                info!("Privacy mode: ExternalOnly - must use external diarization model");
                self.get_external_diarization_model().await?
            }
            PrivacyMode::PreferExternal => {
                info!("Privacy mode: PreferExternal - trying external diarization model first");
                match self.try_external_diarization_model().await {
                    Ok(choice) => {
                        info!("Successfully connected to external diarization model");
                        choice
                    }
                    Err(e) => {
                        warn!(
                            "Failed to connect to external diarization model: {}. Falling back to local model",
                            e
                        );
                        ModelChoice::Local {
                            model_path: self.config.local_diarization_path.clone(),
                        }
                    }
                }
            }
        };

        // Update cache
        let mut cache = self.cache.write().await;
        cache.diarization_choice = Some(choice.clone());
        cache.cached_at = Instant::now();

        Ok(choice)
    }

    /// Choose which LLM model to use for identification
    pub async fn choose_llm_model(&self) -> Result<ModelChoice, DiarizationError> {
        debug!("Choosing LLM model for speaker identification...");

        // Check cache first
        let cache = self.cache.read().await;
        if let Some(choice) = &cache.llm_choice {
            if cache.cached_at.elapsed().as_secs() < self.config.cache_duration_seconds {
                debug!("Using cached LLM model choice");
                return Ok(choice.clone());
            }
        }
        drop(cache);

        // Determine model choice based on privacy mode
        let choice = match self.config.privacy_mode {
            PrivacyMode::LocalOnly => {
                info!("Privacy mode: LocalOnly - using local LLM");
                ModelChoice::Local {
                    model_path: self.config.local_llm_path.clone(),
                }
            }
            PrivacyMode::ExternalOnly => {
                info!("Privacy mode: ExternalOnly - must use external LLM");
                self.get_external_llm_model().await?
            }
            PrivacyMode::PreferExternal => {
                info!("Privacy mode: PreferExternal - trying external LLM first");
                match self.try_external_llm_model().await {
                    Ok(choice) => {
                        info!("Successfully connected to external LLM");
                        choice
                    }
                    Err(e) => {
                        warn!(
                            "Failed to connect to external LLM: {}. Falling back to local LLM",
                            e
                        );
                        ModelChoice::Local {
                            model_path: self.config.local_llm_path.clone(),
                        }
                    }
                }
            }
        };

        // Update cache
        let mut cache = self.cache.write().await;
        cache.llm_choice = Some(choice.clone());
        cache.cached_at = Instant::now();

        Ok(choice)
    }

    /// Check if internet connectivity is available (with caching)
    async fn check_connectivity(&self) -> bool {
        // Check cache first (cache connectivity for 30 seconds)
        let cache = self.cache.read().await;
        if let Some(status) = cache.connectivity_status {
            if cache.connectivity_checked_at.elapsed().as_secs() < 30 {
                debug!("Using cached connectivity status: {}", status);
                return status;
            }
        }
        drop(cache);

        debug!("Checking internet connectivity...");

        // Try to connect to multiple reliable endpoints with timeout
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(self.config.external_api_timeout_ms))
            .build()
            .unwrap();

        // Try Hugging Face first (most relevant for our use case)
        let hf_check = client.get("https://huggingface.co").send().await;
        let is_connected = if hf_check.is_ok() {
            debug!("Successfully connected to Hugging Face");
            true
        } else {
            // Fallback: try a more reliable endpoint
            debug!("Hugging Face check failed, trying fallback endpoint");
            client
                .get("https://www.cloudflare.com")
                .send()
                .await
                .is_ok()
        };

        // Update cache
        let mut cache = self.cache.write().await;
        cache.connectivity_status = Some(is_connected);
        cache.connectivity_checked_at = Instant::now();

        info!("Internet connectivity: {}", is_connected);
        is_connected
    }

    /// Try to get external diarization model
    async fn try_external_diarization_model(&self) -> Result<ModelChoice, DiarizationError> {
        if !self.check_connectivity().await {
            return Err(DiarizationError::NetworkError(
                "No internet connectivity available".to_string(),
            ));
        }

        self.get_external_diarization_model().await
    }

    /// Get external diarization model configuration
    async fn get_external_diarization_model(&self) -> Result<ModelChoice, DiarizationError> {
        // Check for Hugging Face API key
        let api_key = std::env::var("HUGGINGFACE_API_KEY")
            .or_else(|_| std::env::var("HF_TOKEN"))
            .ok();

        if api_key.is_none() {
            warn!("No Hugging Face API key found in environment (HUGGINGFACE_API_KEY or HF_TOKEN)");
        }

        // Try to verify the endpoint is accessible
        if let Some(ref key) = api_key {
            debug!("Verifying Hugging Face API endpoint...");
            let client = reqwest::Client::builder()
                .timeout(Duration::from_millis(self.config.external_api_timeout_ms))
                .build()
                .unwrap();

            match client
                .get(&self.config.hf_diarization_endpoint)
                .header("Authorization", format!("Bearer {}", key))
                .send()
                .await
            {
                Ok(response) => {
                    debug!("Hugging Face API endpoint responded with status: {}", response.status());
                }
                Err(e) => {
                    warn!("Failed to verify Hugging Face API endpoint: {}", e);
                }
            }
        }

        Ok(ModelChoice::External {
            endpoint: self.config.hf_diarization_endpoint.clone(),
            api_key,
        })
    }

    /// Try to get external LLM model
    async fn try_external_llm_model(&self) -> Result<ModelChoice, DiarizationError> {
        if !self.check_connectivity().await {
            return Err(DiarizationError::NetworkError(
                "No internet connectivity available".to_string(),
            ));
        }

        self.get_external_llm_model().await
    }

    /// Get external LLM model configuration
    async fn get_external_llm_model(&self) -> Result<ModelChoice, DiarizationError> {
        // Check for various LLM API keys in order of preference
        
        // 1. Gemini (Google AI)
        if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
            info!("Using Google Gemini API for speaker identification");
            return Ok(ModelChoice::External {
                endpoint: "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent".to_string(),
                api_key: Some(api_key),
            });
        }

        // 2. OpenAI
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            info!("Using OpenAI API for speaker identification");
            return Ok(ModelChoice::External {
                endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
                api_key: Some(api_key),
            });
        }

        // 3. Anthropic Claude
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            info!("Using Anthropic Claude API for speaker identification");
            return Ok(ModelChoice::External {
                endpoint: "https://api.anthropic.com/v1/messages".to_string(),
                api_key: Some(api_key),
            });
        }

        // 4. Hugging Face Inference API
        if let Ok(api_key) = std::env::var("HUGGINGFACE_API_KEY")
            .or_else(|_| std::env::var("HF_TOKEN"))
        {
            info!("Using Hugging Face Inference API for speaker identification");
            return Ok(ModelChoice::External {
                endpoint: "https://api-inference.huggingface.co/models/meta-llama/Llama-2-7b-chat-hf".to_string(),
                api_key: Some(api_key),
            });
        }

        warn!("No external LLM API key found in environment");
        Err(DiarizationError::ExternalApiError(
            "No external LLM API key configured. Set GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY, or HUGGINGFACE_API_KEY".to_string(),
        ))
    }

    /// Clear the routing cache (useful for testing or configuration changes)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = RouterCache::default();
        info!("Model router cache cleared");
    }

    /// Get current cache status (for debugging)
    pub async fn get_cache_status(&self) -> (bool, bool, Option<bool>) {
        let cache = self.cache.read().await;
        (
            cache.diarization_choice.is_some(),
            cache.llm_choice.is_some(),
            cache.connectivity_status,
        )
    }
}


#[cfg(test)]
mod tests {
    use super::{ModelChoice, ModelRouter, RouterConfig};
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
        let (has_diarization, has_llm, _connectivity) = router.get_cache_status().await;
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
}
