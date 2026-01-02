#!/usr/bin/env python3
"""
Convert SVG logo to WebP format with transparency.
Requires: cairosvg, Pillow (pip install cairosvg pillow)
"""

import os
import sys
from pathlib import Path

try:
    import cairosvg
    from PIL import Image
except ImportError as e:
    print(f"Error: Missing required package: {e}")
    print("Install dependencies with: pip install cairosvg pillow")
    sys.exit(1)


def convert_svg_to_webp(input_file, output_file, width=None, height=None, quality=90):
    """Convert SVG to WebP with transparency preserved."""
    try:
        # First convert SVG to PNG with transparency
        # Use RGBA format to preserve transparency
        png_data = cairosvg.svg2png(
            url=str(input_file),
            output_width=width,
            output_height=height,
            dpi=300 if width and width >= 1024 else 200
        )
        
        # Load PNG data into PIL Image
        from io import BytesIO
        img = Image.open(BytesIO(png_data))
        
        # Ensure RGBA mode for transparency
        if img.mode != 'RGBA':
            img = img.convert('RGBA')
        
        # Save as WebP with transparency support
        img.save(
            output_file,
            'WEBP',
            quality=quality,
            method=6,  # Best compression
            lossless=False  # Use lossy compression for smaller file size
        )
        
        return True
    except Exception as e:
        print(f"Error converting {input_file}: {e}")
        return False


def main():
    script_dir = Path(__file__).parent
    input_file = script_dir / "raps-logo.svg"
    output_dir = script_dir / "output"
    
    # WebP sizes optimized for web use
    # Using sizes that are good for web: 256px for small, 512px for medium, 1024px for high-res
    sizes = [
        (256, 90),   # Small - good quality
        (512, 90),   # Medium - good quality  
        (1024, 85),  # Large - slightly lower quality for file size
    ]
    
    # Create output directory
    output_dir.mkdir(exist_ok=True)
    
    if not input_file.exists():
        print(f"Error: Input file not found: {input_file}")
        sys.exit(1)
    
    print("\nConverting logo to WebP formats with transparency...")
    
    # Convert to WebP at different sizes
    for size, quality in sizes:
        output_file = output_dir / f"raps-logo-{size}.webp"
        
        if convert_svg_to_webp(input_file, output_file, width=size, height=size, quality=quality):
            file_size = output_file.stat().st_size / 1024  # Size in KB
            print(f"  Created: {output_file} ({size} x {size}, {file_size:.1f} KB, quality={quality})")
        else:
            print(f"  Failed: {output_file}")
    
    # Also create a standard high-quality version for general use
    output_file = output_dir / "raps-logo.webp"
    if convert_svg_to_webp(input_file, output_file, width=512, height=512, quality=90):
        file_size = output_file.stat().st_size / 1024
        print(f"  Created: {output_file} (512 x 512, {file_size:.1f} KB, quality=90)")
    
    print(f"\nConversion complete! WebP files are in: {output_dir}")


if __name__ == "__main__":
    main()

