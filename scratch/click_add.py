from PIL import Image
import subprocess
import time

# Load the cropped image
img = Image.open("/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/crop_keybindings.png")
width, height = img.size
print(f"Crop size: {width}x{height}")

# Find blue button pixels.
# The button background color is blue, e.g. RGB around [41, 76, 101] or similar.
# Let's search for pixels where B > R + 15 and B > G + 10, and not too dark or light.
blue_pixels = []
for y in range(height):
    for x in range(width):
        r, g, b = img.getpixel((x, y))[:3]
        if b > r + 15 and b > g + 10 and 30 < b < 180:
            blue_pixels.append((x, y))

if blue_pixels:
    print(f"Found {len(blue_pixels)} blue pixels.")
    min_x = min(p[0] for p in blue_pixels)
    max_x = max(p[0] for p in blue_pixels)
    min_y = min(p[1] for p in blue_pixels)
    max_y = max(p[1] for p in blue_pixels)
    
    print(f"Blue bounding box: X=[{min_x}, {max_x}], Y=[{min_y}, {max_y}]")
    
    target_pt = ((min_x + max_x) // 2, (min_y + max_y) // 2)
    print(f"Target center in crop: {target_pt}")
    
    # Calculate screen coordinates (in physical pixels)
    # Remember crop starts at: wx = 185 * 2 = 370
    # wy_crop_start = wy + wh // 2 = 27 * 2 + 1069 = 1123
    screen_x_physical = 370 + target_pt[0]
    screen_y_physical = 1123 + target_pt[1]
    
    # wlrctl pointer move takes coordinates in logical pixels!
    # Wait, let's divide physical pixels by 2.0 to get logical pixels.
    logical_x = int(screen_x_physical / 2.0)
    logical_y = int(screen_y_physical / 2.0)
    print(f"Clicking at screen logical coords: ({logical_x}, {logical_y})")
    
    # Reset cursor to top-left
    subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer move -3000 -3000", shell=True)
    time.sleep(0.1)
    # Move to logical coordinates
    subprocess.run(f"env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer move {logical_x} {logical_y}", shell=True)
    time.sleep(0.2)
    # Click
    subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer click left", shell=True)
    time.sleep(0.8)
    
    # Take new screenshot
    screenshot_path = "/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/screenshot_after_click.png"
    subprocess.run(f"env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 grim {screenshot_path}", shell=True)
    
    # Crop again to verify
    img_new = Image.open(screenshot_path)
    wx, wy, ww, wh = 185 * 2, 27 * 2, 641 * 2, 1069 * 2
    crop_box = (wx, wy + wh // 2, wx + ww // 2, wy + wh)
    cropped_new = img_new.crop(crop_box)
    cropped_new.save("/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/crop_keybindings_after.png")
    print("Verification crop saved to crop_keybindings_after.png")
else:
    print("Could not find blue pixels for Add Keybind button.")
