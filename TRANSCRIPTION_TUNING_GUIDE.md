# Meetily Audio Transcription Tuning Guide

## Overview

Meetily uses Whisper (and optionally Parakeet) for audio transcription with adaptive configuration based on your hardware. This guide explains how to tune transcription accuracy for your specific needs.

## Current Adaptive Configuration

Meetily automatically detects your hardware and adjusts transcription parameters accordingly:

### Performance Tiers

Your system is automatically classified into one of four tiers:

1. **Ultra** (Best Quality)
   - Requirements: GPU (Metal/CUDA) + 16GB+ RAM + 8+ CPU cores
   - Beam Size: 5
   - Temperature: 0.1
   - Chunk Size: Quality (larger chunks)

2. **High** (High Quality)
   - Requirements: GPU (Metal/CUDA/Vulkan) + 12GB+ RAM + 6+ CPU cores
   - Beam Size: 3
   - Temperature: 0.2
   - Chunk Size: Balanced

3. **Medium** (Balanced)
   - Requirements: 4+ CPU cores + 8GB+ RAM
   - Beam Size: 2
   - Temperature: 0.3
   - Chunk Size: Balanced

4. **Low** (Fast Processing)
   - Requirements: Basic hardware
   - Beam Size: 1
   - Temperature: 0.4
   - Chunk Size: Fast (smaller chunks)
   - GPU: Disabled (CPU only)

## Key Parameters for Accuracy

### 1. Beam Size (Most Important for Accuracy)

**What it does**: Controls how many alternative transcription paths Whisper explores simultaneously.

**Current Values**:
- Ultra: 5 (maximum quality)
- High: 3
- Medium/Windows: 2
- Low: 1

**To Improve Accuracy**:
- Increase beam size for better accuracy (but slower processing)
- Recommended range: 2-5
- Values above 5 provide diminishing returns

**Location**: `frontend/src-tauri/src/audio/hardware_detector.rs`

```rust
PerformanceTier::Ultra => AdaptiveWhisperConfig {
    beam_size: 5,  // Increase this for better accuracy
    // ...
},
```

### 2. Temperature (Affects Confidence Threshold)

**What it does**: Controls how conservative Whisper is with predictions. Lower = more conservative.

**Current Values**:
- Ultra: 0.1 (very conservative)
- High: 0.2
- Medium: 0.3
- Low: 0.4

**To Improve Accuracy**:
- Lower temperature = more accurate but may miss some words
- Higher temperature = more words transcribed but may include hallucinations
- Recommended range: 0.1-0.3 for accuracy

**Location**: `frontend/src-tauri/src/audio/hardware_detector.rs`

```rust
temperature: 0.1,  // Lower for more accuracy
```

### 3. No Speech Threshold

**What it does**: Determines when audio is considered "silence" vs "speech".

**Current Value**: 0.55 (balanced)

**To Improve Accuracy**:
- Lower value (0.4-0.5): Captures more quiet speech but may include background noise
- Higher value (0.6-0.7): Filters out noise better but may miss quiet speech
- Current 0.55 is well-balanced for most use cases

**Location**: `frontend/src-tauri/src/whisper_engine/whisper_engine.rs`

```rust
params.set_no_speech_thold(0.55);  // Adjust for quiet speech detection
```

### 4. Language Setting

**What it does**: Tells Whisper what language to expect.

**Current Options**:
- `"auto"`: Automatic language detection
- `"auto-translate"`: Detect language and translate to English (default)
- Specific language code: e.g., `"en"`, `"es"`, `"fr"`, `"de"`, `"zh"`

**To Improve Accuracy**:
- Set a specific language if you know what language will be spoken
- This significantly improves accuracy vs auto-detection
- Use `"en"` for English-only meetings

**How to Change**: Through Meetily settings UI (Language Selection)

### 5. Whisper Model Size

**What it does**: Larger models are more accurate but slower.

**Available Models**:
- `tiny`: Fastest, least accurate (~75MB)
- `base`: Fast, decent accuracy (~150MB)
- `small`: Balanced (~500MB)
- `medium`: High accuracy (~1.5GB)
- `large-v2` / `large-v3`: Best accuracy (~3GB)

**To Improve Accuracy**:
- Use `medium` or `large-v3` for best results
- Requires more VRAM/RAM
- Significantly slower processing

**How to Change**: Through Meetily settings UI (Model Selection)

## Advanced Tuning Parameters

These parameters are set in `frontend/src-tauri/src/whisper_engine/whisper_engine.rs`:

### Entropy Threshold
```rust
params.set_entropy_thold(2.4);  // Lower = more conservative
```
- Controls prediction confidence
- Lower values = higher quality but may skip uncertain words
- Range: 1.5-3.0

### Log Probability Threshold
```rust
params.set_logprob_thold(-1.0);  // Lower = more conservative
```
- Filters out low-confidence predictions
- Lower values = stricter filtering
- Range: -1.5 to -0.5

### Max Initial Timestamp
```rust
params.set_max_initial_ts(1.0);
```
- Controls how far into audio to look for speech start
- Increase if speech starts late in chunks

### Max Length
```rust
params.set_max_len(200);
```
- Maximum tokens per segment
- Increase for longer continuous speech

## Practical Tuning Recommendations

### For Maximum Accuracy (Slow)
```rust
// In hardware_detector.rs
beam_size: 5,
temperature: 0.1,

// In whisper_engine.rs
params.set_no_speech_thold(0.50);
params.set_entropy_thold(2.0);
params.set_logprob_thold(-1.2);
```
- Use `large-v3` model
- Set specific language (not auto)
- Enable GPU acceleration

