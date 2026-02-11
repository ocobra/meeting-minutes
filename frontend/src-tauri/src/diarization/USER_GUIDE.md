# Speaker Diarization and Identification - User Guide

## Overview

Speaker diarization and identification automatically detects who is speaking when in your meetings and extracts speaker names from introductions. This feature enhances your transcripts with speaker labels, making it easier to follow conversations and attribute statements to the right people.

## Features

- **Automatic Speaker Detection**: Identifies when different speakers are talking
- **Name Extraction**: Automatically extracts speaker names from meeting introductions
- **Speaker Labels**: Assigns consistent labels (Speaker 1, Speaker 2, etc.) throughout the meeting
- **Voice Profiles**: Remembers speakers across multiple meetings
- **Manual Corrections**: Edit speaker names and merge duplicate speakers
- **Speaker Statistics**: View speaking time, turn count, and contribution percentages
- **Privacy-First**: All processing happens locally by default
- **Export Support**: Export transcripts with speaker labels in multiple formats

## Getting Started

### Enabling Diarization

1. Open Meetily settings
2. Navigate to the "Diarization" section
3. Toggle "Enable Speaker Diarization" to ON
4. Choose your preferred privacy mode (see Privacy Modes below)

### Privacy Modes

Meetily offers three privacy modes for speaker diarization:

#### Local Only (Maximum Privacy)
- All processing happens on your device
- No data sent to external services
- Requires local models to be downloaded
- Best for: Sensitive meetings, privacy-conscious users

#### Prefer External (Recommended)
- Uses cloud models when internet is available
- Falls back to local models when offline
- Better accuracy and faster processing
- Conserves local system resources
- Best for: Most users, balanced privacy and performance

#### External Only
- Always uses cloud models
- Fails if internet is unavailable
- Best accuracy and performance
- Best for: Users with reliable internet, maximum accuracy needs

### How It Works

1. **Recording**: Start a meeting recording as usual
2. **Diarization**: Meetily automatically detects speaker changes
3. **Identification**: Names are extracted from introductions like "I'm John Smith"
4. **Enhancement**: Transcripts are updated with speaker names
5. **Review**: Review and correct speaker attributions if needed

## Using Speaker Features

### Viewing Speaker-Enhanced Transcripts

Transcripts with speaker diarization show:
- Speaker names or labels before each segment
- Visual indicators for speaker changes
- Confidence indicators (?) for uncertain identifications
- Overlapping speech markers when multiple people talk at once

Example:
```
John Smith: Hello everyone, welcome to the meeting.
Sarah Jones: Thanks John, glad to be here.
[John Smith & Sarah Jones]: [overlapping speech]
Speaker 3 (?): I think we should proceed with the plan.
```

### Manual Corrections

If the automatic identification is incorrect:

1. Click on a speaker name in the transcript
2. Edit the name or select from known speakers
3. Choose "Apply to all segments" to update all instances
4. Merge duplicate speakers if the same person has multiple labels

### Voice Profile Enrollment

To help Meetily recognize you in future meetings:

1. Go to Settings → Diarization → Voice Profiles
2. Click "Enroll New Speaker"
3. Read the provided 15-second passage
4. Confirm your name
5. Your voice profile is saved locally

**Note**: Voice profiles are stored as mathematical hashes (not raw audio) for privacy compliance.

### Speaker Statistics

View detailed statistics for each meeting:
- **Speaking Time**: Total time each speaker talked
- **Percentage**: Proportion of meeting time
- **Turn Count**: Number of times each speaker spoke
- **Average Turn**: Average duration of speaking turns

Access statistics from:
- Meeting summary view
- Export options (included in all formats)
- Speaker statistics panel

## Configuration

### Confidence Threshold

Adjust how confident the system must be before assigning names:
- **High (0.9)**: Only very confident identifications
- **Medium (0.7)**: Balanced (recommended)
- **Low (0.5)**: More names, but potentially less accurate

### Resource Limits

Configure system resource usage:
- **Memory Limit**: Minimum free memory required (default: 500MB)
- **CPU Limit**: Maximum CPU usage allowed (default: 80%)
- **Processing Mode**: Real-time vs Batch processing

### Auto-Deletion

Control how long speaker data is retained:
- **Never**: Keep all voice profiles indefinitely
- **30 days**: Delete profiles not seen in 30 days
- **90 days**: Delete profiles not seen in 90 days (recommended)
- **180 days**: Delete profiles not seen in 180 days

## Exporting Transcripts

