import subprocess
import time
from PIL import Image

# Kill exact binary path to avoid matching this python script path
print("Killing existing settings process...")
subprocess.run('pkill -f "/home/lsgalante/.local/bin/cce-settings"', shell=True)
time.sleep(0.5)

print("Starting settings application...")
subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 /home/lsgalante/.local/bin/cce-settings &", shell=True)
time.sleep(1.5)

print("Focusing window...")
subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl toplevel focus title:'CCE System Settings'", shell=True)
time.sleep(0.3)

print("Resetting cursor...")
subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer move -3000 -3000", shell=True)
time.sleep(0.1)

print("Moving cursor to safe spot...")
subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer move 380 250", shell=True)
time.sleep(0.2)

print("Scrolling down...")
for _ in range(45):
    subprocess.run("env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 wlrctl pointer scroll -15 0", shell=True)
    time.sleep(0.04)

time.sleep(0.8)

# Take screenshot
screenshot_path = "/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/screenshot_scrolled_ok.png"
subprocess.run(f"env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 grim {screenshot_path}", shell=True)
print("Screenshot saved.")

# Crop Keybindings
img = Image.open(screenshot_path)
wx = 100 * 2
wy = 100 * 2
ww = 400 * 2
wh = 680 * 2
crop_box = (wx, wy + wh // 2, wx + ww, wy + wh)
cropped = img.crop(crop_box)
cropped.save("/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/crop_scrolled_final.png")
print("Crop saved.")
