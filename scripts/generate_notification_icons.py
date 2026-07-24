from pathlib import Path
from PIL import Image, ImageDraw
import json


OUT = Path("assets/icons/notifications")
SCALE = 4
SIZE = 32

P = {
    "ink": "#11131b",
    "shadow": "#24202b",
    "cream": "#f4d69a",
    "gold": "#f4b642",
    "gold2": "#ffd86b",
    "brown": "#8a5732",
    "red": "#d94a3d",
    "red2": "#ff705f",
    "green": "#79b96d",
    "green2": "#a4d879",
    "blue": "#4aa3d8",
    "blue2": "#78d7ff",
    "purple": "#8d65b5",
    "purple2": "#c096e8",
    "steel": "#a9b6c8",
    "steel2": "#e2edf8",
    "cyan": "#55d6d9",
    "orange": "#e28b45",
}


def canvas():
    return Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))


def px(d, x, y, color, w=1, h=1):
    d.rectangle([x, y, x + w - 1, y + h - 1], fill=color)


def rect(d, x, y, w, h, color):
    px(d, x, y, color, w, h)


def outline_box(d, x, y, w, h, fill, border=None):
    border = border or P["ink"]
    rect(d, x + 1, y, w - 2, 1, border)
    rect(d, x + 1, y + h - 1, w - 2, 1, border)
    rect(d, x, y + 1, 1, h - 2, border)
    rect(d, x + w - 1, y + 1, 1, h - 2, border)
    rect(d, x + 1, y + 1, w - 2, h - 2, fill)


def sparkle(d, x, y, color=P["gold2"]):
    px(d, x, y - 1, color)
    px(d, x - 1, y, color)
    px(d, x, y, "#ffffff")
    px(d, x + 1, y, color)
    px(d, x, y + 1, color)


def save(name, img):
    big = img.resize((SIZE * SCALE, SIZE * SCALE), Image.Resampling.NEAREST)
    big.save(OUT / f"{name}.png")


def icon_tasks():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 8, 4, 16, 23, P["cream"], P["brown"])
    rect(d, 10, 6, 12, 1, "#ffe8ad")
    rect(d, 11, 11, 10, 2, P["brown"])
    rect(d, 11, 16, 8, 2, P["brown"])
    rect(d, 14, 21, 3, 3, P["ink"])
    rect(d, 17, 18, 3, 3, P["ink"])
    rect(d, 20, 15, 3, 3, P["ink"])
    rect(d, 6, 7, 3, 5, P["brown"])
    rect(d, 22, 23, 3, 2, "#c28b55")
    return img


def icon_task_due():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 7, 6, 18, 20, P["cream"], P["brown"])
    rect(d, 10, 11, 10, 2, P["brown"])
    rect(d, 10, 15, 7, 2, P["brown"])
    rect(d, 16, 20, 2, 3, P["ink"])
    rect(d, 18, 18, 2, 2, P["ink"])
    rect(d, 20, 16, 2, 2, P["ink"])
    rect(d, 23, 4, 4, 4, P["gold"])
    rect(d, 24, 3, 2, 6, P["gold2"])
    return img


def icon_task_overdue():
    img = icon_tasks(); d = ImageDraw.Draw(img)
    rect(d, 22, 5, 5, 16, P["red"])
    rect(d, 23, 7, 3, 8, "#ffd2c8")
    rect(d, 23, 17, 3, 3, "#ffd2c8")
    rect(d, 21, 4, 7, 2, P["ink"])
    rect(d, 21, 21, 7, 2, P["ink"])
    return img


def icon_task_completed():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 7, 6, 18, 20, P["cream"], P["brown"])
    rect(d, 10, 11, 8, 2, P["brown"])
    rect(d, 10, 15, 6, 2, P["brown"])
    rect(d, 12, 21, 4, 4, P["green"])
    rect(d, 16, 18, 3, 3, P["green"])
    rect(d, 19, 15, 3, 3, P["green"])
    sparkle(d, 24, 8, P["green2"])
    return img


def icon_task_recurring():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 6, 8, 20, 17, "#f7efe0", P["brown"])
    rect(d, 8, 11, 16, 3, P["gold"])
    rect(d, 11, 17, 3, 3, P["green"])
    rect(d, 17, 17, 3, 3, P["green"])
    rect(d, 10, 4, 3, 5, P["steel"])
    rect(d, 19, 4, 3, 5, P["steel"])
    rect(d, 22, 20, 4, 2, P["cyan"])
    rect(d, 24, 18, 2, 2, P["cyan"])
    rect(d, 20, 22, 2, 2, P["cyan"])
    return img


