# RAPS Logo

Logo for RAPS (Rapeseed) featuring a stylized rapeseed flower design with orange and blue petals, representing the rapeseed plant.

## Files

### Source Files
- `raps-logo.svg` - Source SVG file (vector format, scalable)

### Conversion Scripts
- `convert-logo.ps1` - PowerShell script to convert SVG to PNG formats
- `convert-logo.py` - Python script to convert SVG to PNG formats

### Generated Output Files
All PNG files are available in the `output/` directory (square format):
- `output/raps-logo.png` - High-resolution version (1024x1024) - **Use this for general purposes**
- `output/raps-logo-256.png` - Small size (256x256) - For icons/favicons
- `output/raps-logo-512.png` - Medium size (512x512) - For app icons
- `output/raps-logo-1024.png` - Large size (1024x1024) - For high-res displays
- `output/raps-logo-2048.png` - Extra large (2048x2048) - For print/banners

## Design

The logo consists of:
- **Rapeseed flower**: A stylized rapeseed flower (*Brassica napus*) with:
  - Four main petals arranged in a cruciform (cross) pattern - bright yellow, natural rapeseed color
  - Four smaller accent petals between the main ones for fullness
  - Yellow/golden center representing the flower's stamen
  - Green stem with leaves pointing upward
- **Background**: Dark charcoal gray (#2C2C2C)

**Color scheme**:
- Yellow: #FFEB3B / #FFC107 (flower petals - natural rapeseed color)
- Yellow/Gold: #FFD700 (flower center)
- Green: #4CAF50 (stem and leaves)

## Converting to PNG

### Using PowerShell (Windows)

```powershell
cd logo
.\convert-logo.ps1
```

The script will automatically detect available tools:
- Inkscape (preferred)
- ImageMagick
- Python with cairosvg

### Using Python

```bash
cd logo
# Install dependency
pip install cairosvg

# Run conversion
python convert-logo.py
```

Or specify custom sizes:
```bash
python convert-logo.py 128 256 512 1024
```

## Using the Logo

The logo files are ready to use! Reference them from the `logo/output/` directory:

### In Markdown/README
```markdown
![RAPS Logo](logo/output/raps-logo.png)
```

### In HTML
```html
<img src="logo/output/raps-logo.png" alt="RAPS Logo" width="200"/>
```

### Recommended Sizes
- **General use**: `raps-logo.png` (1024x1024)
- **GitHub README**: `raps-logo-512.png` or `raps-logo.png`
- **Favicon**: `raps-logo-256.png` (256x256)
- **High-res displays**: `raps-logo-1024.png` or `raps-logo-2048.png`

## Requirements

For PowerShell script:
- Inkscape, ImageMagick, or Python with cairosvg

For Python script:
- Python 3.6+
- cairosvg (`pip install cairosvg`)

