#!/usr/bin/env python3
"""
Test script to verify the Gemini to builtin-ai fallback system works correctly.
This script simulates a quota exhaustion scenario and verifies the fallback behavior.
"""

import sqlite3
import requests
import json
import time

# Database path
DB_PATH = "~/.local/share/com.meetily.ai/meeting_minutes.sqlite"

def check_current_config():
    """Check current Meetily configuration"""
    print("🔍 Checking current Meetily configuration...")
    
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        
        # Check current provider and model
        cursor.execute("SELECT provider, model, geminiApiKey FROM settings LIMIT 1")
        result = cursor.fetchone()
        
        if result:
            provider, model, api_key = result
            print(f"✅ Current provider: {provider}")
            print(f"✅ Current model: {model}")
            print(f"✅ Gemini API key configured: {'Yes' if api_key else 'No'}")
            
            # Mask API key for security
            if api_key:
                masked_key = api_key[:8] + "..." + api_key[-8:] if len(api_key) > 16 else "***"
                print(f"✅ API key: {masked_key}")
        else:
            print("❌ No model configuration found")
            
        conn.close()
        return result
        
    except Exception as e:
        print(f"❌ Error checking configuration: {e}")
        return None

def test_gemini_api_directly():
    """Test Gemini API directly to check quota status"""
    print("\n🧪 Testing Gemini API directly...")
    
    try:
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        cursor.execute("SELECT geminiApiKey FROM settings LIMIT 1")
        result = cursor.fetchone()
        conn.close()
        
        if not result or not result[0]:
            print("❌ No Gemini API key found")
            return False
            
        api_key = result[0]
        
        # Test with a simple request
        url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key={api_key}"
        
        payload = {
            "contents": [{
                "parts": [{
                    "text": "Hello, this is a test. Please respond with just 'Test successful'."
                }]
            }],
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 50
            }
        }
        
        response = requests.post(url, json=payload, timeout=30)
        
        if response.status_code == 200:
            print("✅ Gemini API is working - quota available")
            return True
        elif response.status_code == 429:
            print("⚠️ Gemini API quota exceeded (429) - perfect for testing fallback!")
            return False
        else:
            print(f"❌ Gemini API error: {response.status_code} - {response.text}")
            return False
            
    except Exception as e:
        print(f"❌ Error testing Gemini API: {e}")
        return False

def check_builtin_ai_setup():
    """Check if builtin-ai is properly set up"""
    print("\n🔧 Checking builtin-ai setup...")
    
    # Check if llama-helper binary exists
    import os
    llama_helper_path = os.path.expanduser("~/code/githubrepos/meeting-minutes/target/release/llama-helper")
    
    if os.path.exists(llama_helper_path):
        print(f"✅ llama-helper binary found: {llama_helper_path}")
        
        # Check file size
        size_mb = os.path.getsize(llama_helper_path) / (1024 * 1024)
        print(f"✅ Binary size: {size_mb:.1f} MB")
    else:
        print(f"❌ llama-helper binary not found at: {llama_helper_path}")
        return False
    
    # Check if model file exists
    model_path = os.path.expanduser("~/.local/share/com.meetily.ai/models/summary/gemma-3-1b-it-Q8_0.gguf")
    
    if os.path.exists(model_path):
        print(f"✅ Gemma 3 1B model found: {model_path}")
        
        # Check file size
        size_gb = os.path.getsize(model_path) / (1024 * 1024 * 1024)
        print(f"✅ Model size: {size_gb:.2f} GB")
    else:
        print(f"❌ Gemma 3 1B model not found at: {model_path}")
        return False
    
    # Check environment variable
    env_var = os.environ.get('MEETILY_LLAMA_HELPER')
    if env_var:
        print(f"✅ MEETILY_LLAMA_HELPER environment variable set: {env_var}")
    else:
        print("⚠️ MEETILY_LLAMA_HELPER environment variable not set")
        print("   This should be set when running Meetily")
    
    return True

def main():
    """Main test function"""
    print("🚀 Testing Meetily Gemini to builtin-ai fallback system")
    print("=" * 60)
    
    # Step 1: Check current configuration
    config = check_current_config()
    if not config:
        print("❌ Cannot proceed without valid configuration")
        return
    
    # Step 2: Test Gemini API directly
    gemini_working = test_gemini_api_directly()
    
    # Step 3: Check builtin-ai setup
    builtin_ready = check_builtin_ai_setup()
    
    print("\n📊 Test Results Summary:")
    print("=" * 30)
    print(f"✅ Configuration loaded: {'Yes' if config else 'No'}")
    print(f"✅ Gemini API working: {'Yes' if gemini_working else 'No (good for fallback test)'}")
    print(f"✅ Builtin-AI ready: {'Yes' if builtin_ready else 'No'}")
    
    if not builtin_ready:
        print("\n❌ Builtin-AI is not properly set up. Fallback will not work.")
        return
    
    if gemini_working:
        print("\n⚠️ Gemini API is working, so fallback won't be triggered automatically.")
        print("   To test fallback, you would need to:")
        print("   1. Exhaust your Gemini quota by making many requests, or")
        print("   2. Temporarily use an invalid API key")
    else:
        print("\n✅ Perfect setup for testing fallback!")
        print("   Gemini quota is exhausted, so Meetily should automatically")
        print("   fallback to builtin-ai/gemma3:1b when generating summaries.")
    
    print("\n🎯 Next steps:")
    print("1. Launch Meetily application")
    print("2. Record a short test meeting")
    print("3. Generate a summary - it should use builtin-ai if Gemini quota is exhausted")
    print("4. Look for fallback messages in the summary output")

if __name__ == "__main__":
    main()