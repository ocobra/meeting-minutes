'use client';

import { useState } from 'react';
import { Pencil, Users } from 'lucide-react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/tooltip';
import { invoke } from '@tauri-apps/api/core';

interface SpeakerLabelProps {
  speakerLabel: string;
  speakerName?: string;
  confidence?: number;
  isOverlapping?: boolean;
  meetingId: string;
  onNameUpdate?: (newName: string) => void;
  editable?: boolean;
}

export const SpeakerLabel: React.FC<SpeakerLabelProps> = ({
  speakerLabel,
  speakerName,
  confidence,
  isOverlapping,
  meetingId,
  onNameUpdate,
  editable = true,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editedName, setEditedName] = useState(speakerName || '');
  const [isSaving, setIsSaving] = useState(false);

  const displayName = speakerName || speakerLabel;
  const isLowConfidence = confidence !== undefined && confidence < 0.7;

  const handleSave = async () => {
    if (!editedName.trim() || editedName === speakerName) {
      setIsEditing(false);
      return;
    }

    setIsSaving(true);
    try {
      await invoke('update_speaker_name', {
        meetingId,
        speakerLabel,
        newName: editedName.trim(),
      });
      
      if (onNameUpdate) {
        onNameUpdate(editedName.trim());
      }
      setIsEditing(false);
    } catch (error) {
      console.error('Failed to update speaker name:', error);
      alert('Failed to update speaker name. Please try again.');
    } finally {
      setIsSaving(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSave();
    } else if (e.key === 'Escape') {
      setEditedName(speakerName || '');
      setIsEditing(false);
    }
  };

  if (isEditing) {
    return (
      <div className="inline-flex items-center gap-2 px-2 py-1 bg-blue-50 dark:bg-blue-900/20 rounded">
        <Input
          value={editedName}
          onChange={(e) => setEditedName(e.target.value)}
          onKeyDown={handleKeyDown}
          onBlur={handleSave}
          disabled={isSaving}
          className="h-6 w-32 text-sm"
          placeholder="Enter name..."
          autoFocus
        />
      </div>
    );
  }

  return (
    <div className="inline-flex items-center gap-1.5 group">
      <span
        className={`
          inline-flex items-center gap-1 px-2 py-0.5 rounded text-sm font-medium
          ${isOverlapping 
            ? 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300' 
            : 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300'
          }
          ${isLowConfidence ? 'opacity-70' : ''}
        `}
      >
        {isOverlapping && <Users className="w-3 h-3" />}
        {displayName}
        {isLowConfidence && ' (?)'}
      </span>
      
      {editable && !isOverlapping && (
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="h-5 w-5 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={() => setIsEditing(true)}
            >
              <Pencil className="w-3 h-3" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>Edit speaker name</p>
          </TooltipContent>
        </Tooltip>
      )}
    </div>
  );
};
