# Speaker Diarization and Identification - Developer Guide

## Architecture Overview

The speaker diarization system consists of several interconnected components that work together to detect speakers, identify names, and enhance transcripts.

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Audio Input                              │
└────────────┬────────────────────────────┬───────────────────┘
             │                            │
             v                            v
    ┌────────────────┐          ┌────────────────┐
    │ Transcription  │          │ Model Router   │
    │    Engine      │          │                │
    └────────┬───────┘          └────────┬───────┘
             │                            │
             │                            v
             │                   ┌────────────────┐
             │                   │  Diarization   │
             │                   │    Engine      │
             │                   └────────┬───────┘
             │                            │
             v                            v
    ┌────────────────────────────────────────────┐
    │      Synchronization Layer                 │
    └────────────────┬───────────────────────────┘
                     │
                     v
    ┌────────────────────────────────────────────┐
    │      Identification Service                │
    └────────────────┬───────────────────────────┘
                     │
                     v
    ┌────────────────────────────────────────────┐
    │         Speaker Mapper                     │
    │    (with Voice Profile Database)           │
    └────────────────┬───────────────────────────┘
                     │
                     v
    ┌────────────────────────────────────────────┐
    │       Transcript Enhancer                  │
    └────────────────┬───────────────────────────┘
                     │
                     v
    ┌────────────────────────────────────────────┐
    │         Export / UI Display                │
    └────────────────────────────────────────────┘