def icon_high_priority():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 15, 4, 2, 3, P["gold2"])
    rect(d, 13, 7, 6, 4, P["gold"])
    rect(d, 11, 11, 10, 6, P["orange"])
    rect(d, 9, 17, 14, 8, P["red"])
    rect(d, 8, 24, 16, 2, P["ink"])
    rect(d, 13, 12, 6, 2, "#fff0a8")
    rect(d, 14, 18, 4, 5, "#ffd6c8")
    sparkle(d, 6, 9)
    sparkle(d, 25, 13)
    return img


def icon_warning():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 15, 5, 2, 3, P["red2"])
    rect(d, 13, 8, 6, 5, P["red"])
    rect(d, 11, 13, 10, 6, P["orange"])
    rect(d, 9, 19, 14, 5, P["red"])
    rect(d, 8, 24, 16, 2, P["ink"])
    rect(d, 15, 12, 2, 7, "#fff0d0")
    rect(d, 15, 21, 2, 2, "#fff0d0")
    return img


def icon_info():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 12, 6, 8, 4, P["blue2"])
    rect(d, 10, 10, 12, 14, P["blue"])
    rect(d, 12, 24, 8, 2, P["ink"])
    rect(d, 15, 13, 2, 7, "#e6fbff")
    rect(d, 15, 10, 2, 2, "#e6fbff")
    sparkle(d, 24, 10, P["cyan"])
    sparkle(d, 7, 20, P["blue2"])
    return img


def icon_focus():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 9, 5, 14, 3, P["steel2"])
    rect(d, 9, 24, 14, 3, P["steel"])
    rect(d, 11, 8, 10, 3, P["ink"])
    rect(d, 11, 21, 10, 3, P["ink"])
    rect(d, 13, 11, 6, 4, P["gold"])
    rect(d, 15, 15, 2, 3, P["gold2"])
    rect(d, 13, 18, 6, 3, P["orange"])
    return img


def icon_daily_summary():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 6, 7, 20, 18, "#f7efe0", P["brown"])
    rect(d, 8, 10, 16, 3, P["gold"])
    for x in (10, 15, 20):
        rect(d, x, 15, 3, 3, P["green"])
        rect(d, x, 20, 3, 3, P["blue"])
    rect(d, 10, 4, 3, 5, P["steel"])
    rect(d, 19, 4, 3, 5, P["steel"])
    return img


def icon_idle():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 8, 9, 16, 14, "#1f2630", P["blue"])
    rect(d, 11, 13, 10, 2, P["steel2"])
    rect(d, 11, 17, 6, 2, P["steel"])
    rect(d, 23, 5, 4, 2, P["cyan"])
    rect(d, 25, 3, 2, 2, P["cyan"])
    rect(d, 25, 8, 3, 2, P["cyan"])
    rect(d, 10, 24, 12, 2, P["shadow"])
    return img


def icon_fellowship():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 6, 13, 6, 5, P["cream"])
    rect(d, 20, 13, 6, 5, P["cream"])
    rect(d, 10, 16, 12, 4, P["gold"])
    rect(d, 8, 18, 6, 4, P["brown"])
    rect(d, 18, 18, 6, 4, P["brown"])
    rect(d, 5, 12, 3, 7, P["ink"])
    rect(d, 24, 12, 3, 7, P["ink"])
    rect(d, 13, 14, 6, 2, "#ffe8ad")
    return img


def icon_notification_swarm():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 10, 6, 12, 3, P["purple2"])
    rect(d, 8, 9, 16, 11, P["purple"])
    rect(d, 11, 20, 3, 6, P["purple"])
    rect(d, 18, 20, 3, 6, P["purple"])
    rect(d, 11, 12, 4, 4, P["ink"])
    rect(d, 17, 12, 4, 4, P["ink"])
    rect(d, 15, 16, 2, 7, P["ink"])
    for x, y in [(5, 9), (25, 9), (6, 22), (24, 22), (3, 16), (28, 16)]:
        rect(d, x, y, 2, 2, P["purple2"])
    return img


def icon_level_up():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 15, 4, 2, 19, P["gold2"])
    rect(d, 11, 8, 10, 8, P["gold"])
    rect(d, 8, 13, 16, 5, P["gold"])
    rect(d, 12, 18, 8, 6, P["orange"])
    rect(d, 10, 24, 12, 2, P["ink"])
    sparkle(d, 7, 8)
    sparkle(d, 25, 17)
    rect(d, 14, 7, 4, 3, "#fff5ad")
    return img


def icon_achievement():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 10, 6, 12, 9, P["gold"])
    rect(d, 8, 8, 3, 5, P["gold2"])
    rect(d, 21, 8, 3, 5, P["gold2"])
    rect(d, 13, 15, 6, 5, P["brown"])
    rect(d, 11, 20, 10, 3, P["gold"])
    rect(d, 9, 23, 14, 3, P["ink"])
    rect(d, 13, 8, 6, 2, "#fff3a8")
    sparkle(d, 25, 7)
    return img


