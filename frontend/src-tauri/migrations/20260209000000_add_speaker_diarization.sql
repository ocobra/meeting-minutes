-- Migration: Add speaker diarization tables
-- Date: 2026-02-09
-- Description: Creates tables for speaker diarization and identification feature

-- Voice profiles for known speakers
CREATE TABLE IF NOT EXISTS voice_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    embedding_hash TEXT NOT NULL,  -- SHA-256 hash of voice embedding (for privacy)
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    meeting_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT  -- JSON for additional data
);

-- Speaker mappings for each meeting
CREATE TABLE IF NOT EXISTS speaker_mappings (
    meeting_id TEXT NOT NULL,
    speaker_label TEXT NOT NULL,
    speaker_name TEXT,
    voice_profile_id TEXT,
    confidence REAL NOT NULL,
    is_manual BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (meeting_id, speaker_label),
    FOREIGN KEY (voice_profile_id) REFERENCES voice_profiles(id) ON DELETE SET NULL
);

-- Speaker segments for each meeting
CREATE TABLE IF NOT EXISTS speaker_segments (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    speaker_label TEXT NOT NULL,
    start_time REAL NOT NULL,
    end_time REAL NOT NULL,
    confidence REAL NOT NULL,
    embedding_hash TEXT,  -- SHA-256 hash of voice embedding
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Speaker enrollment sessions
CREATE TABLE IF NOT EXISTS enrollment_sessions (
    id TEXT PRIMARY KEY,
    voice_profile_id TEXT NOT NULL,
    audio_duration_seconds REAL NOT NULL,
    sample_count INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (voice_profile_id) REFERENCES voice_profiles(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_voice_profiles_name ON voice_profiles(name);
CREATE INDEX IF NOT EXISTS idx_voice_profiles_embedding_hash ON voice_profiles(embedding_hash);
CREATE INDEX IF NOT EXISTS idx_speaker_mappings_meeting ON speaker_mappings(meeting_id);
CREATE INDEX IF NOT EXISTS idx_speaker_mappings_profile ON speaker_mappings(voice_profile_id);
CREATE INDEX IF NOT EXISTS idx_speaker_segments_meeting ON speaker_segments(meeting_id);
CREATE INDEX IF NOT EXISTS idx_speaker_segments_label ON speaker_segments(meeting_id, speaker_label);
CREATE INDEX IF NOT EXISTS idx_enrollment_sessions_profile ON enrollment_sessions(voice_profile_id);
