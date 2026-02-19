# External Models and Authentication Setup

## Overview

The speaker diarization feature uses a **hybrid approach** with external/cloud models as the primary option and local models as fallback. This document explains what external models are used and how to configure authentication.

---

## External Models Used

### 1. Speaker Diarization Model

**Primary Model**: Hugging Face Inference API - `pyannote/speaker-diarization-3.1`

- **Endpoint**: `https://api-inference.huggingface.co/models/pyannote/speaker-diarization-3.1`
- **Purpose**: Segments audio by speaker, extracts voice embeddings
- **Fallback**: Local pyannote.audio model (runs on your machine)

### 2. Speaker Identification Model (LLM)

**Multiple Options** (in order of preference):

1. **Google Gemini** (Recommended)
   - **Endpoint**: `https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent`
   - **Purpose**: Extracts speaker names from transcript introductions
   - **Requires**: `GEMINI_API_KEY`
   - **Benefits**: Fast, accurate, generous free tier (15 req/min)

2. **OpenAI GPT** (GPT-3.5, GPT-4)
   - **Endpoint**: `https://api.openai.com/v1/chat/completions`
   - **Purpose**: Extracts speaker names from transcript introductions
   - **Requires**: `OPENAI_API_KEY`

3. **Anthropic Claude**
   - **Endpoint**: `https://api.anthropic.com/v1/messages`
   - **Purpose**: Extracts speaker names from transcript introductions
   - **Requires**: `ANTHROPIC_API_KEY`

4. **Hugging Face Inference API** (Llama-2-7b-chat)
   - **Endpoint**: `https://api-inference.huggingface.co/models/meta-llama/Llama-2-7b-chat-hf`
   - **Purpose**: Extracts speaker names from transcript introductions
   - **Requires**: `HUGGINGFACE_API_KEY` or `HF_TOKEN`

5. **Ollama** (Local LLM - Default Fallback)
   - **Endpoint**: `http://localhost:11434`
   - **Model**: `llama3.2:latest`
   - **Purpose**: Local LLM for speaker identification
   - **Requires**: Ollama installed and running locally

---

## Authentication Setup

### Current Status: ⚠️ NO API KEYS CONFIGURED

The application is currently configured to use **local models only** because no external API keys are set up. This means:

- ✅ Diarization works (using local pyannote.audio)
- ✅ Name identification works (using local Ollama LLM)
- ❌ External models are NOT being used
- ❌ No cloud API calls are being made

### How to Enable External Models

To use external models for better accuracy and performance, you need to set up API keys:

#### Option 1: Google Gemini (Recommended for Name Identification)

1. **Get API Key**:
   - Go to https://aistudio.google.com/app/apikey
   - Create a new API key

2. **Set Environment Variable**:
   ```bash
   export GEMINI_API_KEY="YOUR_GEMINI_API_KEY_HERE"
   ```

3. **Restart Application**:
   ```bash
   meetily
   ```

**Benefits**:
- Free tier: 15 requests/minute (1500/day)
- Fast and accurate
- Same API key used for summaries and speaker identification

#### Option 2: Hugging Face (Recommended for Diarization)

1. **Get API Key**:
   - Go to https://huggingface.co/settings/tokens
   - Create a new token with "Read" access
   - Accept the pyannote.audio model terms: https://huggingface.co/pyannote/speaker-diarization-3.1

2. **Set Environment Variable**:
   ```bash
   export HUGGINGFACE_API_KEY="hf_your_token_here"
   # OR
   export HF_TOKEN="hf_your_token_here"
   ```

3. **Restart Application**:
   ```bash
   meetily
   ```

#### Option 2: Hugging Face (Recommended for Diarization)

1. **Get API Key**:
   - Go to https://huggingface.co/settings/tokens
   - Create a new token with "Read" access
   - Accept the pyannote.audio model terms: https://huggingface.co/pyannote/speaker-diarization-3.1

2. **Set Environment Variable**:
   ```bash
   export HUGGINGFACE_API_KEY="hf_your_token_here"
   # OR
   export HF_TOKEN="hf_your_token_here"
   ```

3. **Restart Application**:
   ```bash
   meetily
   ```

#### Option 3: OpenAI (Alternative for Name Identification)

1. **Get API Key**:
   - Go to https://platform.openai.com/api-keys
   - Create a new API key

2. **Set Environment Variable**:
   ```bash
   export OPENAI_API_KEY="sk-your_key_here"
   ```

3. **Restart Application**:
   ```bash
   meetily
   ```

#### Option 3: OpenAI (Alternative for Name Identification)

1. **Get API Key**:
   - Go to https://platform.openai.com/api-keys
   - Create a new API key

2. **Set Environment Variable**:
   ```bash
   export OPENAI_API_KEY="sk-your_key_here"
   ```

3. **Restart Application**:
   ```bash
   meetily
   ```

#### Option 4: Anthropic Claude

1. **Get API Key**:
   - Go to https://console.anthropic.com/settings/keys
   - Create a new API key

2. **Set Environment Variable**:
   ```bash
   export ANTHROPIC_API_KEY="YOUR_ANTHROPIC_API_KEY_HERE"
   ```

3. **Restart Application**:
   ```bash
   meetily
   ```

#### Option 4: Anthropic Claude

1. **Get API Key**:
   - Go to https://console.anthropic.com/settings/keys
   - Create a new API key

2. **Set Environment Variable**:
   ```bash
   export ANTHROPIC_API_KEY="YOUR_ANTHROPIC_API_KEY_HERE"
   ```

3. **Restart Application**:
   ```bash
   meetily
   ```

#### Option 5: Use Local Models Only (Current Setup)

