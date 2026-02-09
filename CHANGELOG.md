# Changelog

All notable changes to Meetily will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-09

### Added

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

### Fixed
- **MP4 Recording Issues**: Resolved issues with MP4 recording functionality
- **Memory Optimization**: Improved memory usage during long recording sessions
- **Gap Analysis**: Enhanced gap detection and handling in transcription
- **Timestamp Display**: Fixed duplicate timestamp formats in summary headers

### Technical Details
- **Files Modified**:
  - `frontend/src-tauri/src/utils/timestamp_formatter.rs` (created)
  - `frontend/src-tauri/src/utils/mod.rs` (updated)
  - `frontend/src-tauri/src/summary/service.rs` (enhanced)
  - `frontend/src-tauri/src/database/repositories/meeting.rs` (folder deletion)
  
- **Build Requirements**:
  - Vulkan support: `VULKAN_SDK=/usr BLAS_INCLUDE_DIRS=/usr/include/x86_64-linux-gnu bash build-gpu.sh`
  - Package: `meetily_0.2.0_amd64.deb`

### Backward Compatibility
- Existing meetings without timestamps continue to work normally
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
