#!/usr/bin/env python3
"""Laufender Elefant im Terminal. Beenden mit Ctrl+C."""
import random, shutil, sys, time

DELAY = 0.13   # Sekunden pro Frame
COLOR = 33     # ANSI-Farbe (33 = gelb, wie im fastfetch-Logo)
FAR_SHIFT = 2  # ferne Beine: Versatz nach rechts (Perspektive)
FAR_DIM = True # ferne Beine abgedunkelt

BODY = r"""
                        ____
                   .---'-    \
      .-----------/           \
     /           (         ^  |   __
    (             \        O  /  / .'
    (              '-'  (.   (_.' /
     \                    \     ./
      |    |       |    |/ '._.'
"""

# Beinposen, Zeilen 8-10. Phase 0 = Fuss setzt vorne auf, 0-4 Standphase
# (Fuss wandert 1 Spalte pro Frame nach hinten), 5-7 Schwungphase (Fuss angehoben).
HIND = [
 [r"      \   @\  ", r"       \    \ ", r"        '_:::\ "],
 [r"       )   @) ", r"       |    | ", r"       '_:::\ "],
 [r"       )   @) ", r"      /    /  ", r"      '_:::\ "],
 [r"      |   @|  ", r"     /    /   ", r"     '_:::\ "],
 [r"     /   @/   ", r"    /    /    ", r"    '_:::\ "],
 [r"     /   @/   ", r"     '_:::\   ", r""],
 [r"      |   @|  ", r"      '_:::\  ", r""],
 [r"       )   @) ", r"       '_:::\ ", r""],
]
FORE = [
 [r"                   \  @ \ ", r"                    \    \ ", r"                     '_:::\ "],
 [r"                   |  @ | ", r"                   (    | ", r"                    '_:::\ "],
 [r"                   |  @ | ", r"                   |    | ", r"                   '_:::\ "],
 [r"                   |  @ | ", r"                  /    / ", r"                  '_:::\ "],
 [r"                  /  @ / ", r"                 /    / ", r"                 '_:::\ "],
 [r"                  /  @ / ", r"                  '_:::\ ", r""],
 [r"                   |  @ | ", r"                   '_:::\ ", r""],
 [r"                   \  @ \ ", r"                    '_:::\ ", r""],
]
# Passgang-Viertakt: Aufsetzen hinten rechts (nah), vorne rechts, hinten links, vorne links.
# (Posen, Phasenversatz in Frames)
LEGS_FAR = [(HIND, 4), (FORE, 6)]
LEGS_NEAR = [(HIND, 0), (FORE, 2)]

TAIL = {
 "up":   [(4, 0, "&"), (5, 0, "'._/")],
 "mid":  [(5, 0, "&._/")],
 "down": [(5, 1, "._/"), (6, 0, "&'")],
}
TAIL_SEQ = ["up", "up", "mid", "down", "down", "down", "mid", "up"]

# Ruecken (Zeile 2, Spalten 6-17): Jedes Bein wirkt wie ein umgekipptes Pendel. Am
# tiefsten ist der Koerper, wenn beide Beine eines Paares gleichzeitig stehen
# (eins setzt vorne auf, eins drueckt hinten ab). Dann federt die Huefte samt Ecke
# zum Hinterteil bzw. die Schulter bis zum Nacken eine halbe Zeile ein ("-" -> "_").
BACK = {"hip": "____.-------", "shoulder": ".-------.___"}

# Ohr schwingt nach vorn, solange das nahe Vorderbein schwingt (einmal pro Zyklus).
EAR = [(3, 17, "("), (4, 18, "\\"), (5, 19, "'-'")]
EAR_OUT = [(3, 18, "("), (4, 19, "\\"), (5, 20, "'-'")]

# Ruessel pendelt um eine Spalte: Die Originalkurve wird unveraendert als Ganzes
# nach vorn verschoben, nur der Ansatz am Gesicht wird laenger. Dabei spannt er
# sich, der Durchhang der Schlaufe wird flacher. Er schwingt einen Frame nach dem
# Koerper: erst federt die Schulter ein, dann schwingt der Ruessel vor.
# Zeilen 3-7, jeweils ab Startspalte.
TRUNK_COLS = [31, 31, 29, 27, 26]
TRUNK = {
 "rest": [r"   __",  r"  / .'",  r"(_.' /",  r"     ./",  r" '._.'"],
 "fwd":  [r"    __", r"   / .'", r"(__.' /", r"      ./", r" '----'"],
}
TRUNK_SEQ = ["rest", "rest", "rest", "fwd", "fwd", "fwd", "fwd", "rest"]

# Auge blinzelt unregelmaessig alle paar Sekunden, unabhaengig vom Laufzyklus:
# schnell zu ("-"), etwas langsamer wieder auf ("o"). Frames innerhalb der Periode.
EYE = (4, 27)
BLINK_PERIOD = 101
BLINKS = (9, 40, 45, 88)