Export speaker-enhanced transcripts in multiple formats:

### Text Format
Plain text with speaker labels:
```
[00:00] John Smith: Hello everyone
[00:15] Sarah Jones: Thanks for joining
```

### Markdown Format
Formatted markdown with statistics table:
```markdown
## Meeting Transcript

**John Smith**: Hello everyone

**Sarah Jones**: Thanks for joining

## Speaker Statistics
| Speaker | Time | Percentage | Turns |
|---------|------|------------|-------|
| John Smith | 5m 30s | 45% | 12 |
| Sarah Jones | 6m 45s | 55% | 15 |
```

### JSON Format
Complete metadata for programmatic access:
```json
{
  "segments": [
    {
      "speaker_name": "John Smith",
      "text": "Hello everyone",
      "start_time": 0.0,
      "end_time": 2.5,
      "confidence": 0.95
    }
  ],
  "statistics": {
    "speakers": [...]
  }
}
```

## Troubleshooting

### Diarization Not Working

**Problem**: Speaker detection is not running

**Solutions**:
1. Check that diarization is enabled in settings
2. Verify sufficient system resources (500MB+ free memory)
3. Check that Python environment is set up (see setup guide)
4. Review logs for error messages

### Poor Speaker Detection

**Problem**: Speakers are not correctly separated

**Solutions**:
1. Ensure good audio quality (clear microphone, minimal background noise)
2. Try batch processing mode for better accuracy
3. Adjust speaker change sensitivity in settings
4. Use external models for better accuracy

### Names Not Extracted

**Problem**: Speaker labels shown instead of names

**Solutions**:
1. Ensure speakers introduce themselves clearly ("I'm [name]")
2. Check confidence threshold (lower it if too strict)
3. Manually assign names and save as voice profiles
4. Verify LLM service is available

### High Resource Usage

**Problem**: Diarization uses too much CPU/memory

**Solutions**:
1. Enable resource limits in settings
2. Use real-time mode with smaller chunks
3. Defer diarization to after meeting ends
4. Use external models to conserve local resources

## Privacy and Data

### What Data is Stored?

- **Voice Embeddings**: Mathematical representations (hashes) of voice characteristics
- **Speaker Mappings**: Associations between labels and names for each meeting
- **Enrollment Sessions**: Metadata about voice profile creation (no raw audio)

### What Data is NOT Stored?

- **Raw Audio**: Never stored from enrollment sessions
- **Embedding Vectors**: Only irreversible SHA-256 hashes are stored
- **External Data**: No data sent to external services in Local Only mode

### Deleting Your Data

To delete speaker data:

1. **Individual Profiles**: Settings → Voice Profiles → Delete
2. **Meeting Data**: Delete the meeting to remove all speaker segments
3. **All Data**: Settings → Diarization → Delete All Voice Profiles

### GDPR and CCPA Compliance

Meetily's speaker diarization is designed for privacy compliance:
- Voice embeddings stored as irreversible hashes
- No raw audio retention
- User consent required for enrollment
- Easy data deletion
- Local processing by default

## Best Practices

### For Best Results

1. **Clear Introductions**: Have participants introduce themselves at the start
2. **Good Audio**: Use quality microphones and minimize background noise
3. **Enroll Speakers**: Pre-enroll frequent participants
4. **Review and Correct**: Manually verify important meetings
5. **Regular Cleanup**: Delete old voice profiles you no longer need

### For Privacy

1. **Use Local Only Mode**: For sensitive meetings
2. **Disable Diarization**: For meetings you don't want analyzed
3. **Regular Deletion**: Enable auto-deletion of old profiles
4. **Review Profiles**: Periodically check and delete unused profiles

### For Performance

1. **Use External Models**: When internet is available
2. **Batch Processing**: For recorded meetings (better accuracy)
3. **Real-Time Mode**: For live meetings (lower latency)
4. **Resource Limits**: Set appropriate limits for your system

## Keyboard Shortcuts

- **Edit Speaker Name**: Click on speaker name
- **Merge Speakers**: Select multiple segments, right-click → Merge
- **View Statistics**: Ctrl/Cmd + Shift + S
- **Export Transcript**: Ctrl/Cmd + E

## Support

For additional help:
- Check the FAQ in settings
- Review error logs in the console
- Report issues on GitHub
- Contact support for assistance

## Updates

Speaker diarization is continuously improving. Check for updates regularly to get:
- Better accuracy
- New features
- Performance improvements
- Bug fixes
