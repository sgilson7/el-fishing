"""The game's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the game the way a person would: start the Open Swell,
cast, hold the scope, open the settings. Expected values are worked out by
hand from the data files (C4), not read from the game's own output. Waits are
on selectors and data-* attributes the page sets, never on a fixed time
alone: a key has to be held for a while, so holds wait on the game's tick.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
STRINGS = json.loads((ROOT / "data" / "strings.json").read_text())
GEAR = json.loads((ROOT / "data" / "gear.json").read_text())


def ticks(page):
    return int(page.get_attribute("body", "data-tick") or "0")


def hold(page, keys, n):
    """Hold keys down for n game ticks (60 a second)."""
    start = ticks(page)
    for k in keys:
        page.keyboard.down(k)
    page.wait_for_function(f"() => Number(document.body.dataset.tick) >= {start + n}", timeout=30000)
    for k in keys:
        page.keyboard.up(k)


def run(page, check, engine):
    check(page.get_attribute("body", "data-screen") == "title", "the core's title screen is shown first")
    check(page.inner_text("#title-card h2") == STRINGS["title"], "the title from data/strings.json is drawn")
    # seven how-to lines and one for each of the two species in data/place.json
    check(page.locator("#how li").count() == 9, "the how-to lines and one line for each species are listed")

    page.click("#title-card .btn.go")
    page.wait_for_selector("body[data-screen='fishing']")
    check(True, "the start button opens the Open Swell")
    # the eyebrow is set in capitals by CSS, and innerText reports them so
    check(STRINGS["place.open_swell"].lower() in page.inner_text("#place").lower(), "the HUD names the place")
    # 120 cm of line out at the start (data/gear.json start_out_cm)
    start_out = GEAR["line"]["start_out_cm"] / 100
    page.wait_for_function("() => Number(document.body.dataset.tick) > 2")
    check(abs(float(page.get_attribute("body", "data-out")) - start_out) < 0.05, f"line out starts at {start_out} m")
    # the swell worked by hand from data/place.json: Hs = 4 sqrt(sum a^2 / 2) with
    # amplitudes 0.55 0.35 0.25 0.18 0.10 0.06 0.03 0.015 m is 2.07 m; the tallest
    # wave is 126 m long, so its period is sqrt(2 pi 126 / 9.81) = 8.98 s
    check("2.1 m, 9 s" in page.inner_text("#place"), "the swell reads 2.1 m, 9 s, as worked by hand")
    check(page.is_visible("#ring"), "the tension ring is shown")
    check(page.is_visible("#arms"), "the arm-key panel is open before the first catch")

    # a cast: finger on the line, rod back with I, forward with K, let go of F
    page.keyboard.down("f")
    hold(page, ["i"], 28)
    page.keyboard.down("k")
    hold(page, [], 9)
    page.keyboard.up("f")
    hold(page, [], 14)
    page.keyboard.up("k")
    hold(page, [], 90)
    out = float(page.get_attribute("body", "data-out"))
    check(out > 5, f"a cast lets line run out past 5 m ({out} m)")
    check(page.get_attribute("body", "data-bail") == "open", "the bail is open after the cast")

    # Space closes the bail and winds: line out falls
    hold(page, ["Space"], 60)
    after = float(page.get_attribute("body", "data-out"))
    check(page.get_attribute("body", "data-bail") == "closed", "Space closes the bail")
    check(after < out, f"winding brings line in ({out} m to {after} m)")

    # the scope: shown while the right mouse button is held, gone after
    box = page.locator("#gl").bounding_box()
    page.mouse.move(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
    page.mouse.down(button="right")
    page.wait_for_selector("#scope:not([hidden])")
    check(True, "holding the right mouse button shows the scope")
    page.mouse.up(button="right")
    page.wait_for_selector("#scope[hidden]", state="attached")
    check(True, "letting go hides the scope")

    # the settings, and back
    page.keyboard.press("Escape")
    page.wait_for_selector("body[data-screen='settings']")
    check(page.locator("#settings .chip").count() == 8, "the settings offer arm speed, steady hands and steady hull")
    page.click("#settings .btn.go")
    page.wait_for_selector("body[data-screen='fishing']")
    check(True, "the resume button returns to fishing")
    check(page.inner_text("#error") == "", "no error message is shown")