FRAMES = 8
WIDTH, HEIGHT = 40, 11

def compose(k):
    """Frame k -> (Zeilen, Dimm-Maske)."""
    g = [list(l.ljust(WIDTH)) for l in BODY.strip("\n").split("\n")]
    g += [[" "] * WIDTH for _ in range(HEIGHT - len(g))]
    dim = [[False] * WIDTH for _ in range(HEIGHT)]

    def put(r, c, s, d=False, clear=False):
        for i, ch in enumerate(s):
            if ch != " " or clear:
                g[r][c + i] = ch; dim[r][c + i] = d

    def leg(poses, off, shift=0, d=False):
        rows = poses[(k - off) % FRAMES]
        for i, s in enumerate(rows):
            if d: s = s.replace("@", " ")    # ferne Beine ohne Knie-Markierung
            if not s.strip(): continue
            a, b = len(s) - len(s.lstrip()), len(s.rstrip())
            for c in range(a, b):            # zwischen den Kanten deckend
                g[8 + i][c + shift] = s[c]; dim[8 + i][c + shift] = d
        return rows

    for r, c, s in TAIL[TAIL_SEQ[k % FRAMES]]:
        put(r, c, s)
    for r, (c, s) in enumerate(zip(TRUNK_COLS, TRUNK[TRUNK_SEQ[k % FRAMES]]), 3):
        put(r, c, s.ljust(WIDTH - c), clear=True)
    def both_standing(i):   # i = 0: Hinterbeine, 1: Vorderbeine
        return all((k - legs[i][1]) % FRAMES < 5 for legs in (LEGS_NEAR, LEGS_FAR))
    if both_standing(0): put(2, 6, BACK["hip"])
    elif both_standing(1): put(2, 6, BACK["shoulder"])
    if (k - LEGS_NEAR[1][1]) % FRAMES >= 5:
        for r, c, t in EAR: put(r, c, " " * len(t), clear=True)
        for r, c, t in EAR_OUT: put(r, c, t)
    t = k % BLINK_PERIOD
    if t in BLINKS: put(*EYE, "-")
    elif t - 1 in BLINKS: put(*EYE, "o")
    for poses, off in LEGS_FAR:
        leg(poses, off, FAR_SHIFT, FAR_DIM)
    hind = HIND[k % FRAMES][0]; fore = FORE[(k - 2) % FRAMES][0]
    be = len(hind.rstrip()); fs = len(fore) - len(fore.lstrip())
    end = "\\" if fore.strip()[0] in "|(" else "_"
    for c in range(be, fs): g[8][c] = " "; dim[8][c] = False   # Bauch verdeckt ferne Beine
    put(8, be, "." + "_" * (fs - be - 2) + end)
    for poses, off in LEGS_NEAR:
        leg(poses, off)
    return ["".join(r) for r in g], dim

def make_ground(length=157):
    rng = random.Random(7)
    tufts = ["\\|,", "..\\).", ". ..", ",", ".", "\\|/", "."]
    g = [" "] * length
    x = 0
    while True:
        x += rng.randint(4, 11)
        t = rng.choice(tufts)
        if x + len(t) >= length:
            return "".join(g)
        g[x:x + len(t)] = t
        x += len(t)

GROUND = make_ground()

def render(k, cols, rows):
    lines, dim = compose(k)
    left = max(0, (cols - WIDTH) // 2)
    top = max(0, (rows - HEIGHT) // 2)
    out = [" " * cols] * top
    for r, line in enumerate(lines):
        line = (" " * left + line).ljust(cols)[:cols]
        d = ([False] * left + dim[r] + [False] * cols)[:cols]
        if r == HEIGHT - 1:   # Bodenzeile: Gras scrollt nach links, 1 Spalte Abstand zu den Fuessen
            ground = "".join(GROUND[(c + k) % len(GROUND)] for c in range(cols))
            pad = " " + line + " "
            line = "".join(line[c] if pad[c:c + 3] != "   " else ground[c] for c in range(cols))
        s, cur = "", False
        for ch, dd in zip(line, d):
            if dd != cur:
                s += "\033[2m" if dd else "\033[22m"; cur = dd
            s += ch
        out.append(s + ("\033[22m" if cur else ""))
    out += [" " * cols] * (rows - len(out))
    return "\n".join(out[:rows])

def main():
    out = sys.stdout
    out.write("\033[?1049h\033[?25l\033[%dm" % COLOR)
    try:
        k = 0
        while True:
            cols, rows = shutil.get_terminal_size()
            out.write("\033[H" + render(k, cols, rows))
            out.flush()
            time.sleep(DELAY)
            k += 1
    except KeyboardInterrupt:
        pass
    finally:
        out.write("\033[0m\033[?25h\033[?1049l")
        out.flush()

if __name__ == "__main__":
    main()
