# ⚓ anchorr-engine

A high-performance Rust-backed video processing engine for Python. Built for heavy-duty encoding tasks with a focus on mathematical precision and parallel execution.

## Features
- **Parallel Processing**: Uses Rust/Rayon to saturate all CPU cores for batch encodes.
- **Precision Scaling**: Automatically enforces Mod-16 resolution compliance.
- **Automated Metadata Handling**: Strips all global metadata and stream tags by default.
- **Smart Crop**: Integrated `cropdetect` logic to automatically identify and strip black bars.
- **Optimized 10-bit Path**: Forced 10-bit HEVC (Main10) with SAO and Intra-smoothing disabled for maximum detail retention.

## Prerequisites
Requires `ffmpeg` and `ffprobe` to be available in your system PATH.

## Installation
```bash
uv pip install anchorr
```

## Quick Start

### 1. Probe Metadata
```python
from anchorr import AnchEngine

anch = AnchEngine()
info = anch.probe("video.mkv")
print(f"{info.width}x{info.height} using {info.codec}")
```

### 2. Auto-Crop & Encode
```python
# Detect coordinates automatically
coords = anch.get_blackbar_coords("input.mkv")

# Encode to 10-bit HEVC (mod-16 compliant)
anch.transform(
    input="input.mkv",
    output="output.mkv",
    codec="libx265",
    res="1080",
    crop=coords,
    flags="-crf 18 --preset slow"
)
```

### 3. Batch Processing
```python
# List of (input, output, codec, resolution, crop, flags)
tasks = [
    ("v1.mkv", "v1_out.mkv", "libx265", "1080", "1920:800:0:140", ""),
    ("v2.mkv", "v2_out.mkv", "libx265", "1080", "1920:1080:0:0", "")
]

# Runs all encodes in parallel using Rust threads
results = anch.batch(tasks)
```

## Development
To build from source:
1. Install Rust/Cargo.
2. Run `uv run maturin develop` to compile the extension locally.

## License
MIT

