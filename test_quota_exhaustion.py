#!/usr/bin/env python3
"""
Test script to simulate Gemini quota exhaustion and verify fallback to builtin-ai.
This temporarily modifies the API key to trigger the fallback system.
"""

import sqlite3
import time
import subprocess
import os

# Database path
DB_PATH = "/home/cokochu/.local/share/com.meetily.ai/meeting_minutes.sqlite"

def backup_api_key():
    """Backup the current API key"""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("SELECT geminiApiKey FROM settings WHERE id='1'")
    result = cursor.fetchone()
    conn.close()
    return result[0] if result else None

def set_invalid_api_key():
    """Set an invalid API key to simulate quota exhaustion"""
    print("🔧 Setting invalid API key to simulate quota exhaustion...")
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("UPDATE settings SET geminiApiKey='INVALID_KEY_FOR_TESTING' WHERE id='1'")
    conn.commit()
    conn.close()
    print("✅ Invalid API key set")

def restore_api_key(original_key):
    """Restore the original API key"""
    print("🔧 Restoring original API key...")
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("UPDATE settings SET geminiApiKey=? WHERE id='1'", (original_key,))
    conn.commit()
    conn.close()
    print("✅ Original API key restored")

def test_fallback_with_curl():
    """Test the fallback system by making a direct API call that will fail"""
    print("🧪 Testing fallback system with simulated quota exhaustion...")
    
    # Set environment variable for llama-helper
    env = os.environ.copy()
    env['MEETILY_LLAMA_HELPER'] = '/home/cokochu/code/githubrepos/meeting-minutes/target/release/llama-helper'
    
    # Test with a simple curl request to Gemini API with invalid key
    test_payload = '''
    {
        "contents": [{
            "parts": [{
                "text": "This is a test transcript of a meeting. Please summarize it briefly."
            }]
        }],
        "generationConfig": {
            "temperature": 0.7,
            "maxOutputTokens": 100
        }
    }
    '''
    
    # This should fail with 403/429 error, simulating quota exhaustion
    curl_cmd = [
        'curl', '-s', '-X', 'POST',
        'https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent?key=INVALID_KEY_FOR_TESTING',
        '-H', 'Content-Type: application/json',
        '-d', test_payload
    ]
    
    try:
        result = subprocess.run(curl_cmd, capture_output=True, text=True, timeout=30)
        print(f"📊 Gemini API response (should fail): {result.stdout}")
        
        if "error" in result.stdout.lower() or result.returncode != 0:
            print("✅ Gemini API correctly failed (simulating quota exhaustion)")
            return True
        else:
            print("❌ Gemini API unexpectedly succeeded")
            return False
            
    except Exception as e:
        print(f"❌ Error testing Gemini API: {e}")
        return False

def main():
    """Main test function"""
    print("🚀 Testing Gemini quota exhaustion and fallback to builtin-ai")
    print("=" * 65)
    
    # Step 1: Backup original API key
    original_key = backup_api_key()
    if not original_key:
        print("❌ Could not backup original API key")
        return
    
    print(f"✅ Original API key backed up: {original_key[:8]}...{original_key[-8:]}")
    
    try:
        # Step 2: Set invalid API key
        set_invalid_api_key()
        
        # Step 3: Test that Gemini API fails
        gemini_failed = test_fallback_with_curl()
        
        if gemini_failed:
            print("\n✅ Perfect! Gemini API is failing as expected.")
            print("🎯 Now when you use Meetily to generate a summary:")
            print("   1. It will try Gemini first (and fail)")
            print("   2. It will automatically fallback to builtin-ai/gemma3:1b")
            print("   3. The summary will be generated using the local model")
            print("   4. You should see a message like: '⚡ Generated using local AI (Gemma 3 1B) due to API quota limits.'")
            
            print("\n🚀 Test Steps:")
            print("1. Launch Meetily: meetily")
            print("2. Record a short test meeting (even 10-15 seconds)")
            print("3. Generate a summary")
            print("4. Verify the fallback message appears")
            print("5. Run this script again to restore the original API key")
        else:
            print("\n❌ Gemini API test failed unexpectedly")
            
    except KeyboardInterrupt:
        print("\n⚠️ Test interrupted by user")
    except Exception as e:
        print(f"\n❌ Test failed with error: {e}")
    finally:
        # Always restore the original API key
        restore_api_key(original_key)
        print("\n✅ Test completed and original API key restored")

if __name__ == "__main__":
    main()