```

## Core Components

### 1. Model Router (`router.rs`)

**Purpose**: Determines whether to use external/cloud or local models based on privacy settings and connectivity.

**Key Methods**:
```rust
pub async fn choose_diarization_model(&self) -> Result<ModelChoice>
pub async fn choose_llm_model(&self) -> Result<ModelChoice>
```

**Strategy**:
1. Check privacy mode (LocalOnly → always local)
2. Check internet connectivity (with timeout)
3. Try external model (with fallback)
4. Cache decision for session

**Configuration**:
```rust
pub struct RouterConfig {
    pub privacy_mode: PrivacyMode,
    pub external_api_timeout_ms: u64,
    pub cache_duration_seconds: u64,
}
```

### 2. Diarization Engine (`engine.rs`)

**Purpose**: Segments audio by speaker using voice embeddings and clustering.

**Technology**: Python (pyannote.audio) with Rust FFI wrapper

**Key Methods**:
```rust
pub fn process_audio(&self, audio: &[f32], sample_rate: u32) -> Result<Vec<SpeakerSegment>>
pub fn process_audio_chunk(&mut self, audio: &[f32], sample_rate: u32) -> Result<Vec<SpeakerSegment>>
pub fn finalize(&mut self) -> Result<Vec<SpeakerSegment>>
```

**Processing Modes**:
- **Batch**: Process complete audio for best accuracy
- **Real-Time**: Process chunks for live meetings

**Output**:
```rust
pub struct SpeakerSegment {
    pub speaker_label: String,      // "Speaker 1", "Speaker 2"
    pub start_time: f64,             // Seconds
    pub end_time: f64,               // Seconds
    pub confidence: f32,             // 0.0-1.0
    pub embedding: Vec<f32>,         // Voice embedding vector
}
```

### 3. Identification Service (`identification.rs`)

**Purpose**: Extracts speaker names from transcript content using LLM analysis.

**Key Methods**:
```rust
pub async fn identify_speakers(&self, request: IdentificationRequest) -> Result<Vec<IdentificationResult>>
```

**LLM Prompt Strategy**:
- Analyzes transcript for introduction patterns
- Extracts names like "I'm [name]", "This is [name]"
- Returns JSON with speaker labels, names, and confidence scores

**Integration**: Uses existing `summary::llm_client` module for LLM access

### 4. Speaker Mapper (`mapper.rs`)

**Purpose**: Maps speaker labels to identified names and manages voice profiles.

**Key Methods**:
```rust
pub async fn map_speakers(&self, meeting_id: &str, segments: &[SpeakerSegment], identifications: &[IdentificationResult]) -> Result<Vec<SpeakerMapping>>
pub async fn query_known_profiles(&self, embeddings: &[Vec<f32>]) -> Result<Vec<Option<VoiceProfile>>>
pub async fn create_voice_profile(&self, name: String, embedding: Vec<f32>) -> Result<VoiceProfile>
pub async fn update_mapping(&self, meeting_id: &str, speaker_label: &str, speaker_name: String, is_manual: bool) -> Result<()>
pub async fn merge_speaker_labels(&self, meeting_id: &str, source_label: &str, target_label: &str) -> Result<()>
```

**Database Integration**: Uses SQLite for persistent storage

### 5. Synchronization Layer (`sync.rs`)

**Purpose**: Aligns diarization timestamps with transcription word-level timestamps.

**Key Methods**:
```rust
pub fn synchronize(transcript_segments: &[TranscriptSegment], speaker_segments: &[SpeakerSegment]) -> Result<Vec<SynchronizedSegment>>
```

**Algorithm**:
1. Find overlapping segments by time
2. Use word-level timestamps as ground truth
3. Handle timing discrepancies (500ms tolerance)
4. Mark overlapping speech

### 6. Transcript Enhancer (`enhancer.rs`)

**Purpose**: Formats transcripts with speaker information and calculates statistics.

**Key Methods**:
```rust
pub fn enhance_transcript(&self, transcript_segments: &[TranscriptSegment], speaker_segments: &[SpeakerSegment], mappings: &[SpeakerMapping]) -> Result<EnhancedTranscript>
pub fn format_segment(&self, segment: &EnhancedSegment) -> String
pub fn calculate_statistics(&self, segments: &[EnhancedSegment]) -> SpeakerStatistics
```

**Output Format**:
```
[Speaker Name]: [text]
[Speaker Name & Other Name]: [overlapping speech]
Speaker 1 (?): [low confidence text]
```

### 7. Profile Manager (`profile_manager.rs`)

**Purpose**: Manages voice profile CRUD operations and enrollment.

**Key Methods**:
```rust
pub async fn create_profile(&self, name: String, embedding: Vec<f32>, consent_given: bool) -> Result<VoiceProfile>
pub async fn get_profile(&self, profile_id: &str) -> Result<Option<VoiceProfile>>
pub async fn delete_profile(&self, profile_id: &str) -> Result<()>
pub async fn auto_delete_old_profiles(&self) -> Result<u64>
pub async fn create_enrollment_session(&self, voice_profile_id: String, audio_duration_seconds: f64, sample_count: u32) -> Result<EnrollmentSession>
```

**Privacy**: Embeddings stored as SHA-256 hashes, no raw audio retention

### 8. Confidence Scorer (`confidence.rs`)

**Purpose**: Calculates and applies confidence thresholds for identifications.

**Key Methods**:
```rust
pub fn calculate_confidence(&self, identification: &IdentificationResult, context: &str) -> f32
pub fn should_assign_name(&self, confidence: f32) -> bool
pub fn apply_confidence_threshold(&self, mapping: &mut SpeakerMapping)
pub fn add_confidence_indicator(&self, segment: &mut EnhancedSegment)
```

**Factors**:
- Pattern match strength
- Context quality (filler words, clarity)
- Name quality (length, special characters)

### 9. Error Recovery (`error_recovery.rs`)

**Purpose**: Handles errors with retry logic and graceful degradation.

**Key Methods**:
```rust
pub async fn retry_with_backoff<F, T, E>(&self, operation: F, operation_name: &str) -> Result<T, E>
pub fn handle_diarization_error(&self, error: &DiarizationError, context: &str) -> Option<Vec<SpeakerSegment>>
pub fn create_fallback_transcript(&self, transcript_text: Vec<String>, timestamps: Vec<(f64, f64)>) -> EnhancedTranscript
```

**Strategy**:
- Retry transient failures (network, API, database)
- Fallback to simpler models
- Ensure transcripts available even when diarization fails

### 10. Resource Monitor (`resource_monitor.rs`)

**Purpose**: Monitors system resources and recommends processing modes.

**Key Methods**:
```rust
pub fn check_resources(&self) -> Result<ResourceStatus>
pub fn estimate_diarization_cost(&self, audio_duration_seconds: f64, mode: ProcessingMode) -> ResourceEstimate
```

**Thresholds**:
- Minimum memory: 500MB (default)
- Maximum CPU: 80% (default)
- Check interval: 5 seconds

## Database Schema

### Voice Profiles Table
```sql
CREATE TABLE voice_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    embedding_hash TEXT NOT NULL,  -- SHA-256 hash
    created_at TIMESTAMP NOT NULL,
    last_seen TIMESTAMP NOT NULL,
    meeting_count INTEGER DEFAULT 0,
    metadata TEXT  -- JSON
);
```

### Speaker Mappings Table
```sql
CREATE TABLE speaker_mappings (
    meeting_id TEXT NOT NULL,
    speaker_label TEXT NOT NULL,
    speaker_name TEXT,
    voice_profile_id TEXT,
    confidence REAL NOT NULL,
    is_manual BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    PRIMARY KEY (meeting_id, speaker_label),
    FOREIGN KEY (voice_profile_id) REFERENCES voice_profiles(id)
);
```

### Speaker Segments Table
```sql
CREATE TABLE speaker_segments (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    speaker_label TEXT NOT NULL,
    start_time REAL NOT NULL,
    end_time REAL NOT NULL,
    confidence REAL NOT NULL,
    embedding_hash TEXT,  -- SHA-256 hash
    created_at TIMESTAMP NOT NULL
);
```

### Enrollment Sessions Table
```sql
CREATE TABLE enrollment_sessions (
    id TEXT PRIMARY KEY,
    voice_profile_id TEXT NOT NULL,
    audio_duration_seconds REAL NOT NULL,
    sample_count INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (voice_profile_id) REFERENCES voice_profiles(id)
);
```

## Build Requirements

### Critical: Vulkan Feature Flag

**ALL builds MUST use the `vulkan` feature flag:**

```bash
# Development
cargo build --features vulkan

