# Meetily Gemini Auto-Update Guide

This guide shows you how to configure Meetily to always use the latest Google Gemini models for summarization.

## 🚀 Quick Setup

### Option 1: Use Latest Model Alias (Recommended)
```bash
./switch_to_gemini.sh
```
This configures Meetily to use `gemini-flash-latest`, which automatically points to Google's latest Flash model.

### Option 2: Auto-Discovery Script
```bash
./auto_update_gemini.sh
```
This script automatically detects and configures the latest available Gemini model.

## 🔄 Staying Updated

### Method 1: Use Google's "Latest" Aliases
Google provides model aliases that automatically resolve to the latest versions:
- `gemini-flash-latest` → Latest Flash model (fast, cost-effective)
- `gemini-pro-latest` → Latest Pro model (highest quality)

**Advantage**: Automatic updates without any action needed
**Current Configuration**: `gemini-flash-latest`

### Method 2: Periodic Auto-Updates
Run the auto-update script monthly or when you want the absolute latest:
```bash
./auto_update_gemini.sh
```

### Method 3: Manual Updates
Check for new models and update manually:
```bash
# List available models
curl -s "https://generativelanguage.googleapis.com/v1beta/models?key=YOUR_API_KEY" | grep gemini

# Update database manually
sqlite3 ~/.local/share/com.meetily.ai/meeting_minutes.sqlite \
  "UPDATE settings SET model='NEW_MODEL_NAME' WHERE id='1';"
```

## 📊 Current Configuration

**Provider**: gemini  
**Model**: gemini-flash-latest  
**API Key**: YOUR_GEMINI_API_KEY_HERE  
**Endpoint**: https://generativelanguage.googleapis.com/v1beta/models/

## 🎯 Model Selection Guide

| Model | Speed | Quality | Cost | Use Case |
|-------|-------|---------|------|----------|
| `gemini-flash-latest` | ⚡ Fast | ✅ Good | 💰 Low | Meeting summaries (recommended) |
| `gemini-pro-latest` | 🐌 Slower | 🌟 Excellent | 💰💰 Higher | Complex analysis |
| `gemini-2.5-flash` | ⚡ Fast | ✅ Good | 💰 Low | Specific version |

## 🔧 Troubleshooting

### Model Not Found Error
If you get a "model not found" error:
1. Run `./auto_update_gemini.sh` to get the latest model
2. Or manually update to `gemini-flash-latest`

### API Authentication Error
1. Verify your API key is valid at https://aistudio.google.com/app/apikey
2. Check the key in database: `sqlite3 ~/.local/share/com.meetily.ai/meeting_minutes.sqlite "SELECT geminiApiKey FROM settings;"`

### Performance Optimization
- **For speed**: Use `gemini-flash-latest`
- **For quality**: Use `gemini-pro-latest`
- **For cost**: Stick with Flash models

## 🎉 Benefits of Auto-Updates

✅ **Always Latest**: Automatically get Google's newest models  
✅ **Better Performance**: New models are typically faster and more accurate  
✅ **New Features**: Access to latest capabilities as they're released  
✅ **Cost Optimization**: Newer models often provide better value  
✅ **Zero Maintenance**: Set it once, forget about it  

## 💡 Pro Tips

1. **Use `gemini-flash-latest`** for automatic updates without scripts
2. **Run auto-update monthly** to catch any new model releases
3. **Monitor costs** at https://console.cloud.google.com/billing
4. **Test new models** with short recordings before important meetings
5. **Keep backups** of working configurations

Expected memory savings: ~3GB (from Ollama removal)  
Cost per meeting summary: <$0.001 (less than 1 cent)  

If you have issues, check the logs or refer to this guide.