No configuration needed! The application automatically falls back to:
- **Diarization**: Local pyannote.audio (CPU-based)
- **Identification**: Local Ollama LLM (requires Ollama installed)

---

## Privacy Modes

The application supports three privacy modes (configured in Settings → Speakers):

### 1. LocalOnly (Maximum Privacy)
- **Behavior**: Never uses external APIs
- **Processing**: All on your machine
- **Requires**: No API keys needed
- **Performance**: Slower, uses more CPU/RAM
- **Privacy**: Maximum - no data leaves your machine

### 2. PreferExternal (Recommended - Default)
- **Behavior**: Tries external APIs first, falls back to local
- **Processing**: Cloud when available, local as fallback
- **Requires**: API keys for external models (optional)
- **Performance**: Fastest when API keys configured
- **Privacy**: Balanced - data sent to cloud only when available

### 3. ExternalOnly (Maximum Accuracy)
- **Behavior**: Requires external APIs, fails if unavailable
- **Processing**: Cloud only
- **Requires**: API keys for external models (required)
- **Performance**: Fastest and most accurate
- **Privacy**: Minimum - all data sent to cloud

---

## How the Model Router Works

The Model Router automatically selects which model to use:

```
1. Check Privacy Mode
   ├─ LocalOnly? → Use local models
   ├─ ExternalOnly? → Require external models
   └─ PreferExternal? → Continue to step 2

2. Check Internet Connectivity
   ├─ No internet? → Use local models
   └─ Internet available? → Continue to step 3

3. Check API Keys
   ├─ No API keys? → Use local models
   └─ API keys found? → Continue to step 4

4. Try External API
   ├─ Success? → Use external models
   └─ Failed? → Fall back to local models

5. Cache Decision (5 minutes)
```

---

## Checking Current Configuration

### View Logs

Check which models are being used:

```bash
# View application logs
tail -f ~/.local/share/meetily/logs/*.log | grep -E "(ModelRouter|diarization|identification)"
```

### Expected Log Messages

**When using local models** (current setup):
```
INFO ModelRouter: Privacy mode: PreferExternal - trying external diarization model first
WARN ModelRouter: No Hugging Face API key found in environment (HUGGINGFACE_API_KEY or HF_TOKEN)
WARN ModelRouter: Failed to connect to external diarization model. Falling back to local model
INFO ModelRouter: Using local diarization model
```

**When using external models** (with API keys):
```
INFO ModelRouter: Privacy mode: PreferExternal - trying external diarization model first
INFO ModelRouter: Successfully connected to external diarization model
INFO ModelRouter: Using external diarization model
```

---

## Cost Considerations

### External API Costs

If you configure external API keys, be aware of potential costs:

#### Hugging Face Inference API
- **Free Tier**: Limited requests per month
- **Pro Tier**: $9/month for more requests
- **Pricing**: https://huggingface.co/pricing

#### OpenAI API
- **GPT-3.5-turbo**: ~$0.002 per 1K tokens
- **GPT-4**: ~$0.03 per 1K tokens
- **Typical cost per meeting**: $0.01 - $0.10 (depending on length)
- **Pricing**: https://openai.com/pricing

#### Anthropic Claude
- **Claude 3 Haiku**: ~$0.25 per million tokens
- **Claude 3 Sonnet**: ~$3 per million tokens
- **Typical cost per meeting**: $0.01 - $0.05
- **Pricing**: https://www.anthropic.com/pricing

### Local Processing (Free)

Using local models has **zero API costs** but requires:
- More CPU/RAM usage
- Longer processing time
- Python environment with pyannote.audio
- Ollama installed for LLM

---

## Recommendations

### For Testing (Current Setup)
✅ **Use local models** - No API keys needed, works out of the box

### For Production Use
✅ **Configure Hugging Face + OpenAI**:
```bash
export HUGGINGFACE_API_KEY="hf_your_token_here"
export OPENAI_API_KEY="sk-your_key_here"
```

Benefits:
- Faster processing (5-10x speedup)
- Better accuracy
- Lower CPU/RAM usage on your machine
- Minimal cost (~$0.05 per hour of meeting audio)

### For Maximum Privacy
✅ **Use LocalOnly mode** - Set in Settings → Speakers → Privacy Mode

---

## Troubleshooting

### Issue: "No external models available"

**Cause**: No API keys configured

**Solution**: Set up API keys as described above, or use LocalOnly mode

### Issue: "External API timeout"

**Cause**: Slow internet connection or API service issues

**Solution**: 
1. Check internet connection
2. Try again later
3. Switch to LocalOnly mode temporarily

### Issue: "Hugging Face model access denied"

**Cause**: Haven't accepted pyannote.audio model terms

**Solution**:
1. Go to https://huggingface.co/pyannote/speaker-diarization-3.1
2. Click "Agree and access repository"
3. Restart application

---

## Summary

### Current Setup
- ✅ Local diarization: pyannote.audio (CPU)
- ✅ Local identification: Ollama LLM
- ❌ External models: Not configured (no API keys)
- 📍 Privacy Mode: PreferExternal (will use local as fallback)

### To Enable External Models
1. Get Gemini API key: https://aistudio.google.com/app/apikey
2. Get Hugging Face API key: https://huggingface.co/settings/tokens
3. Set environment variables:
   ```bash
   export GEMINI_API_KEY="YOUR_GEMINI_API_KEY_HERE"
   export HUGGINGFACE_API_KEY="YOUR_HUGGINGFACE_API_KEY_HERE"
   ```
4. Restart Meetily

### Benefits of External Models
- 🚀 5-10x faster processing
- 🎯 Better accuracy
- 💻 Lower CPU/RAM usage
- 💰 Minimal cost (~$0/month with free tiers)

---

**Last Updated**: February 10, 2026  
**Application Version**: 0.2.0
