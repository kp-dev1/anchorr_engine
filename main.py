from anchorr import AnchEngine

# Initialize your engine
anch = AnchEngine()
input_file = "vfr_blackbars_input.mkv"
output_file = "final_cfr_720p_x265.mkv"

print(f"⚓ Analyzing {input_file}...")

# 1. Detect the black bars automatically
# This will return something like "1920:800:0:140"
crop_coords = anch.get_blackbar_coords(input_file)
print(f"⚓ Detected Crop: {crop_coords}")

# 2. Transform the video
# Your .transform() method already forces CFR via '-vsync cfr' in the Rust code
success = anch.transform(
    input=input_file,
    output=output_file,
    codec="libx265",       # Switch to H.265
    res="720",             # Downscale to 720p
    crop=crop_coords,      # Remove the bars detected above
    flags="-crf 20"        # Additional quality settings
)

if success:
    print(f"⚓ Success! Created {output_file}")
    
    # 3. Verify the results
    new_meta = anch.probe(output_file)
    print(f"⚓ New Specs: {new_meta.width}x{new_meta.height} | Codec: {new_meta.codec}")
else:
    print("⚓ Encoding failed.")

