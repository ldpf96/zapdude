"""Generate the Zapdude application icon source PNG (1024x1024, RGBA)."""

from PIL import Image, ImageDraw

W = 1024
IMG = Image.new("RGBA", (W, W), (0, 0, 0, 0))
D = ImageDraw.Draw(IMG)

BACKGROUND = (7, 94, 84, 255)      # deep WhatsApp-tone green
ACCENT = (37, 211, 102, 255)       # WhatsApp green
BOLT = (255, 255, 255, 255)

# Rounded app background.
D.rounded_rectangle((16, 16, W - 16, W - 16), radius=200, fill=BACKGROUND)
# Green accent ring.
D.rounded_rectangle((16, 16, W - 16, W - 16), radius=200, outline=ACCENT, width=28)

# White lightning bolt (light Zap brand mark).
bolt = [
    (560, 90),
    (250, 560),
    (470, 560),
    (390, 950),
    (800, 400),
    (520, 400),
    (640, 90),
]
D.polygon(bolt, fill=BOLT)

IMG.save(r"C:\Users\PC\Desktop\Zapdude\zapdude-rs\tools\icon_source.png")
print("icon_source.png written")
