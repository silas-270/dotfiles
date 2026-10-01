#!/usr/bin/env python3
"""Spielt die Ruessel-Frames aus ruessel.txt (Studio-Datei) im Terminal ab, unten rechts
wie spaeter am Bildschirmrand, mit Auge und Braue.
Ablauf: mit Frame 1 von rechts reinschieben (Lauftempo des Elefanten), warten und einmal
blinzeln, Ruesselschwung, kurz warten, wieder rausschieben. Beenden auch mit Ctrl+C.

    ruessel_vorschau.py        Schwung aus ruessel.txt
    ruessel_vorschau.py lauf   stattdessen der Ruesselschwung aus dem Laufen (elefant.py)"""
import os, shutil, sys, time
from studio import load
import elefant

STEP = 0.13              # Sekunden pro Spalte beim Rein/Raus, wie FRAME_DELAY im Overlay
WAIT_IN = 1.5            # Sekunden nach dem Reinkommen, darin einmal blinzeln
BLINK_AT = 0.7           # ... nach so vielen Sekunden
WAIT_OUT = 1.0           # Sekunden nach dem Schwung, bevor er geht
FPS = 10                 # Ruesselschwung
TIMES = 2                # Schwuenge hintereinander (1 -> 6 -> 1)
COLOR = 33
# Auge im 20x16-Raster (Zeile, Spalte ab 0), die Braue eine Zeile darueber.
# Gleiche Stelle wie im Stehbild des Overlays, rechts neben der Stirnlinie.
EYE = (7, 19)
BROW = "^"
BLINK = [("-", 0.13), ("o", 0.13)]   # schnell zu, etwas langsamer wieder auf
# Laufraster (elefant.py) -> dieses Raster, gespiegelt: Spalte GRID_AT - c, Zeile + 3
GRID_AT, GRID_DOWN = 46, 3

def walk_frames(rest):
    """Frame 1 (Ruhe) -> [Ruhe, vorgeschwungen] mit dem Ruessel aus dem Laufen."""
    fwd = [row[:] for row in rest]
    mirror = str.maketrans("/\\()", "\\/)(")
    T = elefant.TRUNK
    for r, (c, a, b) in enumerate(zip(elefant.TRUNK_COLS, T["rest"], T["fwd"]), 3):
        for s in (" " * len(a), b):            # ruhenden weg, vorgeschwungenen hin
            for i, ch in enumerate(s):
                if GRID_AT - c - i < len(rest[0]):   # fuehrende Leerzeichen liegen hinterm Kopf
                    fwd[r + GRID_DOWN][GRID_AT - c - i] = ch.translate(mirror)
    return [rest, fwd]

def walk_swing():
    """Ruesseltakt aus dem Laufen, TIMES-mal: [(Frame, Sekunden)]"""
    return [(int(p == "fwd"), elefant.DELAY) for p in elefant.TRUNK_SEQ * TIMES]

def own_swing(n):
    """1 -> n -> 1, TIMES-mal, Frame 1 an den Anschluessen nur einmal"""
    return [(k, 1 / FPS) for k in (list(range(1, n)) + list(range(n - 2, -1, -1))) * TIMES]

def timeline(frames, w, swing):
    """-> [(Frame, Spalten rechts ausserhalb, Auge, Sekunden)]"""
    first = min(len(r) - len("".join(r).lstrip()) for f in frames[:1] for r in f
                if "".join(r).strip())
    width = w - first                    # sichtbare Breite von Frame 1
    t = [(0, off, "O", STEP) for off in range(width, 0, -1)]
    t += [(0, 0, "O", BLINK_AT)] + [(0, 0, e, d) for e, d in BLINK]
    t += [(0, 0, "O", WAIT_IN - BLINK_AT - sum(d for _, d in BLINK))]
    t += [(k, 0, "O", d) for k, d in swing]
    t += [(0, 0, "O", WAIT_OUT)]
    t += [(0, off, "O", STEP) for off in range(1, width + 1)]
    return t

def animation(lauf=False):
    """-> (Breite, Hoehe, Frames, timeline), auch fuers Overlay (SUPER+E an/aus)."""
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ruessel.txt")
    w, h, frames, _, _ = load(path)
    if lauf:
        frames = walk_frames(frames[0])
        swing = walk_swing()
    else:
        swing = own_swing(len(frames))
    return w, h, frames, timeline(frames, w, swing)

def main():
    w, h, frames, tl = animation(sys.argv[1:] == ["lauf"])
    r, c = EYE

    sys.stdout.write("\x1b[?1049h\x1b[?25l")
    try:
        for k, off, eye, delay in tl:
            g = [row[:] for row in frames[k]]
            g[r][c], g[r - 1][c] = eye, BROW
            cols, rows = shutil.get_terminal_size()
            top, left = max(0, rows - h), cols - w + off
            out = ["\x1b[H\x1b[2J"]
            for i, row in enumerate(g):
                line = "".join(row)[max(0, -left):w - off]   # was rechts raussteht, abschneiden
                out.append(f"\x1b[{top + i + 1};{max(0, left) + 1}H\x1b[{COLOR}m{line}\x1b[0m")
            sys.stdout.write("".join(out))
            sys.stdout.flush()
            time.sleep(delay)
    except KeyboardInterrupt:
        pass
    finally:
        sys.stdout.write("\x1b[?25h\x1b[?1049l")
        sys.stdout.flush()

if __name__ == "__main__":
    main()