# Release
cargo build --release --features vulkan

# Tests
cargo test --features vulkan
```

**Why Vulkan?**
- Enables GPU acceleration for pyannote.audio models
- 5-10x speedup for diarization
- Reduces CPU load during live meetings
- Required for real-time processing

### Dependencies

**Cargo.toml**:
```toml
[features]
default = ["vulkan"]
vulkan = ["dep:vulkan-bindings"]

[dependencies]
# Core
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
log = "0.4"

# Diarization specific
sha2 = "0.10"
sysinfo = "0.30"
vulkan-bindings = { version = "0.1", optional = true }
```

**Python Requirements** (`python/requirements-diarization.txt`):
```
pyannote.audio>=3.1.0
torch>=2.0.0
torchaudio>=2.0.0
```

### Environment Setup

```bash
# Set up Python environment
cd frontend/src-tauri/python
./setup_diarization.sh

# Optional: External API keys
export HUGGINGFACE_API_KEY="your_key"
export OPENAI_API_KEY="your_key"

# Optional: Force local-only mode
export MEETILY_LOCAL_ONLY=true
```

## API Endpoints (Tauri Commands)

### Implemented Commands

All commands are implemented in `src/diarization/commands.rs` and registered in `src/lib.rs`.

#### Core Diarization Commands

**`start_diarization`** - Initiates speaker diarization for a recorded meeting
```rust
#[tauri::command]
async fn start_diarization<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
    audio_path: String,
    config: Option<DiarizationConfigDto>,
) -> Result<serde_json::Value, String>
```

**`get_speaker_segments`** - Retrieves all speaker segments for a meeting
```rust
#[tauri::command]
async fn get_speaker_segments<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<SpeakerSegmentDto>, String>
```

**`update_speaker_name`** - Manually corrects or assigns a name to a speaker label
```rust
#[tauri::command]
async fn update_speaker_name<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
    speaker_label: String,
    new_name: String,
) -> Result<serde_json::Value, String>
```

**`merge_speakers`** - Consolidates two speaker labels into one
```rust
#[tauri::command]
async fn merge_speakers<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
    source_label: String,
    target_label: String,
) -> Result<serde_json::Value, String>
```

**`get_speaker_statistics`** - Returns speaking time, percentage, and turn count for each speaker
```rust
#[tauri::command]
async fn get_speaker_statistics<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<SpeakerStatisticsDto>, String>
```

#### Configuration Commands

**`configure_diarization`** - Updates the global diarization configuration
```rust
#[tauri::command]
async fn configure_diarization<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    config: DiarizationConfigDto,
) -> Result<serde_json::Value, String>
```

#### Voice Profile Commands

**`enroll_speaker`** - Creates a voice profile for a speaker (requires consent)
```rust
#[tauri::command]
async fn enroll_speaker<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    name: String,
    audio_samples: Vec<String>,
    consent_given: bool,
) -> Result<VoiceProfileDto, String>
```

**`delete_voice_profile`** - Removes a voice profile and all associated data
```rust
#[tauri::command]
async fn delete_voice_profile<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<serde_json::Value, String>
```

**`list_voice_profiles`** - Returns all stored voice profiles
```rust
#[tauri::command]
async fn list_voice_profiles<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Vec<VoiceProfileDto>, String>
```

### Data Transfer Objects (DTOs)

**DiarizationConfigDto**:
```rust
pub struct DiarizationConfigDto {
    pub processing_mode: String,      // "Batch" | "RealTime"
    pub privacy_mode: String,          // "LocalOnly" | "PreferExternal" | "ExternalOnly"
    pub confidence_threshold: Option<f32>,
    pub enable_identification: Option<bool>,
}
```

**SpeakerSegmentDto**:
```rust
pub struct SpeakerSegmentDto {
    pub speaker_label: String,
    pub speaker_name: Option<String>,
    pub start_time: f64,
    pub end_time: f64,
    pub confidence: f32,
    pub embedding_hash: Option<String>,
}
```

**SpeakerStatisticsDto**:
```rust
pub struct SpeakerStatisticsDto {
    pub speaker_label: String,
    pub speaker_name: Option<String>,
    pub speaking_time_seconds: f64,
    pub speaking_percentage: f32,
    pub turn_count: usize,
}
```

**VoiceProfileDto**:
```rust
pub struct VoiceProfileDto {
    pub id: String,
    pub name: String,
    pub embedding_hash: String,
    pub created_at: String,
    pub last_seen: String,
    pub meeting_count: u32,
}
```

### Frontend Usage Examples

**TypeScript/React**:
```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Start diarization
await invoke('start_diarization', {
  meetingId: 'meeting-123',
  audioPath: '/path/to/audio.wav',
  config: {
    processing_mode: 'Batch',
    privacy_mode: 'PreferExternal',
    confidence_threshold: 0.7
  }
});

