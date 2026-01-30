# Setting up Google Gemini for Meetily

## Why Switch to Gemini?

**Memory & CPU Benefits:**
- **Ollama (local)**: Uses 2-4GB+ RAM constantly, high CPU usage
- **Gemini (cloud)**: Uses minimal memory (~50MB), no local processing
- **Performance**: Gemini responses are often faster than local models
- **Quality**: Gemini 1.5 Flash is highly capable for meeting summaries

## Step 1: Get Your Gemini API Key

1. Go to [Google AI Studio](https://aistudio.google.com/app/apikey)
2. Sign in with your Google account
3. Click "Create API Key"
4. Copy the API key (starts with `AIza...`)

## Step 2: Configure Meetily

### Option A: Through the UI (Recommended)
1. Open Meetily
2. Go to Settings
3. Change LLM Provider to "Gemini"
4. Paste your API key in the Gemini API Key field
5. Select model: `gemini-1.5-flash` (recommended for speed and cost)

### Option B: Direct Database Update
If the UI doesn't show Gemini yet, you can update the database directly:

```bash
# Navigate to your Meetily data directory
cd ~/.local/share/com.meetily.ai/

# Update the database
sqlite3 meetily.db "UPDATE settings SET provider='gemini', model='gemini-1.5-flash', geminiApiKey='YOUR_API_KEY_HERE' WHERE id='1';"
```

## Step 3: Stop Ollama (Optional)

Since you won't need Ollama anymore, you can stop it to free up resources:

```bash
# Stop Ollama service
sudo systemctl stop ollama

# Disable Ollama from starting automatically
sudo systemctl disable ollama

# Or just kill the process
pkill ollama
```

## Step 4: Test the Setup

1. Start a new recording in Meetily
2. Generate a summary
3. Check that it uses Gemini (you should see "Gemini" in the logs)

## Memory Usage Comparison

**Before (with Ollama):**
- Ollama: ~3GB RAM
- Meetily: ~200MB RAM
- **Total: ~3.2GB**

**After (with Gemini):**
- Gemini API calls: ~50MB RAM
- Meetily: ~200MB RAM  
- **Total: ~250MB** (92% reduction!)

## Cost Information

Gemini 1.5 Flash pricing (as of 2024):
- Input: $0.075 per 1M tokens
- Output: $0.30 per 1M tokens

For typical meeting summaries:
- 1-hour meeting transcript: ~15,000 tokens
- Summary generation: ~1,000 tokens
- **Cost per meeting: ~$0.001 (less than 1 cent)**

## Troubleshooting

### "Invalid API key" error
- Double-check your API key is correct
- Make sure you copied the full key (starts with `AIza`)
- Verify the key is active in Google AI Studio

### "Quota exceeded" error  
- You may have hit the free tier limit
- Check your usage in Google AI Studio
- Consider upgrading to paid tier if needed

### Gemini not showing in UI
- Make sure you're using the latest version with Gemini support
- Try the direct database update method above