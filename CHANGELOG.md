# Changelog

All notable changes to Meetily will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-10

### Added

#### Speaker Diarization and Identification (NEW MAJOR FEATURE)
- **Automatic Speaker Detection**: Identifies and labels different speakers in meeting recordings
- **Speaker Segmentation**: Segments audio by speaker using voice embeddings and clustering
- **Name Identification**: Automatically extracts speaker names from introductions using LLM analysis
- **Voice Profiles**: Store and recognize speakers across multiple meetings
- **Speaker Statistics**: View speaking time, percentage, and turn count for each speaker
- **Manual Corrections**: Edit speaker names inline in the transcript view
- **Privacy Modes**: Choose between LocalOnly, PreferExternal, or ExternalOnly processing
- **GPU Acceleration**: Vulkan support for 5-10x faster diarization processing
- **Confidence Scoring**: Configurable confidence thresholds for name assignment
- **Overlapping Speech Detection**: Identifies when multiple speakers talk simultaneously

#### UI Components for Speaker Diarization
- **SpeakerLabel Component**: Color-coded badges with inline editing for speaker names
- **SpeakerStatisticsView Component**: Comprehensive statistics display with progress bars
- **DiarizationSettings Component**: Full configuration panel in Settings → Speakers tab
- **Enhanced TranscriptView**: Shows speaker labels above transcript segments
- **Visual Indicators**: Low confidence markers, overlapping speech badges

#### Database Schema
- **voice_profiles table**: Stores speaker voice profiles with SHA-256 hashed embeddings
- **speaker_mappings table**: Maps speaker labels to identified names per meeting
- **speaker_segments table**: Stores speaker segments with timestamps and confidence
- **enrollment_sessions table**: Tracks voice profile enrollment sessions

#### Backend Components
- **Model Router**: Intelligent routing between external/local models based on privacy settings
- **Diarization Engine**: Python (pyannote.audio) with Rust FFI wrapper
- **Identification Service**: LLM-based name extraction from transcript content
- **Speaker Mapper**: Maps labels to names and manages voice profiles
- **Synchronization Layer**: Aligns diarization timestamps with transcription
- **Transcript Enhancer**: Formats transcripts with speaker information
- **Profile Manager**: CRUD operations for voice profiles
- **Confidence Scorer**: Calculates and applies confidence thresholds
- **Error Recovery**: Retry logic and graceful degradation
- **Resource Monitor**: Monitors CPU/memory and recommends processing modes

#### Tauri Commands (9 new commands)
- `start_diarization`: Initiates speaker diarization for a meeting
- `get_speaker_segments`: Retrieves speaker segments for a meeting
- `update_speaker_name`: Manually corrects speaker names
- `merge_speakers`: Consolidates duplicate speaker labels
- `get_speaker_statistics`: Returns speaking time and turn statistics
- `configure_diarization`: Updates diarization configuration
- `enroll_speaker`: Creates voice profiles with consent
- `delete_voice_profile`: Removes voice profiles and data
- `list_voice_profiles`: Returns all stored voice profiles

#### Documentation
- **USER_GUIDE.md**: Complete user documentation for speaker diarization
- **DEVELOPER_GUIDE.md**: Architecture, API, and development guide
- **UI_INTEGRATION_GUIDE.md**: Frontend component documentation
- **SPEAKER_DIARIZATION_DEPLOYMENT.md**: Build and deployment instructions

#### Meeting Timestamp Enhancement
- **Timestamped Meeting Titles**: Meeting titles now automatically include the date in format `Meeting-YYYY-MM-DD-[LLM Generated Title]` (e.g., `Meeting-2026-02-09-Technical Updates`)
- **Timestamped Summary Headers**: All generated summaries now include a readable timestamp header in format `Meeting - [Readable Timestamp] - [LLM Title]` (e.g., `Meeting - Feb 9, 2026 7:49 AM -05:00 - Technical Updates`)
- **Timestamp Context Injection**: Meeting date and time are automatically injected into LLM prompts for better temporal context in summaries
- **Timezone Support**: All timestamps display in local timezone with proper abbreviations (e.g., EST, PST, -05:00)
- **New Timestamp Formatting Utilities**: Added `timestamp_formatter.rs` module with three formatting functions:
  - `format_timestamp_for_title()`: Concise format for UI display
  - `format_timestamp_for_summary()`: Detailed format for summary content
  - `format_timestamp_for_filename()`: Filesystem-safe format for file operations

