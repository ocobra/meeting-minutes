# Using Gemini for Speaker Identification

**Last Updated**: February 10, 2026  
**Version**: 0.2.0

---

## ✅ Yes, You Can Use Gemini for Speaker Diarization!

**Important Clarification**: Gemini can be used for **speaker name identification** (the LLM part), but NOT for **speaker segmentation** (the audio processing part).

---

## Understanding the Two Components

### 1. Speaker Segmentation (Audio Processing)
**What it does**: Analyzes audio to detect different speakers and segment by voice  
**Technology**: pyannote.audio (specialized audio ML model)  
**Provider**: Hugging Face Inference API  
**Cannot be replaced**: This requires specialized audio processing models

### 2. Speaker Name Identification (LLM Analysis)
**What it does**: Analyzes transcript text to extract speaker names from introductions  
**Technology**: Large Language Model (LLM)  
**Provider**: Gemini, OpenAI, Claude, or others  
**✅ Can use Gemini**: This is just text analysis, perfect for LLMs

---

## How It Works

```
Audio File
    ↓
[1] Speaker Segmentation (Hugging Face/pyannote.audio)
    → Identifies: "Speaker 1", "Speaker 2", "Speaker 3"
    → Segments audio by voice characteristics
    ↓
Transcript + Speaker Labels
    ↓
[2] Name Identification (Gemini/OpenAI/Claude)
    → Analyzes: "Hi, I'm John Smith"
    → Extracts: "Speaker 1" = "John Smith"
    ↓
Final Result: "John Smith: Hi, I'm John Smith"
```

---

## ✅ Gemini Support Added

I've just added Gemini support to the speaker identification router. Gemini is now the **first choice** for speaker name identification.

### Priority Order (Automatic Selection)

1. **Gemini** (if `GEMINI_API_KEY` is set) ← NEW!
2. OpenAI (if `OPENAI_API_KEY` is set)
3. Anthropic Claude (if `ANTHROPIC_API_KEY` is set)
4. Hugging Face (if `HUGGINGFACE_API_KEY` is set)
5. Ollama (local fallback)

---

## Setup Instructions

### Step 1: Get Gemini API Key

1. Go to https://aistudio.google.com/app/apikey
2. Click "Create API Key"
3. Copy the key (starts with `AI...`)

### Step 2: Set Environment Variable

```bash
# Add to ~/.bashrc for persistence
echo 'export GEMINI_API_KEY="YOUR_GEMINI_API_KEY_HERE"' >> ~/.bashrc
source ~/.bashrc

# Verify it's set
echo $GEMINI_API_KEY
```

### Step 3: Restart Meetily

```bash
meetily
```

### Step 4: Verify It's Working

Check the logs to confirm Gemini is being used:

```bash
tail -f ~/.local/share/meetily/logs/*.log | grep -i "gemini"
```

You should see:
```
INFO Using Google Gemini API for speaker identification
```

---

## Benefits of Using Gemini

### 1. Cost Savings
- **Free Tier**: 15 requests/minute (1500/day)
- **Typical Usage**: 1 request per meeting
- **Monthly Cost**: $0 for most users

### 2. Single API Key
- Use the same Gemini API key for:
  - Meeting summaries
  - Speaker name identification
- No need for multiple API keys

### 3. Performance
- Fast response times
- Accurate name extraction
- Good at understanding context

### 4. Resource Savings
- Zero CPU usage (cloud-based)
- Zero RAM usage (no local model)
- Offloads processing from your machine

---

## Complete Recommended Setup

For optimal performance with minimal cost:

```bash
# Speaker segmentation (audio processing)
export HUGGINGFACE_API_KEY="hf_your_token_here"

# Speaker identification + summaries (text analysis)
export GEMINI_API_KEY="YOUR_GEMINI_API_KEY_HERE"
```

**Total Cost**: $0/month (free tiers) or $9/month (HF Pro for unlimited diarization)

---

## Comparison: Gemini vs OpenAI vs Local

| Feature | Gemini | OpenAI | Ollama (Local) |
|---------|--------|--------|----------------|
| **Cost** | Free (15 req/min) | ~$0.01/meeting | Free |
| **Speed** | Fast | Fast | Slow |
| **Accuracy** | Excellent | Excellent | Good |
| **CPU Usage** | 0% | 0% | 40-50% |
| **RAM Usage** | 0 MB | 0 MB | 3.2 GB |
| **Internet** | Required | Required | Not required |
| **Privacy** | Cloud | Cloud | Local |

