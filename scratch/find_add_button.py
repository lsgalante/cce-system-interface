from PIL import Image

# Load the screenshot
img = Image.open("/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/screenshot_final.png")
print("Image size:", img.size)

# Scale window bounds by 2
wx = 100 * 2
wy = 100 * 2
ww = 400 * 2
wh = 680 * 2

# Touchpad is in the upper half of the window
crop_touchpad = img.crop((wx, wy, wx + ww, wy + wh // 2))
crop_touchpad.save("/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/crop_touchpad_final.png")
print("Touchpad crop saved.")

# Keybindings is in the lower half of the window
crop_keybindings = img.crop((wx, wy + wh // 2, wx + ww, wy + wh))
crop_keybindings.save("/home/lsgalante/.gemini/antigravity/brain/e9f42138-61bb-4037-ac7e-0b1d2091db1c/crop_keybindings_final.png")
print("Keybindings crop saved.")
