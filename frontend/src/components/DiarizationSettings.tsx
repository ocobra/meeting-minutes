'use client';

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { DiarizationConfig } from '@/types';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { Label } from './ui/label';
import { Switch } from './ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Slider } from './ui/slider';
import { Button } from './ui/button';
import { Users, Shield, Gauge, Save, AlertCircle } from 'lucide-react';
import { Alert, AlertDescription } from './ui/alert';

export const DiarizationSettings: React.FC = () => {
  const [enabled, setEnabled] = useState(false);
  const [config, setConfig] = useState<DiarizationConfig>({
    processing_mode: 'Batch',
    privacy_mode: 'PreferExternal',
    confidence_threshold: 0.7,
    enable_identification: true,
  });
  const [saving, setSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState<boolean | null>(null);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      // Load from localStorage or backend
      const savedEnabled = localStorage.getItem('diarization_enabled');
      if (savedEnabled !== null) {
        setEnabled(savedEnabled === 'true');
      }

      const savedConfig = localStorage.getItem('diarization_config');
      if (savedConfig) {
        setConfig(JSON.parse(savedConfig));
      }
    } catch (error) {
      console.error('Failed to load diarization settings:', error);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    setSaveSuccess(null);

    try {
      // Save enabled state
      localStorage.setItem('diarization_enabled', enabled.toString());
      
      // Save configuration
      localStorage.setItem('diarization_config', JSON.stringify(config));
      
      // Send to backend
      await invoke('configure_diarization', { config });
      
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(null), 3000);
    } catch (error) {
      console.error('Failed to save diarization settings:', error);
      setSaveSuccess(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="w-5 h-5" />
            Speaker Diarization
          </CardTitle>
          <CardDescription>
            Automatically identify and label different speakers in your meetings
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Enable/Disable Toggle */}
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="diarization-enabled">Enable Speaker Diarization</Label>
              <p className="text-sm text-muted-foreground">
                Detect and label different speakers in recordings
              </p>
            </div>
            <Switch
              id="diarization-enabled"
              checked={enabled}
              onCheckedChange={setEnabled}
            />
          </div>

          {enabled && (
            <>
              {/* Processing Mode */}
              <div className="space-y-2">
                <Label htmlFor="processing-mode" className="flex items-center gap-2">
                  <Gauge className="w-4 h-4" />
                  Processing Mode
                </Label>
                <Select
                  value={config.processing_mode}
                  onValueChange={(value: 'Batch' | 'RealTime') =>
                    setConfig({ ...config, processing_mode: value })
                  }
                >
                  <SelectTrigger id="processing-mode">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="Batch">
                      Batch (Higher Accuracy)
                    </SelectItem>
                    <SelectItem value="RealTime">
                      Real-Time (Lower Latency)
                    </SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {config.processing_mode === 'Batch'
                    ? 'Process entire recording for best accuracy'
                    : 'Process in real-time with lower latency'}
                </p>
              </div>

              {/* Privacy Mode */}
              <div className="space-y-2">
                <Label htmlFor="privacy-mode" className="flex items-center gap-2">
                  <Shield className="w-4 h-4" />
                  Privacy Mode
                </Label>
                <Select
                  value={config.privacy_mode}
                  onValueChange={(value: 'LocalOnly' | 'PreferExternal' | 'ExternalOnly') =>
                    setConfig({ ...config, privacy_mode: value })
                  }
                >
                  <SelectTrigger id="privacy-mode">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="LocalOnly">
                      Local Only (Maximum Privacy)
                    </SelectItem>
                    <SelectItem value="PreferExternal">
                      Prefer External (Recommended)
                    </SelectItem>
                    <SelectItem value="ExternalOnly">
                      External Only (Maximum Accuracy)
                    </SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {config.privacy_mode === 'LocalOnly' && 'Never use external models, all processing stays local'}
                  {config.privacy_mode === 'PreferExternal' && 'Use external models when available, fallback to local'}
                  {config.privacy_mode === 'ExternalOnly' && 'Only use external models, fail if unavailable'}
                </p>
              </div>

              {/* Confidence Threshold */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="confidence-threshold">
                    Confidence Threshold
                  </Label>
                  <span className="text-sm text-muted-foreground">
                    {((config.confidence_threshold || 0.7) * 100).toFixed(0)}%
                  </span>
                </div>
                <Slider
                  id="confidence-threshold"
                  min={0}
                  max={100}
                  step={5}
                  value={[(config.confidence_threshold || 0.7) * 100]}
                  onValueChange={(values: number[]) =>
                    setConfig({ ...config, confidence_threshold: values[0] / 100 })
                  }
                />
                <p className="text-xs text-muted-foreground">
                  Minimum confidence required to assign speaker names
                </p>
              </div>

              {/* Enable Identification */}
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label htmlFor="enable-identification">
                    Enable Name Identification
                  </Label>
                  <p className="text-sm text-muted-foreground">
                    Extract speaker names from introductions
                  </p>
                </div>
                <Switch
                  id="enable-identification"
                  checked={config.enable_identification}
                  onCheckedChange={(checked) =>
                    setConfig({ ...config, enable_identification: checked })
                  }
                />
              </div>

              {/* Privacy Notice */}
              {config.privacy_mode !== 'LocalOnly' && (
                <Alert>
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription>
                    External models may send audio data to third-party services.
                    Voice embeddings are stored as irreversible hashes for privacy.
                  </AlertDescription>
                </Alert>
              )}
            </>
          )}

          {/* Save Button */}
          <div className="flex items-center gap-2 pt-4">
            <Button
              onClick={handleSave}
              disabled={saving}
              className="flex items-center gap-2"
            >
              <Save className="w-4 h-4" />
              {saving ? 'Saving...' : 'Save Settings'}
            </Button>
            {saveSuccess === true && (
              <span className="text-sm text-green-600 dark:text-green-400">
                Settings saved successfully
              </span>
            )}
            {saveSuccess === false && (
              <span className="text-sm text-red-600 dark:text-red-400">
                Failed to save settings
              </span>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Voice Profiles Section */}
      {enabled && (
        <Card>
          <CardHeader>
            <CardTitle>Voice Profiles</CardTitle>
            <CardDescription>
              Manage known speaker voice profiles for automatic recognition
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">
              Voice profile management coming soon. This will allow you to enroll
              known speakers for automatic recognition across meetings.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
};