// Get speaker segments
const segments = await invoke('get_speaker_segments', {
  meetingId: 'meeting-123'
});

// Update speaker name
await invoke('update_speaker_name', {
  meetingId: 'meeting-123',
  speakerLabel: 'Speaker 1',
  newName: 'John Doe'
});

// Get statistics
const stats = await invoke('get_speaker_statistics', {
  meetingId: 'meeting-123'
});
```

## Testing

### Unit Tests

Run all unit tests:
```bash
cargo test --features vulkan --lib diarization
```

Current coverage: **96 tests passing**

### Test Structure

Each module has comprehensive unit tests:
- `router.rs`: 6 tests (connectivity, caching, privacy modes)
- `engine.rs`: 1 test (creation)
- `identification.rs`: 6 tests (LLM integration, parsing)
- `mapper.rs`: 6 tests (mapping, profiles, merging)
- `sync.rs`: 8 tests (alignment, overlaps, conflicts)
- `enhancer.rs`: 6 tests (formatting, statistics)
- `export.rs`: 7 tests (formats, preservation)
- `embedding.rs`: 12 tests (hashing, similarity, clustering)
- `profile_manager.rs`: 10 tests (CRUD, enrollment, auto-deletion)
- `confidence.rs`: 13 tests (scoring, thresholds, indicators)
- `error_recovery.rs`: 12 tests (retry, fallback, degradation)
- `resource_monitor.rs`: 9 tests (monitoring, estimation, caching)

### Property-Based Tests (Optional)

Property tests validate universal correctness properties across randomized inputs. See design document for full list of 67 properties.

Example:
```rust
#[test]
fn test_speaker_labels_unique() {
    // Property 3: Speaker Labels Are Unique Within Session
    proptest!(|(segments in arbitrary_segments())| {
        let labels: HashSet<_> = segments.iter()
            .map(|s| &s.speaker_label)
            .collect();
        prop_assert_eq!(labels.len(), segments.len());
    });
}
```

## Integration Points

### Transcription Engines

Diarization integrates with:
- **Whisper**: Word-level timestamps for alignment
- **Parakeet**: Word-level timestamps for alignment

Both engines provide `TranscriptSegment` with `WordTiming` data.

### LLM Service

Uses existing `summary::llm_client` module:
- Supports multiple providers (Ollama, OpenAI, Claude, Groq, Gemini)
- Handles API calls and response parsing
- Provides fallback mechanisms

### Database

Uses SQLite via `sqlx`:
- Async operations with tokio runtime
- Transaction support for consistency
- Migration system for schema updates

## Performance Considerations

### Memory Usage

Typical memory usage:
- Base: ~100MB
- Per hour of audio: ~50MB
- Voice profiles: ~1KB each

### CPU Usage

- Real-time mode: ~30% of one core
- Batch mode: ~50% of one core
- GPU acceleration: Significantly reduces CPU load

### Processing Speed

- Real-time mode: ~1.5x real-time
- Batch mode: ~2x real-time
- With GPU: 5-10x faster

### Optimization Tips

1. **Use External Models**: Conserves local resources
2. **Enable GPU**: Vulkan feature flag required
3. **Batch Processing**: Better accuracy, higher throughput
4. **Resource Limits**: Prevent system overload
5. **Caching**: Router caches decisions for 5 minutes

## Error Handling

### Error Types

```rust
pub enum DiarizationError {
    AudioProcessingError(String),
    ModelLoadError(String),
    IdentificationError(String),
    DatabaseError(String),
    ResourceConstraintError(String),
    NetworkError(String),
    ExternalApiError(String),
    EmbeddingError(String),
    SynchronizationError(String),
    ExportError(String),
    ConsentRequired(String),
    InvalidConfiguration(String),
    // ...
}
```

### Retry Strategy

- **Retryable**: Network, API, Database, Resource errors
- **Non-retryable**: Audio processing, Configuration, Consent errors
- **Max retries**: 3 attempts
- **Backoff**: Exponential (1s, 2s, 4s)

### Graceful Degradation

When diarization fails:
1. Log error details (not exposed to user)
2. Return empty speaker segments
3. Provide transcripts without speaker labels
4. Continue processing with available data

## Privacy and Security

### Data Storage

**Stored**:
- Voice embedding hashes (SHA-256, irreversible)
- Speaker mappings (label → name associations)
- Enrollment metadata (no raw audio)

**NOT Stored**:
- Raw audio from enrollment
- Raw embedding vectors
- External API responses

### Compliance

- **GDPR**: Right to deletion, data minimization, consent
- **CCPA**: Data deletion, no sale of data
- **Privacy by Design**: Local processing default, hashed embeddings

### Security Best Practices

1. **Input Validation**: Sanitize all user inputs
2. **SQL Injection**: Use parameterized queries
3. **API Keys**: Store securely, never log
4. **Error Messages**: Don't expose sensitive details
5. **Consent**: Required for voice profile creation

## Debugging

### Enable Debug Logging

```rust
env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
```

### Common Issues

**Diarization not running**:
- Check `cargo build --features vulkan` was used
- Verify Python environment setup
- Check resource availability

**Poor accuracy**:
- Use batch mode for better results
- Try external models
- Ensure good audio quality

**High resource usage**:
- Enable resource limits
- Use external models
- Reduce chunk size in real-time mode

### Logging Levels

- **ERROR**: Critical failures
- **WARN**: Degraded functionality
- **INFO**: Normal operations
- **DEBUG**: Detailed diagnostics

## Contributing

### Code Style

- Follow Rust standard style (`rustfmt`)
- Add rustdoc comments to public APIs
- Include examples for complex functions
- Write tests for new features

### Pull Request Checklist

- [ ] All tests pass (`cargo test --features vulkan`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated

### Testing Requirements

- Unit tests for all new functions
- Integration tests for end-to-end flows
- Property tests for universal properties (optional)
- Manual testing with real audio

## Future Enhancements

### Planned Features

- Real-time streaming support (currently batch-focused)
- Multi-language support
- Speaker verification
- Emotion detection
- Advanced voice profile matching (vector database)

### Performance Improvements

- WebGPU support (in addition to Vulkan)
- Model quantization for faster inference
- Incremental processing for long meetings
- Parallel processing for multiple meetings

## References

- [pyannote.audio Documentation](https://github.com/pyannote/pyannote-audio)
- [Vulkan Documentation](https://www.vulkan.org/)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [Tauri Documentation](https://tauri.app/)
