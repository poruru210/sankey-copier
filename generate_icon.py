#!/usr/bin/env python3
"""
SANKEY Copier Icon Generator
Generates .ico files with multiple sizes for Windows application
"""

from PIL import Image, ImageDraw, ImageFont
import os

def create_icon_image(size):
    """Create a single icon image at the specified size"""
    # Create image with transparency
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Calculate dimensions
    padding = max(1, size // 20)
    inner_size = size - (padding * 2)

    # Background circle with gradient effect
    # Draw multiple circles for gradient effect
    num_layers = min(5, size // 8)
    for i in range(num_layers):
        offset = i * max(1, size // 40)
        # Make sure we don't go past center
        if padding + offset >= size - padding - offset:
            break
        alpha = 255 - (i * (180 // max(1, num_layers)))
        color = (45 + i*10, 100 + i*15, 210 - i*10, alpha)
        draw.ellipse(
            [padding + offset, padding + offset,
             size - padding - offset, size - padding - offset],
            fill=color,
            outline=None
        )

    # Draw "S" letter or flow arrows depending on size
    if size >= 32:
        # For larger icons, draw stylized "S" with flow arrows
        line_width = max(2, size // 16)
        arrow_color = (255, 255, 255, 255)

        # Upper curve of S
        draw.arc(
            [padding + inner_size//4, padding,
             size - padding - inner_size//4, padding + inner_size//2],
            start=180, end=0,
            fill=arrow_color,
            width=line_width
        )

        # Lower curve of S
        draw.arc(
            [padding + inner_size//4, padding + inner_size//2,
             size - padding - inner_size//4, size - padding],
            start=0, end=180,
            fill=arrow_color,
            width=line_width
        )

        # Add arrow tips for flow indication
        arrow_size = max(3, size // 16)
        center_x = size // 2

        # Top arrow pointing right
        draw.polygon([
            (size - padding - inner_size//4 - arrow_size, padding + inner_size//4 - arrow_size),
            (size - padding - inner_size//4 + arrow_size, padding + inner_size//4),
            (size - padding - inner_size//4 - arrow_size, padding + inner_size//4 + arrow_size)
        ], fill=arrow_color)

        # Bottom arrow pointing right
        draw.polygon([
            (size - padding - inner_size//4 - arrow_size, size - padding - inner_size//4 - arrow_size),
            (size - padding - inner_size//4 + arrow_size, size - padding - inner_size//4),
            (size - padding - inner_size//4 - arrow_size, size - padding - inner_size//4 + arrow_size)
        ], fill=arrow_color)
    else:
        # For smaller icons (16x16), simple design
        # Just draw a white dot in the center
        center_size = max(2, size // 4)
        center_x = size // 2
        center_y = size // 2
        draw.ellipse(
            [center_x - center_size, center_y - center_size,
             center_x + center_size, center_y + center_size],
            fill=(255, 255, 255, 255)
        )

    return img

def generate_ico_file(output_path):
    """Generate .ico file with multiple sizes"""
    print(f"Generating icon: {output_path}")

    # Standard Windows icon sizes
    sizes = [16, 32, 48, 64, 128, 256]
    images = []

    for size in sizes:
        print(f"  Creating {size}x{size} image...")
        img = create_icon_image(size)
        images.append(img)

    # Save as .ico file
    images[0].save(
        output_path,
        format='ICO',
        sizes=[(img.width, img.height) for img in images],
        append_images=images[1:]
    )

    print(f"  Saved: {output_path}")
    return output_path

def main():
    """Main function to generate all required icon files"""
    print("SANKEY Copier Icon Generator")
    print("=" * 50)

    # Get script directory
    script_dir = os.path.dirname(os.path.abspath(__file__))

    # Define output paths
    icons_to_generate = [
        # Installer resources
        os.path.join(script_dir, 'installer', 'resources', 'icon.ico'),
        # Tray application
        os.path.join(script_dir, 'sankey-copier-tray', 'icon.ico'),
    ]

    # Ensure directories exist
    for icon_path in icons_to_generate:
        os.makedirs(os.path.dirname(icon_path), exist_ok=True)

    # Generate icons
    generated = []
    for icon_path in icons_to_generate:
        try:
            generated.append(generate_ico_file(icon_path))
        except Exception as e:
            print(f"  Error: {e}")

    print("=" * 50)
    print(f"Generated {len(generated)} icon files successfully!")
    print("\nGenerated files:")
    for path in generated:
        print(f"  - {path}")

    # Also generate PNG previews for verification
    print("\nGenerating PNG previews...")
    preview_sizes = [256, 64, 32, 16]
    preview_dir = os.path.join(script_dir, 'icon_preview')
    os.makedirs(preview_dir, exist_ok=True)

    for size in preview_sizes:
        img = create_icon_image(size)
        preview_path = os.path.join(preview_dir, f'icon_{size}x{size}.png')
        img.save(preview_path, 'PNG')
        print(f"  - {preview_path}")

if __name__ == '__main__':
    main()
