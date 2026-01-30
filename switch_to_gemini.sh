#!/bin/bash

# Meetily Gemini Setup Script
# This script helps you switch from Ollama to Google Gemini

echo "🚀 Meetily Gemini Setup"
echo "======================="
echo

# Check if Meetily database exists
MEETILY_DB="$HOME/.local/share/com.meetily.ai/meeting_minutes.sqlite"
if [ ! -f "$MEETILY_DB" ]; then
    echo "❌ Meetily database not found at: $MEETILY_DB"
    echo "   Please run Meetily at least once to create the database."
    exit 1
fi

echo "✅ Found Meetily database"

# Get API key from user
echo
echo "📝 Please enter your Gemini API key:"
echo "   (Get it from: https://aistudio.google.com/app/apikey)"
read -p "API Key: " API_KEY

if [ -z "$API_KEY" ]; then
    echo "❌ No API key provided. Exiting."
    exit 1
fi

# Validate API key format (basic check)
if [[ ! $API_KEY =~ ^AIza[A-Za-z0-9_-]{35}$ ]]; then
    echo "⚠️  Warning: API key format doesn't look correct (should start with 'AIza')"
    read -p "Continue anyway? (y/N): " CONTINUE
    if [[ ! $CONTINUE =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Update database
echo
echo "🔧 Updating Meetily configuration..."

sqlite3 "$MEETILY_DB" "UPDATE settings SET provider='gemini', model='gemini-flash-latest', geminiApiKey='$API_KEY' WHERE id='1';"

if [ $? -eq 0 ]; then
    echo "✅ Successfully configured Gemini!"
else
    echo "❌ Failed to update database"
    exit 1
fi

# Check if Ollama is running
echo
echo "🔍 Checking Ollama status..."

if pgrep -x "ollama" > /dev/null; then
    echo "🟡 Ollama is currently running"
    read -p "Would you like to stop Ollama to free up memory? (y/N): " STOP_OLLAMA
    
    if [[ $STOP_OLLAMA =~ ^[Yy]$ ]]; then
        echo "🛑 Stopping Ollama..."
        pkill ollama
        
        # Check if it's a systemd service
        if systemctl is-active --quiet ollama 2>/dev/null; then
            echo "🛑 Stopping Ollama service..."
            sudo systemctl stop ollama
            
            read -p "Disable Ollama from starting automatically? (y/N): " DISABLE_OLLAMA
            if [[ $DISABLE_OLLAMA =~ ^[Yy]$ ]]; then
                sudo systemctl disable ollama
                echo "✅ Ollama disabled from auto-start"
            fi
        fi
        
        echo "✅ Ollama stopped"
    fi
else
    echo "✅ Ollama is not running"
fi

# Show memory usage
echo
echo "📊 Current memory usage:"
free -h | grep -E "Mem:|Swap:"

echo
echo "🎉 Setup complete!"
echo
echo "Next steps:"
echo "1. Start Meetily"
echo "2. Create a test recording"
echo "3. Generate a summary to test Gemini"
echo
echo "Expected memory savings: ~3GB (from Ollama removal)"
echo "Cost per meeting summary: <$0.001 (less than 1 cent)"
echo
echo "If you have issues, check the logs or refer to setup_gemini.md"