### For Balanced Performance (Recommended)
```rust
// Current defaults are well-balanced
beam_size: 3,
temperature: 0.2,
params.set_no_speech_thold(0.55);
```
- Use `medium` or `small` model
- Use `auto-translate` for mixed languages
- Enable GPU acceleration

### For Fast Processing (Lower Accuracy)
```rust
beam_size: 1,
temperature: 0.4,
params.set_no_speech_thold(0.60);
```
- Use `tiny` or `base` model
- Set specific language
- CPU-only mode acceptable

## Audio Quality Improvements

Beyond Whisper parameters, improve input audio quality:

### 1. Microphone Selection
- Use a quality external microphone
- Position microphone 6-12 inches from mouth
- Use pop filter to reduce plosives

### 2. Recording Environment
- Minimize background noise
- Use acoustic treatment if possible
- Close windows, turn off fans/AC during recording

### 3. Audio Levels
- Speak at consistent volume
- Avoid clipping (too loud)
- Avoid levels too quiet (below -30dB)

### 4. Audio Processing
Meetily includes built-in audio processing:
- **Noise Reduction**: Enabled by default (nnnoiseless)
- **Normalization**: Automatic level adjustment
- **VAD (Voice Activity Detection)**: Filters silence

## How to Apply Custom Tuning

### Method 1: Modify Hardware Detector (Recommended)

Edit `frontend/src-tauri/src/audio/hardware_detector.rs`:

```rust
pub fn get_whisper_config(&self) -> AdaptiveWhisperConfig {
    // Force high-quality settings regardless of hardware
    AdaptiveWhisperConfig {
        beam_size: 5,           // Maximum quality
        temperature: 0.1,       // Very conservative
        use_gpu: true,          // Enable GPU if available
        max_threads: Some(8),   // Use more threads
        chunk_size_preference: ChunkSizePreference::Quality,
    }
}
```

### Method 2: Modify Whisper Engine Parameters

Edit `frontend/src-tauri/src/whisper_engine/whisper_engine.rs`:

```rust
// Around line 580
params.set_no_speech_thold(0.50);  // Lower for quiet speech
params.set_entropy_thold(2.0);     // Lower for higher quality
params.set_temperature(0.1);       // Override adaptive temperature
```

### Method 3: Environment Variables (Future Enhancement)

Consider adding environment variable support:
```bash
export MEETILY_BEAM_SIZE=5
export MEETILY_TEMPERATURE=0.1
export MEETILY_LANGUAGE=en
```

## Rebuild After Changes

After modifying parameters, rebuild Meetily:

```bash
cd frontend
VULKAN_SDK=/usr BLAS_INCLUDE_DIRS=/usr/include/x86_64-linux-gnu bash build-gpu.sh
sudo dpkg -i target/release/bundle/deb/meetily_0.2.0_amd64.deb
```

## Monitoring Transcription Quality

### Check Logs
Meetily logs transcription details:
```bash
# View logs
tail -f ~/.local/share/meetily/logs/meetily.log

# Look for:
# - "Performance Tier: Ultra/High/Medium/Low"
# - "Beam Size: X"
# - "Successfully loaded model: ..."
```

### Confidence Scores
Meetily provides confidence scores for each transcription segment. Check the transcript view for low-confidence segments (highlighted in UI).

## Common Issues and Solutions

### Issue: Missing Quiet Speech
**Solution**: Lower `no_speech_thold` from 0.55 to 0.45-0.50

### Issue: Hallucinations (Random Words)
**Solution**: 
- Lower temperature (0.1-0.2)
- Increase beam size (3-5)
- Lower entropy threshold (2.0-2.2)

### Issue: Slow Transcription
**Solution**:
- Reduce beam size (1-2)
- Use smaller model (tiny/base)
- Increase temperature (0.3-0.4)

### Issue: Wrong Language Detected
**Solution**: Set specific language instead of "auto"

### Issue: Poor Accuracy on Technical Terms
**Solution**:
- Use larger model (medium/large-v3)
- Increase beam size (5)
- Consider custom vocabulary (future feature)

## Performance vs Accuracy Trade-offs

| Setting | Accuracy | Speed | Memory |
|---------|----------|-------|--------|
| Beam Size 1 | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Beam Size 3 | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| Beam Size 5 | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐ |
| Tiny Model | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Medium Model | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| Large-v3 Model | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐ |

## Recommended Configurations by Use Case

### Business Meetings (Balanced)
- Model: `medium`
- Beam Size: 3
- Temperature: 0.2
- Language: `auto-translate`
- GPU: Enabled

### Technical Presentations (High Accuracy)
- Model: `large-v3`
- Beam Size: 5
- Temperature: 0.1
- Language: `en` (or specific language)
- GPU: Enabled

### Quick Notes (Fast)
- Model: `base` or `small`
- Beam Size: 1-2
- Temperature: 0.3
- Language: Specific language
- GPU: Optional

### Multilingual Meetings (Auto-detect)
- Model: `medium` or `large-v3`
- Beam Size: 3-5
- Temperature: 0.2
- Language: `auto-translate`
- GPU: Enabled

## Future Enhancements

Potential improvements for transcription accuracy:

1. **Custom Vocabulary**: Add domain-specific terms
2. **Speaker Diarization**: Identify different speakers
3. **Punctuation Model**: Improve punctuation accuracy
4. **Post-processing**: LLM-based correction of transcripts
5. **Real-time Feedback**: Show confidence scores during recording
6. **A/B Testing**: Compare different parameter sets
7. **User Profiles**: Save custom configurations per use case

## Support and Feedback

If you find specific parameter combinations that work well for your use case, please share them with the community!

- GitHub Issues: https://github.com/Zackriya-Solutions/meeting-minutes/issues
- Discord: https://discord.gg/crRymMQBFH

---

**Last Updated**: February 9, 2026  
**Version**: 0.2.0
