#!/usr/bin/env python3
"""
Speaker Diarization Engine using pyannote.audio

This module provides speaker diarization functionality using the pyannote.audio library.
It segments audio by speaker and extracts voice embeddings for speaker clustering.

Requirements:
    - pyannote.audio
    - torch
    - torchaudio
    - numpy

Usage:
    python diarization_engine.py --audio_path <path> --sample_rate <rate>
"""

import sys
import json
import argparse
import warnings
from typing import List, Dict, Any, Optional
from pathlib import Path

# Suppress warnings
warnings.filterwarnings("ignore")

try:
    import torch
    import torchaudio
    import numpy as np
    from pyannote.audio import Pipeline
    from pyannote.audio.pipelines.speaker_verification import PretrainedSpeakerEmbedding
except ImportError as e:
    print(json.dumps({
        "error": f"Missing required dependency: {e}. Install with: pip install pyannote.audio torch torchaudio"
    }), file=sys.stderr)
    sys.exit(1)


class DiarizationEngine:
    """Speaker diarization engine using pyannote.audio"""
    
    def __init__(
        self,
        model_name: str = "pyannote/speaker-diarization-3.1",
        embedding_model: str = "pyannote/embedding",
        use_auth_token: Optional[str] = None,
        device: str = "cuda" if torch.cuda.is_available() else "cpu"
    ):
        """
        Initialize the diarization engine.
        
        Args:
            model_name: Hugging Face model name for diarization pipeline
            embedding_model: Model name for speaker embeddings
            use_auth_token: Hugging Face authentication token (required for some models)
            device: Device to run on ('cuda' or 'cpu')
        """
        self.device = device
        self.model_name = model_name
        
        print(f"Initializing diarization engine on device: {device}", file=sys.stderr)
        
        try:
            # Load diarization pipeline
            self.pipeline = Pipeline.from_pretrained(
                model_name,
                use_auth_token=use_auth_token
            )
            self.pipeline.to(torch.device(device))
            
            # Load embedding model for voice fingerprints
            self.embedding_model = PretrainedSpeakerEmbedding(
                embedding_model,
                device=torch.device(device),
                use_auth_token=use_auth_token
            )
            
            print("Diarization engine initialized successfully", file=sys.stderr)
            
        except Exception as e:
            print(f"Failed to initialize diarization engine: {e}", file=sys.stderr)
            raise
    
    def process_audio(
        self,
        audio_path: str,
        sample_rate: int = 16000,
        min_speakers: Optional[int] = None,
        max_speakers: Optional[int] = None,
        min_segment_duration: float = 1.0
    ) -> Dict[str, Any]:
        """
        Process audio file and return speaker segments.
        
        Args:
            audio_path: Path to audio file
            sample_rate: Audio sample rate
            min_speakers: Minimum number of speakers (optional)
            max_speakers: Maximum number of speakers (optional)
            min_segment_duration: Minimum segment duration in seconds
            
        Returns:
            Dictionary containing speaker segments and metadata
        """
        try:
            print(f"Processing audio: {audio_path}", file=sys.stderr)
            
            # Load audio
            waveform, sr = torchaudio.load(audio_path)
            
            # Resample if necessary
            if sr != sample_rate:
                resampler = torchaudio.transforms.Resample(sr, sample_rate)
                waveform = resampler(waveform)
            
            # Convert to mono if stereo
            if waveform.shape[0] > 1:
                waveform = torch.mean(waveform, dim=0, keepdim=True)
            
            # Run diarization
            diarization = self.pipeline(
                {"waveform": waveform, "sample_rate": sample_rate},
                min_speakers=min_speakers,
                max_speakers=max_speakers
            )
            
            # Extract segments
            segments = []
            speaker_labels = {}
            speaker_counter = 1
            
            for turn, _, speaker in diarization.itertracks(yield_label=True):
                # Assign sequential speaker labels
                if speaker not in speaker_labels:
                    speaker_labels[speaker] = f"Speaker {speaker_counter}"
                    speaker_counter += 1
                
                # Filter by minimum duration
                duration = turn.end - turn.start
                if duration < min_segment_duration:
                    continue
                
                # Extract embedding for this segment
                segment_waveform = waveform[:, int(turn.start * sample_rate):int(turn.end * sample_rate)]
                embedding = self._extract_embedding(segment_waveform, sample_rate)
                
                segments.append({
                    "speaker_label": speaker_labels[speaker],
                    "start_time": float(turn.start),
                    "end_time": float(turn.end),
                    "duration": float(duration),
                    "confidence": 1.0,  # pyannote doesn't provide confidence scores
                    "embedding": embedding.tolist() if embedding is not None else []
                })
            
            result = {
                "success": True,
                "segments": segments,
                "num_speakers": len(speaker_labels),
                "total_duration": float(waveform.shape[1] / sample_rate),
                "sample_rate": sample_rate
            }
            
            print(f"Diarization complete: {len(segments)} segments, {len(speaker_labels)} speakers", file=sys.stderr)
            return result
            
        except Exception as e:
            print(f"Error processing audio: {e}", file=sys.stderr)
            return {
                "success": False,
                "error": str(e),
                "segments": []
            }
    
    def _extract_embedding(self, waveform: torch.Tensor, sample_rate: int) -> Optional[np.ndarray]:
        """
        Extract speaker embedding from audio segment.
        
        Args:
            waveform: Audio waveform tensor
            sample_rate: Sample rate
            
        Returns:
            Embedding vector as numpy array
        """
        try:
            # Ensure minimum length (embedding models need sufficient audio)
            min_samples = int(0.5 * sample_rate)  # 0.5 seconds minimum
            if waveform.shape[1] < min_samples:
                # Pad if too short
                padding = min_samples - waveform.shape[1]
                waveform = torch.nn.functional.pad(waveform, (0, padding))
            
            # Extract embedding
            with torch.no_grad():
                embedding = self.embedding_model(waveform)
            
            return embedding.cpu().numpy().flatten()
            
        except Exception as e:
            print(f"Warning: Failed to extract embedding: {e}", file=sys.stderr)
            return None
    
    def process_audio_chunk(
        self,
        audio_data: np.ndarray,
        sample_rate: int,
        chunk_index: int
    ) -> Dict[str, Any]:
        """
        Process audio chunk for streaming/real-time diarization.
        
        Args:
            audio_data: Audio data as numpy array
            sample_rate: Sample rate
            chunk_index: Index of this chunk in the stream
            
        Returns:
            Dictionary containing speaker segments for this chunk
        """
        # Note: Real-time diarization is complex and requires maintaining state
        # This is a simplified implementation
        print(f"Processing chunk {chunk_index}", file=sys.stderr)
        
        try:
            # Convert numpy array to torch tensor
            waveform = torch.from_numpy(audio_data).float().unsqueeze(0)
            
            # Run diarization on chunk
            diarization = self.pipeline(
                {"waveform": waveform, "sample_rate": sample_rate}
            )
            
            segments = []
            for turn, _, speaker in diarization.itertracks(yield_label=True):
                embedding = self._extract_embedding(
                    waveform[:, int(turn.start * sample_rate):int(turn.end * sample_rate)],
                    sample_rate
                )
                
                segments.append({
                    "speaker_label": speaker,
                    "start_time": float(turn.start),
                    "end_time": float(turn.end),
                    "confidence": 1.0,
                    "embedding": embedding.tolist() if embedding is not None else []
                })
            
            return {
                "success": True,
                "chunk_index": chunk_index,
                "segments": segments
            }
            
        except Exception as e:
            print(f"Error processing chunk: {e}", file=sys.stderr)
            return {
                "success": False,
                "error": str(e),
                "segments": []
            }


