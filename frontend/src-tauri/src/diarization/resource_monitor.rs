//! Resource Monitor - Monitors system resources for diarization
//!
//! This module monitors CPU and memory usage to determine if diarization
//! should be enabled and which processing mode to use.
//!
//! # Architecture
//!
//! The resource monitor provides adaptive processing recommendations based on:
//! - **Available Memory**: Ensures sufficient RAM for model loading
//! - **CPU Usage**: Avoids overloading the system
//! - **Resource Trends**: Monitors resource usage over time
//!
//! # Resource Thresholds
//!
//! **Memory Requirements**:
//! - Minimum: 500 MB free (configurable)
//! - Recommended: 1 GB+ for optimal performance
//! - Batch mode: 2 GB+ for large files
//!
//! **CPU Thresholds**:
//! - Maximum: 80% usage (configurable)
//! - Recommended: < 60% for smooth operation
//!
//! # Processing Mode Recommendations
//!
//! Based on available resources:
//! - **High Resources** (>2GB, <60% CPU): Batch mode with large chunks
//! - **Medium Resources** (1-2GB, 60-80% CPU): RealTime mode with medium chunks
//! - **Low Resources** (<1GB, >80% CPU): RealTime mode with small chunks or disable
//!
//! # Caching
//!
//! Resource checks are cached for 5 seconds (configurable) to avoid
//! excessive system calls and improve performance.
//!
//! # Example
//!
//! ```no_run
//! use crate::diarization::resource_monitor::{ResourceMonitor, ResourceConfig};
//!
//! let config = ResourceConfig {
//!     min_available_memory_mb: 500,
//!     max_cpu_usage_percent: 80.0,
//!     check_interval_ms: 5000,
//! };
//! let monitor = ResourceMonitor::new(config);
//!
//! // Check if resources are available
//! let status = monitor.check_resources()?;
//! if status.has_sufficient_resources {
//!     // Proceed with diarization
//! }
//!
//! // Get processing mode recommendation
//! let mode = monitor.recommend_processing_mode()?;
//!
//! // Estimate resource usage
//! let estimate = monitor.estimate_resource_usage(audio_duration, sample_rate)?;
//! ```

use crate::diarization::{
    types::{ProcessingMode, ResourceEstimate, ResourceStatus},
    DiarizationError,
};
use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};
use std::time::{Duration, Instant};
use std::sync::{Arc, RwLock};

/// Configuration for resource monitoring
#[derive(Debug, Clone)]
pub struct ResourceConfig {
    /// Minimum free memory required (MB)
    pub min_available_memory_mb: u64,
    /// Maximum CPU usage threshold (0.0-100.0)
    pub max_cpu_usage_percent: f32,
    /// How often to check resources (milliseconds)
    pub check_interval_ms: u64,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            min_available_memory_mb: 500,
            max_cpu_usage_percent: 80.0,
            check_interval_ms: 5000,
        }
    }
}

/// Cached resource check result
#[derive(Debug, Clone)]
struct CachedResourceCheck {
    status: ResourceStatus,
    timestamp: Instant,
}

/// Resource monitor for managing diarization resource usage
pub struct ResourceMonitor {
    config: ResourceConfig,
    system: Arc<RwLock<System>>,
    last_check: Arc<RwLock<Option<CachedResourceCheck>>>,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(config: ResourceConfig) -> Self {
        let refresh_kind = RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything());
        
        let mut system = System::new_with_specifics(refresh_kind);
        system.refresh_all();
        