**Recommendation**: Use Gemini for best balance of cost, performance, and accuracy.

---

## What You CANNOT Use Gemini For

### ❌ Speaker Segmentation (Audio Processing)

Gemini cannot replace Hugging Face for speaker segmentation because:

1. **Not an audio model**: Gemini is a text/multimodal LLM, not an audio processing model
2. **No voice embeddings**: Cannot extract voice characteristics (d-vectors, x-vectors)
3. **No speaker clustering**: Cannot group audio segments by voice similarity
4. **Different task**: Audio segmentation requires specialized ML models like pyannote.audio

**For speaker segmentation, you must use**:
- Hugging Face Inference API (pyannote.audio) - Recommended
- Local pyannote.audio (fallback)

---

## Testing Gemini Integration

### Test 1: Check API Key Priority

```bash
# Set only Gemini key
export GEMINI_API_KEY="your_key"
unset OPENAI_API_KEY
unset ANTHROPIC_API_KEY

# Start Meetily and check logs
meetily &
tail -f ~/.local/share/meetily/logs/*.log | grep "Using.*API for speaker identification"
```

Expected output:
```
INFO Using Google Gemini API for speaker identification
```

### Test 2: Record a Meeting

1. Start a recording
2. Speak introductions: "Hi, I'm John Smith"
3. Stop recording
4. Check if names are extracted correctly

### Test 3: Verify Fallback

```bash
# Unset Gemini key to test fallback
unset GEMINI_API_KEY

# Restart Meetily - should fall back to next available provider
```

---

## Troubleshooting

### Issue: Gemini Not Being Used

**Check 1**: Verify API key is set
```bash
echo $GEMINI_API_KEY
```

**Check 2**: Check logs for errors
```bash
tail -f ~/.local/share/meetily/logs/*.log | grep -i "gemini\|llm\|identification"
```

**Check 3**: Verify internet connectivity
```bash
ping generativelanguage.googleapis.com
```

### Issue: Names Not Being Extracted

**Possible Causes**:
1. Speakers didn't introduce themselves clearly
2. Confidence threshold too high
3. LLM response parsing failed

**Solutions**:
1. Ensure clear introductions: "I'm [name]", "This is [name]"
2. Lower confidence threshold in Settings → Speakers
3. Check logs for parsing errors

### Issue: API Quota Exceeded

**Error**: "quota exceeded" or "rate limit"

**Solution**:
- Gemini free tier: 15 requests/minute
- Wait a minute and try again
- Or upgrade to paid tier (if needed)

---

## Code Changes Made

### File: `frontend/src-tauri/src/diarization/router.rs`

Added Gemini as the first priority in `get_external_llm_model()`:

```rust
// 1. Gemini (Google AI)
if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
    info!("Using Google Gemini API for speaker identification");
    return Ok(ModelChoice::External {
        endpoint: "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent".to_string(),
        api_key: Some(api_key),
    });
}
```

### Files Updated:
- ✅ `frontend/src-tauri/src/diarization/router.rs` - Added Gemini support
- ✅ `EXTERNAL_MODELS_AND_AUTH_SETUP.md` - Updated documentation
- ✅ `MEETILY_OPTIMAL_SETTINGS_GUIDE.md` - Updated recommendations
- ✅ `QUICK_OPTIMIZATION_REFERENCE.md` - Updated quick reference

---

## Summary

### ✅ What You CAN Do
- Use Gemini for **speaker name identification** (extracting names from transcripts)
- Use the same Gemini API key for summaries and speaker identification
- Save money with Gemini's generous free tier
- Reduce CPU/RAM usage by offloading to cloud

### ❌ What You CANNOT Do
- Use Gemini for **speaker segmentation** (audio processing)
- Replace Hugging Face pyannote.audio with Gemini
- Process audio files directly with Gemini

### 🎯 Recommended Setup
```bash
# For speaker segmentation (audio)
export HUGGINGFACE_API_KEY="hf_..."

# For speaker identification + summaries (text)
export GEMINI_API_KEY="your_gemini_key"
```

**Result**: Best performance, lowest cost, minimal resource usage!

---

**Questions?** Check the logs or refer to:
- `EXTERNAL_MODELS_AND_AUTH_SETUP.md` - Complete external models guide
- `MEETILY_OPTIMAL_SETTINGS_GUIDE.md` - Optimization guide
- `SPEAKER_DIARIZATION_TESTING_GUIDE.md` - Testing guide

---

**Last Updated**: February 10, 2026  
**Version**: 0.2.0  
**Status**: ✅ Gemini support added and ready to use!
