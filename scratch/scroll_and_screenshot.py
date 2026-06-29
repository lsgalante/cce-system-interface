import subprocess
import time
from PIL import Image

def scroll_content():
    print("Resetting cursor to top-left...")
    subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer move -3000 -3000", shell=True)
    time.sleep(0.1)
    
    print("Moving cursor to settings window content area...")
    subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer move 300 440", shell=True)
    time.sleep(0.2)
    
    print("Clicking to focus settings window...")
    subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer click left", shell=True)
    time.sleep(0.3)
    
    # Scroll down multiple times
    print("Scrolling down...")
    for _ in range(35):
        subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer scroll -15 0", shell=True)
        time.sleep(0.05)
    
    time.sleep(0.8)

scroll_content()

# Take screenshot
screenshot_path = "/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/screenshot_scrolled.png"
subprocess.run(f"env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 grim {screenshot_path}", shell=True)
print("Screenshot saved.")

# Crop the bottom-left area of the window
img = Image.open(screenshot_path)
wx = 100 * 2
wy = 100 * 2
ww = 400 * 2
wh = 680 * 2
crop_box = (wx, wy + wh // 2, wx + ww, wy + wh)
cropped = img.crop(crop_box)
cropped.save("/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/crop_scrolled_final.png")
print("Crop saved to crop_scrolled_final.png")
