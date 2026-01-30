#!/bin/bash

# Meetily Auto-Update Gemini Model Script
# This script automatically detects and configures the latest Gemini model

echo "🔄 Auto-updating Meetily to use the latest Gemini model"
echo "=================================================="
echo

# Check if Meetily database exists
MEETILY_DB="$HOME/.local/share/com.meetily.ai/meeting_minutes.sqlite"
if [ ! -f "$MEETILY_DB" ]; then
    echo "❌ Meetily database not found at: $MEETILY_DB"
    echo "   Please run Meetily at least once to create the database."
    exit 1
fi

# Get current API key from database
API_KEY=$(sqlite3 "$MEETILY_DB" "SELECT geminiApiKey FROM settings WHERE id='1';" 2>/dev/null)

if [ -z "$API_KEY" ]; then
    echo "❌ No Gemini API key found in database."
    echo "   Please run the initial setup first: ./switch_to_gemini.sh"
    exit 1
fi

echo "✅ Found existing Gemini API key"

# Function to get latest model
get_latest_model() {
    # Try gemini-flash-latest first (recommended)
    if curl -s "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key=$API_KEY" \
        -H 'Content-Type: application/json' \
        -d '{"contents":[{"parts":[{"text":"test"}]}]}' \
        --max-time 10 > /dev/null 2>&1; then
        echo "gemini-flash-latest"
        return 0
    fi
    
    # Fallback: Get the latest numbered model
    LATEST_MODEL=$(curl -s "https://generativelanguage.googleapis.com/v1beta/models?key=$API_KEY" \
        --max-time 10 | \
        grep -o '"name": "models/gemini-[0-9.]*-flash"' | \
        sed 's/"name": "models\///' | \
        sed 's/"//' | \
        sort -V | \
        tail -1)
    
    if [ -n "$LATEST_MODEL" ]; then
        echo "$LATEST_MODEL"
        return 0
    fi
    
    # Final fallback
    echo "gemini-2.5-flash"
}

# Get the latest model
echo "🔍 Discovering latest Gemini models..."
LATEST_MODEL=$(get_latest_model)
echo "🎯 Latest model detected: $LATEST_MODEL"

# Get current model from database
CURRENT_MODEL=$(sqlite3 "$MEETILY_DB" "SELECT model FROM settings WHERE id='1';" 2>/dev/null)

if [ "$CURRENT_MODEL" = "$LATEST_MODEL" ]; then
    echo "✅ Already using the latest model: $LATEST_MODEL"
    echo "   No update needed."
else
    echo "🔄 Updating from '$CURRENT_MODEL' to '$LATEST_MODEL'"
    
    # Update database
    sqlite3 "$MEETILY_DB" "UPDATE settings SET model='$LATEST_MODEL' WHERE id='1';"
    
    if [ $? -eq 0 ]; then
        echo "✅ Successfully updated to $LATEST_MODEL!"
    else
        echo "❌ Failed to update database"
        exit 1
    fi
fi

# Show current configuration
echo
echo "📊 Current Meetily Gemini Configuration:"
echo "   Provider: gemini"
echo "   Model: $LATEST_MODEL"
echo "   API Key: $(echo $API_KEY | sed 's/./*/g' | sed 's/\(.\{4\}\).*\(.\{4\}\)/\1****\2/')"

echo
echo "🎉 Meetily is now configured to use the latest Gemini model!"
echo
echo "💡 Pro tip: Run this script periodically to stay updated:"
echo "   ./auto_update_gemini.sh"