def main():
    """Command-line interface for diarization engine"""
    parser = argparse.ArgumentParser(description="Speaker Diarization Engine")
    parser.add_argument("--audio_path", required=True, help="Path to audio file")
    parser.add_argument("--sample_rate", type=int, default=16000, help="Sample rate")
    parser.add_argument("--min_speakers", type=int, help="Minimum number of speakers")
    parser.add_argument("--max_speakers", type=int, help="Maximum number of speakers")
    parser.add_argument("--min_duration", type=float, default=1.0, help="Minimum segment duration")
    parser.add_argument("--model", default="pyannote/speaker-diarization-3.1", help="Model name")
    parser.add_argument("--auth_token", help="Hugging Face auth token")
    parser.add_argument("--device", default="auto", help="Device (cuda/cpu/auto)")
    
    args = parser.parse_args()
    
    # Determine device
    if args.device == "auto":
        device = "cuda" if torch.cuda.is_available() else "cpu"
    else:
        device = args.device
    
    # Initialize engine
    engine = DiarizationEngine(
        model_name=args.model,
        use_auth_token=args.auth_token,
        device=device
    )
    
    # Process audio
    result = engine.process_audio(
        audio_path=args.audio_path,
        sample_rate=args.sample_rate,
        min_speakers=args.min_speakers,
        max_speakers=args.max_speakers,
        min_segment_duration=args.min_duration
    )
    
    # Output JSON result
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
