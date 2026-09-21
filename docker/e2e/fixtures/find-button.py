#!/usr/bin/env python3
"""Locates the hero's primary action button in a screenshot of the real app.

WHY PIXELS AND NOT A COORDINATE. Scenario 3's final claim is that a PERSON
clicking Update gets an update, which means a real pointer has to land on the
real button. A coordinate written down once is a coordinate that silently
starts missing the first time the hero's layout changes, and a click on empty
space looks exactly like a click that did nothing -- the scenario would fail
somewhere further along, blaming the wrong thing. Reading the pixels instead
means a layout change moves the click with it, and a layout change that
removes the button fails HERE, with the screenshot to look at.

WHAT IT LOOKS FOR. The hero's primary action (Install, Update, Start) is a
SOLID near-black pill: --primary is oklch(0.18 0 0) in the light theme, about
RGB(36,36,36). Three things separate it from everything else on the page:

  - it is in the content area, which rules out the sidebar's own dark Home
    button and the selected sidebar row;
  - it is FILLED, which rules out body text and headings, also near-black but
    only a fifth dark by area, and rules out the status badge, the tags and
    the version select, all of which are outlines on the page background;
  - it is the TOPMOST such pill, which rules out the tab strip's selected tab
    just below it -- the one other solid dark pill the page draws.

The fill test is what makes this safe rather than lucky: white label text cuts
holes in the button, so a naive scan for unbroken dark rows finds nothing at
all (it did), and a naive scan that stitches rows back together finds every
heading on the page too.

Prints "X Y" in the app window's own coordinate space, or exits non-zero.

Usage: find-button.py SHOT.png WIN_X WIN_Y WIN_W WIN_H
"""

import sys

from PIL import Image

# Left of this (in window coordinates) is the sidebar, which draws dark pills
# of its own. The sidebar is a fixed-width column; 260 clears it without
# reaching into the content area's own left gutter.
CONTENT_LEFT = 260

# "Near-black" rather than black: the pill is anti-aliased at its rounded ends
# and picks up the window's own compositing. Everything else the page fills is
# white, light grey, or a saturated accent (the icon tile is a strong blue,
# whose blue channel alone puts it far outside this).
DARK_MAX = 90

# Dark pixels closer than this on the same row belong to the same object. It
# has to clear the widest gap inside a button's own label -- the space between
# words, and the divider before the info trigger -- without reaching the next
# element on the row.
ROW_GAP = 28

# A hero action button. Generous on both ends: a long label ("Remove from
# Library") is wide, a short one ("Stop") is narrow.
MIN_W, MAX_W = 48, 360
MIN_H, MAX_H = 22, 60

# How much of the bounding box has to be dark for this to be a filled shape
# rather than text. A button with a white label runs about 0.8; a line of
# near-black body text runs about 0.2.
MIN_FILL = 0.55


def dark_mask(image):
	"""A per-row list of the x positions that are near-black."""
	pixels = image.load()
	width, height = image.size
	rows = []
	for y in range(height):
		row = [
			x
			for x in range(CONTENT_LEFT, width)
			if pixels[x, y][0] <= DARK_MAX
			and pixels[x, y][1] <= DARK_MAX
			and pixels[x, y][2] <= DARK_MAX
		]
		rows.append(row)
	return rows


def row_clusters(xs):
	"""Groups a row's dark x positions into (start, end) spans."""
	if not xs:
		return []
	spans = []
	start = prev = xs[0]
	for x in xs[1:]:
		if x - prev > ROW_GAP:
			spans.append((start, prev))
			start = x
		prev = x
	spans.append((start, prev))
	return spans


def overlaps(a0, a1, b0, b1):
	"""Whether two spans share at least half of the narrower one."""
	shared = min(a1, b1) - max(a0, b0) + 1
	if shared <= 0:
		return False
	return shared >= 0.5 * min(a1 - a0 + 1, b1 - b0 + 1)


def blocks(rows):
	"""Stacks vertically continuous, horizontally overlapping spans."""
	found = []
	open_blocks = []
	for y, xs in enumerate(rows):
		next_open = []
		for x0, x1 in row_clusters(xs):
			for block in open_blocks:
				if block["y1"] == y - 1 and overlaps(block["x0"], block["x1"], x0, x1):
					block["y1"] = y
					block["x0"] = min(block["x0"], x0)
					block["x1"] = max(block["x1"], x1)
					block["dark"] += x1 - x0 + 1
					next_open.append(block)
					break
			else:
				next_open.append(
					{"x0": x0, "x1": x1, "y0": y, "y1": y, "dark": x1 - x0 + 1}
				)
		found.extend(b for b in open_blocks if b not in next_open)
		open_blocks = next_open
	found.extend(open_blocks)
	return found


def fill(block):
	area = (block["x1"] - block["x0"] + 1) * (block["y1"] - block["y0"] + 1)
	return block["dark"] / area if area else 0.0


def main():
	if len(sys.argv) != 6:
		print(__doc__, file=sys.stderr)
		return 2

	shot = sys.argv[1]
	win_x, win_y, win_w, win_h = (int(value) for value in sys.argv[2:6])

	screen = Image.open(shot).convert("RGB")
	# Crop to the window, so everything below is in the window's own
	# coordinates and the desktop wallpaper behind it -- which is itself dark
	# -- cannot contribute a match.
	window = screen.crop(
		(
			win_x,
			win_y,
			min(win_x + win_w, screen.width),
			min(win_y + win_h, screen.height),
		)
	)

	candidates = [
		block
		for block in blocks(dark_mask(window))
		if MIN_W <= block["x1"] - block["x0"] + 1 <= MAX_W
		and MIN_H <= block["y1"] - block["y0"] + 1 <= MAX_H
		and fill(block) >= MIN_FILL
	]

	if not candidates:
		print(f"no filled dark button in the content area of {shot}", file=sys.stderr)
		return 1

	# Topmost wins: the hero's action row sits above the tab strip, whose
	# selected tab is the only other solid dark pill on the page.
	chosen = min(candidates, key=lambda block: (block["y0"], block["x0"]))

	# Thirty per cent in from the left edge, not the centre. The pill is two
	# controls sharing one dark fill: the action itself, and an info trigger
	# occupying the rightmost ~32px that opens a step preview instead of
	# running anything. The centre of a short label's pill can land on that
	# trigger, which would open a dialog and quietly never start the update.
	width = chosen["x1"] - chosen["x0"] + 1
	print(
		f"{chosen['x0'] + width * 30 // 100} "
		f"{(chosen['y0'] + chosen['y1']) // 2}"
	)
	print(
		"candidates: "
		+ ", ".join(
			f"{b['x1'] - b['x0'] + 1}x{b['y1'] - b['y0'] + 1}@({b['x0']},{b['y0']}) "
			f"fill={fill(b):.2f}"
			for b in sorted(candidates, key=lambda b: b["y0"])
		),
		file=sys.stderr,
	)
	return 0


if __name__ == "__main__":
	sys.exit(main())
