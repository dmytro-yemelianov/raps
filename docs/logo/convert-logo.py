#!/usr/bin/env python3
"""
Python script to convert SVG logo to various formats.
Requires: cairosvg (pip install cairosvg)
"""

import os
import sys
from pathlib import Path

try:
    import cairosvg
except ImportError:
    print("Error: cairosvg not installed.")
    print("Install it with: pip install cairosvg")
    sys.exit(1)


def convert_svg_to_png(input_file, output_file, width=None, height=None, dpi=None):
    """Convert SVG to PNG with high quality settings."""
    try:
        # Calculate DPI for high quality if not specified
        # Higher DPI for larger images ensures sharp rendering
        if dpi is None:
            if width is not None:
                if width >= 1024:
                    dpi = 300
                elif width >= 512:
                    dpi = 200
                else:
                    dpi = 150
            else:
                dpi = 150
        
        cairosvg.svg2png(
            url=input_file,
            write_to=output_file,
            output_width=width,
            output_height=height,
            dpi=dpi
        )
        return True
    except Exception as e:
        print(f"Error converting {input_file}: {e}")
        return False


def main():
    script_dir = Path(__file__).parent
    input_file = script_dir / "raps-logo.svg"
    output_dir = script_dir / "output"
    
    # Default sizes (width in pixels, maintaining 4:3 aspect ratio)
    sizes = [256, 512, 1024, 2048]
    
    # Parse command line arguments
    if len(sys.argv) > 1:
        sizes = [int(s) for s in sys.argv[1:]]
    
    # Create output directory
    output_dir.mkdir(exist_ok=True)
    print(f"Created output directory: {output_dir}")
    
    if not input_file.exists():
        print(f"Error: Input file not found: {input_file}")
        sys.exit(1)
    
    print("\nConverting logo to PNG formats...")
    
    # Convert to PNG at different sizes
    for size in sizes:
        # Square aspect ratio
        output_file = output_dir / f"raps-logo-{size}.png"
        
        # Calculate DPI for this size
        dpi = 300 if size >= 1024 else (200 if size >= 512 else 150)
        
        if convert_svg_to_png(input_file, output_file, width=size, height=size, dpi=dpi):
            print(f"  Created: {output_file} ({size} x {size} @ {dpi} DPI)")
        else:
            print(f"  Failed: {output_file}")
    
    # Also create a high-resolution version for general use
    output_file = output_dir / "raps-logo.png"
    if convert_svg_to_png(input_file, output_file, width=1024, height=1024, dpi=300):
        print(f"  Created: {output_file} (1024 x 1024 @ 300 DPI)")
    
    print(f"\nConversion complete! Output files are in: {output_dir}")


if __name__ == "__main__":
    main()