        Self {
            config,
            system: Arc::new(RwLock::new(system)),
            last_check: Arc::new(RwLock::new(None)),
        }
    }

    /// Check current system resources
    pub fn check_resources(&self) -> Result<ResourceStatus, DiarizationError> {
        // Check if we have a recent cached result
        if let Ok(cache) = self.last_check.read() {
            if let Some(cached) = cache.as_ref() {
                let elapsed = cached.timestamp.elapsed();
                if elapsed < Duration::from_millis(self.config.check_interval_ms) {
                    log::debug!("Using cached resource check (age: {:?})", elapsed);
                    return Ok(cached.status.clone());
                }
            }
        }

        // Perform fresh resource check
        let status = self.check_resources_internal()?;

        // Cache the result
        if let Ok(mut cache) = self.last_check.write() {
            *cache = Some(CachedResourceCheck {
                status: status.clone(),
                timestamp: Instant::now(),
            });
        }

        Ok(status)
    }

    /// Internal resource checking implementation
    fn check_resources_internal(&self) -> Result<ResourceStatus, DiarizationError> {
        let mut system = self.system.write()
            .map_err(|e| DiarizationError::ResourceError(format!("Failed to lock system: {}", e)))?;

        // Refresh system information
        system.refresh_cpu_all();
        system.refresh_memory();

        // Get available memory in MB
        let available_memory_mb = system.available_memory() / 1024 / 1024;

        // Calculate average CPU usage across all cores
        let cpu_usage_percent = system.global_cpu_usage();

        log::debug!(
            "Resource check: {} MB available, {:.1}% CPU usage",
            available_memory_mb,
            cpu_usage_percent
        );

        // Determine if diarization can run
        let has_enough_memory = available_memory_mb >= self.config.min_available_memory_mb;
        let cpu_below_threshold = cpu_usage_percent <= self.config.max_cpu_usage_percent;
        let can_run_diarization = has_enough_memory && cpu_below_threshold;

        // Recommend processing mode based on resources
        let recommended_mode = if !can_run_diarization {
            None
        } else if available_memory_mb >= 2000 && cpu_usage_percent < 50.0 {
            // Plenty of resources: use batch mode for better accuracy
            Some(ProcessingMode::Batch)
        } else if available_memory_mb >= 1000 && cpu_usage_percent < 70.0 {
            // Moderate resources: use real-time mode with larger chunks
            Some(ProcessingMode::RealTime { chunk_size_ms: 5000 })
        } else {
            // Limited resources: use real-time mode with smaller chunks
            Some(ProcessingMode::RealTime { chunk_size_ms: 2000 })
        };

        if !can_run_diarization {
            log::warn!(
                "Insufficient resources for diarization: {} MB available (need {}), {:.1}% CPU (max {})",
                available_memory_mb,
                self.config.min_available_memory_mb,
                cpu_usage_percent,
                self.config.max_cpu_usage_percent
            );
        } else {
            log::info!(
                "Resources available for diarization: {} MB, {:.1}% CPU, mode: {:?}",
                available_memory_mb,
                cpu_usage_percent,
                recommended_mode
            );
        }

        Ok(ResourceStatus {
            available_memory_mb,
            cpu_usage_percent,
            can_run_diarization,
            recommended_mode,
        })
    }

    /// Estimate resource cost for diarization
    pub fn estimate_diarization_cost(
        &self,
        audio_duration_seconds: f64,
        mode: ProcessingMode,
    ) -> ResourceEstimate {
        // Rough estimates based on typical diarization costs:
        // - Memory: ~100MB base + 50MB per hour of audio
        // - CPU: ~30% for real-time, ~50% for batch
        // - Duration: ~2x real-time for batch, ~1.5x for real-time
        
        let estimated_memory_mb = 100 + (audio_duration_seconds / 3600.0 * 50.0) as u64;
        let (estimated_cpu_percent, duration_multiplier) = match mode {
            ProcessingMode::RealTime { .. } => (30.0, 1.5),
            ProcessingMode::Batch => (50.0, 2.0),
        };
        let estimated_duration_seconds = audio_duration_seconds * duration_multiplier;

        log::debug!(
            "Estimated diarization cost for {:.1}s audio in {:?} mode: {} MB, {:.1}% CPU, {:.1}s duration",
            audio_duration_seconds,
            mode,
            estimated_memory_mb,
            estimated_cpu_percent,
            estimated_duration_seconds
        );

        ResourceEstimate {
            estimated_memory_mb,
            estimated_cpu_percent,
            estimated_duration_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_monitor_creation() {
        let config = ResourceConfig::default();
        let monitor = ResourceMonitor::new(config);
        
        // Should be able to create monitor
        assert!(monitor.system.read().is_ok());
    }

    #[test]
    fn test_check_resources() {
        let config = ResourceConfig {
            min_available_memory_mb: 100, // Low threshold for testing
            max_cpu_usage_percent: 95.0,
            check_interval_ms: 5000,
        };
        let monitor = ResourceMonitor::new(config);
        
        let result = monitor.check_resources();
        assert!(result.is_ok());
        
        let status = result.unwrap();
        assert!(status.available_memory_mb > 0);
        assert!(status.cpu_usage_percent >= 0.0);
        assert!(status.cpu_usage_percent <= 100.0);
    }

    #[test]
    fn test_resource_caching() {
        let config = ResourceConfig {
            min_available_memory_mb: 100,
            max_cpu_usage_percent: 95.0,
            check_interval_ms: 1000, // 1 second cache
        };
        let monitor = ResourceMonitor::new(config);
        
        // First check
        let result1 = monitor.check_resources();
        assert!(result1.is_ok());
        
        // Second check should use cache
        let result2 = monitor.check_resources();
        assert!(result2.is_ok());
        
        // Results should be identical (cached)
        let status1 = result1.unwrap();
        let status2 = result2.unwrap();
        assert_eq!(status1.available_memory_mb, status2.available_memory_mb);
    }

    #[test]
    fn test_insufficient_memory() {
        let config = ResourceConfig {
            min_available_memory_mb: 999999, // Impossibly high threshold
            max_cpu_usage_percent: 95.0,
            check_interval_ms: 5000,
        };
        let monitor = ResourceMonitor::new(config);
        
        let result = monitor.check_resources();
        assert!(result.is_ok());
        
        let status = result.unwrap();
        assert!(!status.can_run_diarization);
        assert!(status.recommended_mode.is_none());
    }

    #[test]
    fn test_high_cpu_usage() {
        let config = ResourceConfig {
            min_available_memory_mb: 100,
            max_cpu_usage_percent: 0.1, // Very low threshold
            check_interval_ms: 5000,
        };
        let monitor = ResourceMonitor::new(config);
        
        let result = monitor.check_resources();
        assert!(result.is_ok());
        
        let status = result.unwrap();
        // Might fail if CPU is actually idle, but typically will fail
        if status.cpu_usage_percent > 0.1 {
            assert!(!status.can_run_diarization);
        }
    }

    #[test]
    fn test_estimate_batch_mode() {
        let config = ResourceConfig::default();
        let monitor = ResourceMonitor::new(config);
        
        let estimate = monitor.estimate_diarization_cost(
            3600.0, // 1 hour
            ProcessingMode::Batch,
        );
        
        assert_eq!(estimate.estimated_memory_mb, 150); // 100 + 50
        assert_eq!(estimate.estimated_cpu_percent, 50.0);
        assert_eq!(estimate.estimated_duration_seconds, 7200.0); // 2x real-time
    }

    #[test]
    fn test_estimate_realtime_mode() {
        let config = ResourceConfig::default();
        let monitor = ResourceMonitor::new(config);
        
        let estimate = monitor.estimate_diarization_cost(
            3600.0, // 1 hour
            ProcessingMode::RealTime { chunk_size_ms: 5000 },
        );
        
        assert_eq!(estimate.estimated_memory_mb, 150); // 100 + 50
        assert_eq!(estimate.estimated_cpu_percent, 30.0);
        assert_eq!(estimate.estimated_duration_seconds, 5400.0); // 1.5x real-time
    }

    #[test]
    fn test_estimate_short_audio() {
        let config = ResourceConfig::default();
        let monitor = ResourceMonitor::new(config);
        
        let estimate = monitor.estimate_diarization_cost(
            60.0, // 1 minute
            ProcessingMode::Batch,
        );
        
        // Should still have base memory cost
        assert!(estimate.estimated_memory_mb >= 100);
        assert_eq!(estimate.estimated_cpu_percent, 50.0);
    }

    #[test]
    fn test_recommended_mode_high_resources() {
        let config = ResourceConfig {
            min_available_memory_mb: 100,
            max_cpu_usage_percent: 95.0,
            check_interval_ms: 5000,
        };
        let monitor = ResourceMonitor::new(config);
        
        let result = monitor.check_resources();
        assert!(result.is_ok());
        
        let status = result.unwrap();
        if status.can_run_diarization {
            assert!(status.recommended_mode.is_some());
        }
    }
}
