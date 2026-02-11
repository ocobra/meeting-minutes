'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SpeakerStatistics } from '@/types';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from './ui/card';
import { Users, Clock, MessageSquare, TrendingUp } from 'lucide-react';
import { Progress } from './ui/progress';

interface SpeakerStatisticsViewProps {
  meetingId: string;
  className?: string;
}

export const SpeakerStatisticsView: React.FC<SpeakerStatisticsViewProps> = ({
  meetingId,
  className = '',
}) => {
  const [statistics, setStatistics] = useState<SpeakerStatistics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadStatistics();
  }, [meetingId]);

  const loadStatistics = async () => {
    try {
      setLoading(true);
      setError(null);
      const stats = await invoke<SpeakerStatistics[]>('get_speaker_statistics', {
        meetingId,
      });
      setStatistics(stats);
    } catch (err) {
      console.error('Failed to load speaker statistics:', err);
      setError('Failed to load speaker statistics');
    } finally {
      setLoading(false);
    }
  };

  const formatTime = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}m ${secs}s`;
  };

  if (loading) {
    return (
      <Card className={className}>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="w-5 h-5" />
            Speaker Statistics
          </CardTitle>
          <CardDescription>Loading speaker data...</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className={className}>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="w-5 h-5" />
            Speaker Statistics
          </CardTitle>
          <CardDescription className="text-red-500">{error}</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  if (statistics.length === 0) {
    return (
      <Card className={className}>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="w-5 h-5" />
            Speaker Statistics
          </CardTitle>
          <CardDescription>
            No speaker data available. Enable diarization to see speaker statistics.
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  // Sort by speaking time (descending)
  const sortedStats = [...statistics].sort(
    (a, b) => b.speaking_time_seconds - a.speaking_time_seconds
  );

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Users className="w-5 h-5" />
          Speaker Statistics
        </CardTitle>
        <CardDescription>
          Speaking time and participation for {statistics.length} speaker{statistics.length !== 1 ? 's' : ''}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {sortedStats.map((stat) => (
          <div key={stat.speaker_label} className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-8 h-8 rounded-full bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
                  <span className="text-sm font-medium text-blue-700 dark:text-blue-300">
                    {stat.speaker_name?.charAt(0) || stat.speaker_label.charAt(0)}
                  </span>
                </div>
                <div>
                  <p className="font-medium text-sm">
                    {stat.speaker_name || stat.speaker_label}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {stat.speaking_percentage.toFixed(1)}% of meeting
                  </p>
                </div>
              </div>
              <div className="text-right text-sm">
                <div className="flex items-center gap-1 text-muted-foreground">
                  <Clock className="w-3 h-3" />
                  {formatTime(stat.speaking_time_seconds)}
                </div>
                <div className="flex items-center gap-1 text-muted-foreground text-xs">
                  <MessageSquare className="w-3 h-3" />
                  {stat.turn_count} turn{stat.turn_count !== 1 ? 's' : ''}
                </div>
              </div>
            </div>
            <Progress value={stat.speaking_percentage} className="h-2" />
          </div>
        ))}

        {/* Summary */}
        <div className="pt-4 border-t space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">Total Speakers</span>
            <span className="font-medium">{statistics.length}</span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">Total Turns</span>
            <span className="font-medium">
              {statistics.reduce((sum, s) => sum + s.turn_count, 0)}
            </span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">Total Speaking Time</span>
            <span className="font-medium">
              {formatTime(statistics.reduce((sum, s) => sum + s.speaking_time_seconds, 0))}
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
};