def icon_hydration():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 15, 5, 2, 2, P["blue2"])
    rect(d, 13, 7, 6, 4, P["blue2"])
    rect(d, 11, 11, 10, 8, P["blue"])
    rect(d, 10, 18, 12, 5, "#1c73a7")
    rect(d, 12, 23, 8, 2, P["ink"])
    rect(d, 12, 12, 3, 7, "#bff4ff")
    rect(d, 16, 10, 3, 4, "#8ce7ff")
    sparkle(d, 7, 10, P["blue2"])
    return img


def icon_sync():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 9, 18, 15, 6, P["steel"])
    rect(d, 11, 15, 11, 4, P["steel2"])
    rect(d, 14, 10, 5, 9, P["blue"])
    rect(d, 12, 12, 9, 3, P["blue2"])
    rect(d, 15, 8, 3, 2, P["blue2"])
    rect(d, 8, 24, 17, 2, P["ink"])
    return img


def icon_journal():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 9, 5, 15, 22, "#5d3a66", P["purple2"])
    rect(d, 11, 7, 10, 2, "#7b4e8b")
    rect(d, 15, 11, 3, 8, P["gold"])
    rect(d, 13, 13, 7, 3, P["gold2"])
    rect(d, 12, 25, 9, 2, P["red"])
    rect(d, 8, 8, 2, 17, P["ink"])
    return img


def icon_rewards():
    img = canvas(); d = ImageDraw.Draw(img)
    outline_box(d, 7, 11, 18, 13, P["brown"], P["ink"])
    rect(d, 9, 9, 14, 4, "#b76f35")
    rect(d, 12, 14, 8, 7, "#d89043")
    rect(d, 15, 15, 2, 4, P["gold2"])
    rect(d, 14, 13, 4, 2, P["steel2"])
    rect(d, 11, 24, 12, 2, P["shadow"])
    return img


def icon_daily_adventure():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 9, 6, 4, 18, P["steel2"])
    rect(d, 19, 6, 4, 18, P["steel2"])
    rect(d, 8, 20, 6, 3, P["gold"])
    rect(d, 18, 20, 6, 3, P["gold"])
    rect(d, 12, 15, 8, 3, P["ink"])
    rect(d, 7, 5, 7, 3, P["ink"])
    rect(d, 18, 5, 7, 3, P["ink"])
    sparkle(d, 16, 10)
    return img


def icon_evergrowth():
    img = canvas(); d = ImageDraw.Draw(img)
    rect(d, 14, 15, 5, 10, P["brown"])
    rect(d, 11, 20, 11, 3, "#6a3f29")
    rect(d, 8, 10, 16, 8, P["green"])
    rect(d, 10, 7, 12, 7, P["green2"])
    rect(d, 13, 5, 7, 6, "#c0e57f")
    rect(d, 6, 14, 5, 4, "#5d9a53")
    rect(d, 21, 13, 5, 5, "#5d9a53")
    sparkle(d, 24, 22, P["gold"])
    return img


ICONS = {
    "tasks": icon_tasks,
    "task_due": icon_task_due,
    "task_overdue": icon_task_overdue,
    "task_completed": icon_task_completed,
    "task_recurring": icon_task_recurring,
    "task_high_priority": icon_high_priority,
    "warning": icon_warning,
    "info": icon_info,
    "focus": icon_focus,
    "task_daily_summary": icon_daily_summary,
    "task_idle": icon_idle,
    "fellowship": icon_fellowship,
    "notification_swarm": icon_notification_swarm,
    "level_up": icon_level_up,
    "achievement": icon_achievement,
    "hydration": icon_hydration,
    "sync": icon_sync,
    "journal_notes": icon_journal,
    "rewards": icon_rewards,
    "daily_adventure": icon_daily_adventure,
    "evergrowth": icon_evergrowth,
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in ICONS.items():
        save(name, fn())

    manifest = {
        name: f"assets/icons/notifications/{name}.png"
        for name in ICONS
    }
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

    cols = 8
    rows = (len(ICONS) + cols - 1) // cols
    sheet = Image.new("RGBA", (cols * 128, rows * 128), (8, 12, 18, 255))
    for idx, name in enumerate(ICONS):
        icon = Image.open(OUT / f"{name}.png").convert("RGBA")
        sheet.alpha_composite(icon, ((idx % cols) * 128, (idx // cols) * 128))
    sheet.save(OUT / "preview_sheet.png")


if __name__ == "__main__":
    main()
