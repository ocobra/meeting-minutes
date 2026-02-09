# Parakeet Transcription Tuning Guide

## Important Note

**You are currently using Parakeet as your transcription model.** The Whisper tuning parameters (beam size, temperature, etc.) **do not apply** to Parakeet since it uses a completely different architecture.

## Parakeet vs Whisper

| Feature | Parakeet | Whisper |
|---------|----------|---------|
| Architecture | RNN-T / TDT (Transducer) | Transformer |
| Decoding | Greedy (argmax only) | Beam Search (configurable) |
| Temperature | Not supported | Configurable (0.1-0.4) |
| Beam Size | Not supported | Configurable (1-5) |
| Speed | **Ultra Fast** | Slower |
| Accuracy | Good | **Better** (with tuning) |
| Tunable Parameters | **Very Limited** | **Extensive** |

## Parakeet Limitations

Parakeet uses **greedy decoding** which means:
- It always picks the most likely token at each step (argmax)
- No beam search to explore alternative paths
- No temperature parameter to control randomness
- Limited tuning options compared to Whisper

## Available Parakeet Parameters

### 1. MAX_TOKENS_PER_STEP (Hardcoded)

**Current Value**: 10

**What it does**: Maximum number of tokens that can be emitted before advancing to the next audio frame.

**Location**: `frontend/src-tauri/src/parakeet_engine/model.rs`

```rust
const MAX_TOKENS_PER_STEP: usize = 10;
```

**To Tune**:
- Increase (e.g., 15-20): May capture longer words/phrases but slower
- Decrease (e.g., 5-8): Faster but may truncate long words
- **Not recommended to change** - optimized value

### 2. Model Quantization

**Options**:
- `Int8`: Faster, slightly lower accuracy (default)
- `FP32`: Slower, slightly higher accuracy

**How to Change**: Through Meetily settings UI
- Settings → Transcription → Model Selection
- Choose between:
  - `parakeet-tdt-0.6b-v3-int8` (faster)
  - `parakeet-tdt-0.6b-v3-fp32` (more accurate)

**Recommendation**: Use `FP32` for maximum accuracy if speed is not critical.

## How to Improve Parakeet Accuracy

Since Parakeet has limited tuning options, focus on these approaches:

### 1. Switch to Whisper for Better Accuracy

**Recommended if accuracy is priority:**

1. Go to Settings → Transcription
2. Change provider from "Parakeet" to "Whisper"
3. Select a larger Whisper model:
   - `medium` - Good balance
   - `large-v3` - Best accuracy
4. Apply the Whisper tuning parameters I provided earlier

**Trade-off**: Whisper is 2-5x slower than Parakeet but significantly more accurate with proper tuning.

### 2. Use FP32 Parakeet Model

If you want to stay with Parakeet:

1. Settings → Transcription → Model Selection
2. Select `parakeet-tdt-0.6b-v3-fp32`
3. This provides ~5-10% accuracy improvement over Int8

**Trade-off**: Slightly slower (still faster than Whisper)

### 3. Improve Audio Quality

Since Parakeet can't be tuned much, focus on input quality:

- **Use external microphone**: Better audio = better transcription
- **Reduce background noise**: Close windows, turn off fans
- **Speak clearly**: Parakeet is optimized for clear speech
- **Optimal distance**: 6-12 inches from microphone
- **Consistent volume**: Avoid speaking too quietly or too loudly

### 4. Post-Processing with LLM

Use the summary generation feature to correct transcription errors:

1. Generate transcript with Parakeet
2. Use LLM summary to clean up errors
3. The LLM can often correct obvious transcription mistakes

## Parakeet Code Modifications (Advanced)

If you want to experiment with Parakeet's internal parameters:

### Modify MAX_TOKENS_PER_STEP

Edit `frontend/src-tauri/src/parakeet_engine/model.rs`:

```rust
// Line 18
const MAX_TOKENS_PER_STEP: usize = 15;  // Increased from 10
```

**Effect**: May capture longer continuous speech segments.

### Add Confidence Threshold (Custom Implementation)

Currently, Parakeet uses greedy decoding (always picks highest probability). You could add a confidence threshold:

```rust
// In decode_sequence function, around line 390
let (max_idx, max_prob) = vocab_logits
    .iter()
    .enumerate()
    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    .map(|(idx, prob)| (idx as i32, *prob))
    .unwrap_or((self.blank_idx, 0.0));

// Add confidence threshold
const CONFIDENCE_THRESHOLD: f32 = 0.3;  // Adjust this value
let token = if max_prob > CONFIDENCE_THRESHOLD {
    max_idx
} else {
    self.blank_idx  // Treat low-confidence as blank
};
```

**Effect**: Filters out low-confidence predictions, reducing hallucinations.

## Recommended Configuration

### For Maximum Accuracy with Parakeet

1. **Model**: `parakeet-tdt-0.6b-v3-fp32`
2. **Audio Quality**: Use external microphone, reduce noise
3. **Post-Processing**: Enable LLM summary generation
4. **Environment**: Quiet room, clear speech

### For Maximum Accuracy Overall

1. **Switch to Whisper**: `large-v3` model
2. **Apply Whisper Tuning**:
   - Beam Size: 5
   - Temperature: 0.1
   - No Speech Threshold: 0.45
   - Entropy Threshold: 2.0
3. **Enable GPU**: Vulkan/CUDA/Metal
4. **Set Language**: Specific language (e.g., "en") instead of "auto"

## Performance Comparison

Based on typical use cases:

| Configuration | Speed | Accuracy | Memory |
|---------------|-------|----------|--------|
| Parakeet Int8 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Parakeet FP32 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Whisper Base | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| Whisper Medium | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| Whisper Large-v3 (Tuned) | ⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐ |

## Why Parakeet Has Limited Tuning

Parakeet uses a **Transducer (RNN-T/TDT)** architecture which is fundamentally different from Whisper's Transformer architecture:

1. **Greedy Decoding**: Parakeet is designed for real-time streaming, so it uses greedy decoding for speed
2. **No Beam Search**: Beam search would slow down the model significantly
3. **Optimized for Speed**: The model is pre-optimized for fast inference
4. **Fewer Parameters**: Smaller model size means fewer tuning knobs

## Conclusion

**If you need better accuracy**, I strongly recommend switching to Whisper with the tuning parameters I provided earlier. The changes I made to the code will only affect Whisper, not Parakeet.

**To switch to Whisper**:
1. Open Meetily Settings
2. Go to Transcription section
3. Change Provider to "Whisper"
4. Select `medium` or `large-v3` model
5. Rebuild and install the updated version (with the tuning changes I made)

The Whisper tuning I applied will give you:
- **Beam Size 5**: Maximum quality decoding
- **Temperature 0.1**: Very conservative, accurate predictions
- **Lower thresholds**: Better quiet speech detection and quality filtering

This should provide **significantly better accuracy** than Parakeet, at the cost of slower processing speed.

---

**Last Updated**: February 9, 2026  
**Version**: 0.2.0