#### Folder Management
- **Folder Deletion Feature**: Users can now delete meeting folders and all associated meetings
- **Cascade Delete**: Deleting a folder automatically removes all meetings within that folder
- **Improved Folder Organization**: Better folder management capabilities for organizing meetings

### Changed
- **Summary Generation**: Enhanced to include meeting timestamps in both the prompt context and the final summary header
- **Meeting Title Format**: Standardized format with date prefix for better chronological sorting and identification
- **Build System**: Improved Vulkan support detection and configuration for GPU acceleration
- **Default Features**: Vulkan now enabled by default in Cargo.toml for speaker diarization
- **Tauri API Imports**: Updated to use `@tauri-apps/api/core` instead of deprecated paths

### Fixed
- **MP4 Recording Issues**: Resolved issues with MP4 recording functionality
- **Memory Optimization**: Improved memory usage during long recording sessions
- **Gap Analysis**: Enhanced gap detection and handling in transcription
- **Timestamp Display**: Fixed duplicate timestamp formats in summary headers
- **TypeScript Imports**: Fixed Tauri API import paths in UI components
- **Missing UI Components**: Added Card and Slider components for diarization settings

### Technical Details

#### Build Artifacts
- **Debian Package**: `meetily_0.2.0_amd64.deb` (27 MB)
- **AppImage**: `meetily_0.2.0_amd64.AppImage` (98 MB)
- **Build Time**: ~7 minutes (6 min Rust + 30 sec Next.js)
- **Test Coverage**: 235 tests passing (96 diarization + 139 other modules)

#### Files Modified/Created
- **Rust Backend**: ~15,000 lines of new code in `src/diarization/` module
- **TypeScript UI**: ~1,500 lines in 3 new components
- **Python Engine**: ~500 lines in `diarization_engine.py`
- **Documentation**: ~5,000 lines across 4 guide documents
- **Database Migration**: `20260209000000_add_speaker_diarization.sql`

#### Dependencies Added
- `@radix-ui/react-slider`: For confidence threshold slider
- `sha2`: For voice embedding hashing
- `sysinfo`: For resource monitoring
- `pyannote.audio`: For speaker diarization (Python)

#### Build Requirements
- **Vulkan SDK**: Required for GPU acceleration
- **Python 3.8+**: For diarization engine
- **Build Command**: `pnpm run tauri:build:vulkan`
- **Features**: `--features vulkan` (enabled by default)

### Privacy & Security
- **Voice Embeddings**: Stored as SHA-256 hashes (irreversible)
- **No Raw Audio**: Raw audio not retained after enrollment
- **Privacy Modes**: User control over external model usage
- **GDPR/CCPA Compliant**: Right to deletion, data minimization, consent required

### Performance
- **GPU Acceleration**: 5-10x faster with Vulkan
- **Memory Usage**: ~100MB base + ~50MB per hour of audio
- **Processing Speed**: 
  - Real-time mode: ~1.5x real-time
  - Batch mode: ~2x real-time
  - With GPU: 5-10x faster

### Backward Compatibility
- Existing meetings without timestamps continue to work normally
- Speaker diarization is opt-in (disabled by default)
- All existing features remain functional
- Original meeting titles are preserved and not retroactively modified
- Timestamp feature only applies to newly generated summaries

## [0.1.x] - Previous Releases

### Core Features
- Local-first meeting transcription using Whisper and Parakeet models
- AI-powered summary generation with multiple LLM provider support (Ollama, Claude, Groq, OpenRouter, OpenAI)
- Real-time transcription during meetings
- Custom OpenAI-compatible endpoint support
- Professional audio mixing with microphone and system audio capture
- GPU acceleration support (Metal for macOS, CUDA for NVIDIA, Vulkan for AMD/Intel)
- Privacy-first design with all data stored locally
- Multi-platform support (macOS, Windows, Linux)
- Custom summary templates
- Meeting notes editor with BlockNote integration
- Folder organization for meetings
- Export capabilities
- Analytics consent and privacy controls

---

For more information about specific features and implementation details, see the documentation in the `.kiro/specs/` directory.
