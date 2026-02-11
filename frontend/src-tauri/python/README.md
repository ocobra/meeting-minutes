# Python Diarization Module

This directory contains the Python-based speaker diarization engine using pyannote.audio.

## Setup

### 1. Install Python Dependencies

```bash
pip install -r requirements-diarization.txt
```

### 2. Hugging Face Authentication

Some pyannote models require authentication. Get a token from https://huggingface.co/settings/tokens

```bash
export HUGGINGFACE_API_KEY="your_token_here"
# or
export HF_TOKEN="your_token_here"
```

### 3. Accept Model License

Visit https://huggingface.co/pyannote/speaker-diarization-3.1 and accept the model license.

## Usage

### Command Line

```bash
python diarization_engine.py \
    --audio_path /path/to/audio.wav \
    --sample_rate 16000 \
    --min_speakers 2 \
    --max_speakers 5 \
    --auth_token YOUR_HF_TOKEN
```

### From Rust (via PyO3)

The Rust code in `src/diarization/engine.rs` provides FFI bindings to call this Python module.

## GPU Acceleration

The engine automatically uses CUDA if available. Ensure you have:
- CUDA toolkit installed
- PyTorch with CUDA support: `pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu118`

## Models

### Default Model
- **pyannote/speaker-diarization-3.1**: State-of-the-art diarization pipeline

### Embedding Model
- **pyannote/embedding**: Speaker embedding extraction

## Output Format

```json
{
  "success": true,
  "segments": [
    {
      "speaker_label": "Speaker 1",
      "start_time": 0.5,
      "end_time": 3.2,
      "duration": 2.7,
      "confidence": 1.0,
      "embedding": [0.123, -0.456, ...]
    }
  ],
  "num_speakers": 3,
  "total_duration": 120.5,
  "sample_rate": 16000
}
```

## Troubleshooting

### Import Errors
If you get import errors, ensure all dependencies are installed:
```bash
pip install --upgrade pyannote.audio torch torchaudio
```

### CUDA Out of Memory
Reduce batch size or use CPU:
```bash
python diarization_engine.py --device cpu ...
```

### Model Download Issues
Ensure you have accepted the model license on Hugging Face and provided a valid auth token.
