#!/usr/bin/env python3
"""Elefant laeuft als Overlay ueber den Desktop (Hyprland, gtk-layer-shell).

Von rechts reinlaufen (gespiegelt) bis zu einem Viertel der Bildschirmbreite, dort die
Drehung (links -> rechts), dann nach rechts wieder raus. Klicks gehen durch.

Die Lauf- und Drehanimation sind hier eingebettet. Die Einzelversionen fuers Terminal
liegen in dotfiles/themes/savanna-dusk/ (elefant.py, elefant_drehung.py)."""
import os, random
import cairo, gi
gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")
gi.require_version("GtkLayerShell", "0.1")
from gi.repository import GLib, Gdk, Gtk, GtkLayerShell

# ---------------------------------------------------------------------------------------
# Laufanimation (aus elefant.py)
# ---------------------------------------------------------------------------------------

FRAME_DELAY = 0.13   # Sekunden pro Frame
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

# ---------------------------------------------------------------------------------------
# Drehanimation (aus elefant_drehung.py)
# ---------------------------------------------------------------------------------------

# Bild 1 ist die Startstellung (rechts schauend), 2-32 die Drehung (Bein heben, Vorderhaelfte
# dreht + stampft, Hinterbein heben, Hinterteil zieht nach + stampft), 33 die gespiegelte
# Zielstellung. Der Rueckweg (34-64) ist das Spiegelbild von 2-32.
# Bilder 1-33: (Art: H halten / L Bein heben / S stampfen, Bild, Maske mit "#" = abgedunkelt)
TURN_FRAMES = [
 ("H", r"""
                               ____
                          .---'-    \
             .-----------/           \
            /           (         ^  |   __
       &   (             \        O  /  / .'
       '._/(              '-'  (.   (_.' /
            \                    \     ./
             |    |       |    |/ '._.'
              )   @).____\|  @ | |
             /    / /     (    | |
             '_:::\:\      '_:::\:\
""", r"""








                                 #
                    #            #
                   ##            ##
"""),   # Bild 1
 ("L", r"""
                               ____
                          .---'-    \
             .-----------/           \
            /           (         ^  |   __
       &   (             \        O  /  / .'
       '._/(              '-'  (.   (_.' /
            \                    \     ./
             |    |       |    |/ '._.'
              )   @).____\|  @ | |
             /    / /      '_:::\|
             '_:::\:\        '_:::\
""", r"""








                                 #
                    #            #
                   ##        ######
"""),   # Bild 2
 ("S", r"""
                              ____
                           .---'-  \
             .------------/         \
            /            (       ^  |   __
       &   (              \      O  /  / .'
       '._/(               '-' (.  (_.' /
            \                   \     ./
             |    |       |    |/ '._.'
              )   @).____\|  @ |
             /    / /     (    |  '
             '_:::\:\      '_:::\.
""", r"""









                    #
                   ##
"""),   # Bild 3
 ("L", r"""
                              ____
                           .---'-  \
             .------------/         \
            /            (       ^  |   __
       &   (              \      O  /  / .'
       '._/(               '-' (.  (_.' /
            \                   \     ./
             |    |       |    |/ '._.'
              )   @).____\|  @ |
             '_:::\ /   ( (    |
               '_:::\    '_'_:::\
""", r"""









                    #   #
               ######    ##
"""),   # Bild 4
 ("S", r"""
                              ____
                           .---'-  \
              .-----------/         \
             /           (       ^  |   __
        &   (             \      O  /  / .'
        '._/(              '-' (.  (_.' /
             \                  \     ./
              |    |      |    |/ '._.'
               )   @).___\|  @ |
              /    / '  ( (    |
              '_:::\.    '_'_:::\
""", r"""









                        #
                         ##
"""),   # Bild 5
 ("L", r"""
                              ____
                           .---'-  \
              .-----------/         \
             /           (       ^  |   __
        &   (             \      O  /  / .'
        '._/(              '-' (.  (_.' /
             \                  \     ./
              |    |      |    |/ '._.'
             ) )   @).___\|  @ |
            / /    /    (  '_:::\
            '_'_:::\     '_:::\
""", r"""








             #
            #           #
            ##           ######
"""),   # Bild 6
 ("S", r"""
                            ____
                           .--'  \
              .-----------/       \
             /            (     ^ | __
        &   (              \    o / /.'
        '._/(               '-'  (_.'/
             \                  \ ./
              |    |       |    |'-'
             ) )   @).____\|  @ |
            / /    /     ( (    |  '
            '_'_:::\      '_'_:::\.
""", r"""








             #
            #            #
            ##            ##
"""),   # Bild 7
 ("L", r"""
                            ____
                           .--'  \
              .-----------/       \
             /            (     ^ | __
        &   (              \    o / /.'
        '._/(               '-'  (_.'/
             \                  \ ./
              |    |       |    |'-'
             ) )   @).____\|  @ |
            '_/    /     ( (    |
              '_:::\      '_'_:::\
""", r"""








             #
            ##           #
                          ##
"""),   # Bild 8
 ("S", r"""
                            ____
                           .--'  \
               .----------/       \
             /            (     ^ | __
            (              \    o / /.'
        &._/(               '-'  (_.'/
             \                  \ ./
            |  |   |       |    |'-'
            |  |  @|._____\|  @ |
         '  |  /   /     ( (    |
          .(__'_::\       '_'_:::\
""", r"""









                         #
                          ##
"""),   # Bild 9
 ("L", r"""
                            ____
                           .--'  \
               .----------/       \
             /            (     ^ | __
            (              \    o / /.'
        &._/(               '-'  (_.'/
             \                  \ ./
            |  |   |       |    |'-'
            |  |  @|._____\|  @ |
            |  /   /     (  '_:::\
           (__'_::\       '_:::\
""", r"""









                         #
                          ######
"""),   # Bild 10
 ("S", r"""
                 .--.     ____
                .'      .'    '.
               .----------      \
             /            (      |
            (              \   o /
        &._/(               '  (_ /
             \                 \_/
            |  |   |    |    |
            |  |  @|.__\|  @ |
            |  /   /   '(    |  '
           (__'_::\     .'_:::\.
""", r"""
                 ####
                ##









"""),   # Bild 11
 ("L", r"""
                 .--.   ____   .-.
                .'    .'    '.   '.
               .-----'-----   \   )
             /             (   | '
            (              ( . /
        &._/(                 ( \
             \                '-'
            |  |   |    |    |
            |  |  @|.__\|  @ |
            | '_::\     (    |
           (___)         '_:::\
""", r"""
                 ####          ###
                ##               ##
                                  #
                                 #







"""),   # Bild 12
 ("S", r"""
                 .--.   ____   .-.
                .'    .'    '.   '.
               .-'''''-----   \   )
              /  |         (   | '
             (   |         ( . /
             (  /             ( \
             ( &              '-'
             \       \_ |    |
             |  . .  |  |  @ |
             |  | |  |  (    |
            (___) (___)  '_:::\
""", r"""
                 ####          ###
                ##               ##
                                  #
                                 #







"""),   # Bild 13
 ("L", r"""
                 .--.   ____   .-.
                .'    .'    '.   '.
               .-'''''-----   \   )
              /  |         (   | '
             (   |         ( . /
             (   |            ( \
             (   &            '-'
             \       \_ |    |
             |  . .  |  |  @ |
             |  | |  |   '_:::\
            (___) (___)
""", r"""
                 ####          ###
                ##               ##
                                  #
                                 #







"""),   # Bild 14
 ("S", r"""
             .--.   .----.   .--.
           .'    './      \.'    '.
          (    .-'''''''''-.       )
           '. /  |          \    .'
             (   |           )
             (   |           )
             (   &           )
             \               /
             |  . .  | |    |
             |  | |  | |    |  '
            (___) (___)(_____).
""", r"""
             ####   ######   ####
           ##    ###      ###    ##
          #                        #
           ##                    ##




                       #    #
                       #    #
                       #######
"""),   # Bild 15
 ("L", r"""
            .--.   .----.   .--.
          .'    './      \.'    '.
         /     .-'''''''''-.      \
        (     /  |          \      )
         \   (   |           )    /
          '-.(   |           ) .-'
             (   &           )
             \               /
             |  . .  | |    |
            (___) |  | |    |
                  (___)(_____)
""", r"""
            ####   ######   ####
          ##    ###      ###    ##
         #                        #
        #                          #
         #                        #
          ###                  ###


                       #    #
                       #    #
                       #######
"""),   # Bild 16
 ("S", r"""
            .--.   .----.   .--.
          .'    './      \.'    '.
         /      .-''''''''-.      \
        (      /     |      \      )
         \    (      |       )    /
          '-. (     /        ) .-'
              (    &         )
              \              /
               |    .  .    |
            '  |    |  |    |
             .(_____)  (_____)
""", r"""
            ####   ######   ####
          ##    ###      ###    ##
         #                        #
        #                          #
         #                        #
          ###                  ###





"""),   # Bild 17
 ("S", r"""
           .--.   .----.   .--.
         .'    './      \.'    '.
        (       .-''''''''-.     )
         '.    /     |      \  .'
              (      |       )
              (      \       )
              (       &      )
              \              /
               |    .  .    |
               |    |  |    |
              (_____)  (_____)
""", r"""
           ####   ######   ####
         ##    ###      ###    ##
        #                        #
         ##                    ##







"""),   # Bild 18
 ("L", r"""
          .-.   ____   .--.
         .'   .'    '.    '.
         (   /   -----''''-.
          ' |   )     |     \
            \ . )     |      )
           / )        /      )
           '-'       &       )
                             /
               |    .  .    |
               |    |  (_____)
              (_____)
""", r"""
          ###          ####
         ##               ##
         #
          #







"""),   # Bild 19
 ("S", r"""
          .-.   ____   .--.
         .'   .'    '.    '.
         (   /   -----'''''-.
          ' |   )         |  \
            \ . )         |   )
           / )           /    )
           '-'          &     )
              |    | _/       /
              | @  |  |  . .  |
              |    )  |  | |  |  '
             /:::_'  (___) (___).
""", r"""
          ###          ####
         ##               ##
         #
          #







"""),   # Bild 20
 ("L", r"""
          .-.   ____   .--.
         .'   .'    '.    '.
         (   /   -----'''''-.
          ' |   )         |  \
            \ . )         |   )
           / )            |   )
           '-'            &   )
              |    | _/       /
              | @  |  |  . .  |
             /:::_'   |  | |  |
                     (___) (___)
""", r"""
          ###          ####
         ##               ##
         #
          #







"""),   # Bild 21
 ("S", r"""
              ____
            .'    '.
           /      -----''''-.
          |      )        |  \
          \ o   /         |   )
         \ _)  '          |   )
          \_/             &   )
           |    |             /
           | @  | /___|  . .  |
        '  |    ) )   |  | |  |
         ./:::_'_'   (___) (___)
""", r"""









                  #
                ##
"""),   # Bild 22
 ("L", r"""
            ____
          /  '--.
         /       \-----''''-.
      __ | ^     )        |  \
     '.\ \ o    /         |   )
      \'._)  '-'          |   )
        \. /              &   )
        '-'|    |             /
           | @  | /___|  . .  |
           |    ) )  (___) |  |
          /:::_'_'         (___)
""", r"""









                  #
                ##
"""),   # Bild 23
 ("S", r"""
            ____
          /  '--.
         /       \----------.
      __ | ^     )            \
     '.\ \ o    /              )
      \'._)  '-'               )\
        \. /                  /  &
        '-'|    |       |   |  |
           | @  | /____.|@  |  |
           |    ) )    '\   \  |
          /:::_'_'      ./::_'__)
""", r"""









                  #
                ##
"""),   # Bild 24
 ("L", r"""
            ____
          /  '--.
         /       \----------.
      __ | ^     )            \
     '.\ \ o    /              )
      \'._)  '-'               )\_.&
        \. /                  /
        '-'|    |       |   |  |
           | @  | /____.|@  |  |
          /:::_'  )     \   \  |
            /:::_'       /::_'__)
""", r"""









                  #
            ######
"""),   # Bild 25
 ("S", r"""
          ____
        /  -'---.
       /         \----------.
  __   |  ^       )           \
 '. \  \  O      /             )
   \ '._)  .) '-'              )\_.&
    \.     /                  /
     '._.' \|    |      |   |  |
            | @  |/____.|@  |  |
         '  |    ) )    \   \  |
          ./:::_'_'      /::_'__)
""", r"""









                   #
                 ##
"""),   # Bild 26
 ("L", r"""
          ____
        /  -'---.
       /         \----------.
  __   |  ^       )           \
 '. \  \  O      /             )
   \ '._)  .) '-'              )\_.&
    \.     /                  /
     '._.' \|    |      |   |  |
            | @  |/____.|@  |  |
            |    ) )    \   \___)
           /:::_'_'      /::_'
""", r"""









                   #
                 ##
"""),   # Bild 27
 ("S", r"""
          ____
        /  -'---.
       /         \-----------.
  __   |  ^       )           \
 '. \  \  O      /             )   &
   \ '._)  .) '-'              )\_.'
    \.     /                  /
     '._.' \|    |      |    |
            | @  |/___.(@   (
            |    ) )    \    \   '
           /:::_'_'     /:::_'  .
""", r"""









                   #
                 ##
"""),   # Bild 28
 ("L", r"""
          ____
        /  -'---.
       /         \-----------.
  __   |  ^       )           \
 '. \  \  O      /             )   &
   \ '._)  .) '-'              )\_.'
    \.     /                  /
     '._.' \|    |      |    |
            | @  |/___.(@   ( (
           /:::_'  )    \    \ \
             /:::_'     /:::_'_'
""", r"""








                              #
                   #           #
             ######           ##
"""),   # Bild 29
 ("S", r"""
         ____
       /    -'---.
      /           \----------.
 __   |  ^         )          \
'. \  \  O        /            )   &
  \ '._)   .)  '-'             )\_.'
   \.     /                   /
     '._.' \|    |      |    |
            | @  |/___.(@   ( (
            |    )'     \    \ \
           /:::_'.      /:::_'_'
""", r"""








                              #
                               #
                              ##
"""),   # Bild 30
 ("L", r"""
         ____
       /    -'---.
      /           \----------.
 __   |  ^         )          \
'. \  \  O        /            )   &
  \ '._)   .)  '-'             )\_.'
   \.     /                   /
     '._.' \|    |      |    |
          | | @  |/___.(@   ( (
          | |    )      /:::_' \
         /:/:::_'         /:::_'
""", r"""








          #                   #
          #                    #
         ##               ######
"""),   # Bild 31
 ("S", r"""
         ____
       /    -'---.
      /           \-----------.
 __   |  ^         )           \
'. \  \  O        /             )   &
  \ '._)   .)  '-'              )\_.'
   \.     /                    /
     '._.' \|    |       |    |
          | | @  |/____.(@   (
          | |    )       \    \ '
         /:/:::_'        /:::_'.
""", r"""








          #
          #
         ##
"""),   # Bild 32
 ("H", r"""
         ____
       /    -'---.
      /           \-----------.
 __   |  ^         )           \
'. \  \  O        /             )   &
  \ '._)   .)  '-'              )\_.'
   \.     /                    /
     '._.' \|    |       |    |
          | | @  |/____.(@   (
          | |    )     \ \    \
         /:/:::_'      /:/:::_'
""", r"""








          #
          #            #
         ##            ##
"""),   # Bild 33
]

TURN_WIDTH = 44            # Breite der Drehbilder (das Laufraster ist 40 breit)
LIFT = 0.12                # Sekunden: Bein angehoben
STOMP = 0.28               # Sekunden: Drehen + Aufstampfen
HOLD = 0.3                 # Sekunden Pause in Anfangs-/Endstellung der Drehung
TURN_SPEED = 2.0           # Drehung schneller als der Rest (1 = Originaltempo)
TURN_DELAY = {"H": HOLD, "L": LIFT, "S": STOMP}
MIRROR = str.maketrans("/\\()", "\\/)(")

def rows(text):
    r = text.split("\n")[1:-1]
    return [l.ljust(TURN_WIDTH)[:TURN_WIDTH] for l in r] + [" " * TURN_WIDTH] * (HEIGHT - len(r))

def parse(art, mask):
    return rows(art), [[ch != " " for ch in l] for l in rows(mask)]

def mirror(frame):
    art, dim = frame
    return [l[::-1].translate(MIRROR) for l in art], [r[::-1] for r in dim]

def build_turn():
    """Drehung links -> rechts: Bild 33, gespiegelte Bilder 2-32, Bild 1; (Bild, Sekunden)."""
    there = [(parse(a, m), TURN_DELAY[k]) for k, a, m in TURN_FRAMES]
    back = [(mirror(f), t) for f, t in there[1:-1]]
    return [(f, t / TURN_SPEED) for f, t in there[-1:] + back + there[:1]]

# ---------------------------------------------------------------------------------------
# Overlay
# ---------------------------------------------------------------------------------------

# Zeichen wie im kitty-Terminal: JetBrainsMono Nerd Font 11pt (= 14.67px bei 96 dpi),
# Zelle 9x20 px (aus dem Terminal ausgemessen).
FONT = "JetBrainsMono Nerd Font"
SIZE = 11 * 96 / 72
CELL_W, CELL_H, ASCENT = 9, 20, 15

def theme_color(name, default):
    """Farbe aus dem kitty-Theme, damit sie exakt wie im Terminal ist."""
    try:
        with open(os.path.expanduser("~/.config/theme/generated/colors.conf")) as f:
            for line in f:
                key, _, val = line.partition(" ")
                if key == name:
                    h = val.strip().lstrip("#")
                    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    except OSError:
        pass
    return default

COLOR = theme_color("color3", (1.0, 0.55, 0.0))   # ANSI 33 (gelb, wie im fastfetch-Logo)
DIM_ALPHA = 0.4            # kitty dim_opacity (Standard) fuer abgedunkelte Zeichen
MARGIN_BOTTOM = 8
STOP_FRACTION = 4          # Elefant kommt bis 1/4 der Bildschirmbreite
GRASS_AHEAD = WIDTH // 2   # Gras waechst eine halbe Elefantenlaenge voraus
# Die Drehbilder sind gegenueber dem Laufbild versetzt (Auge als Referenz):
# gespiegelt (Start) +3 Spalten, nativ (Ende) +7 Spalten bezogen auf das Laufraster.
TURN_SHIFT_IN, TURN_SHIFT_OUT = 3, 7

class Overlay(Gtk.Window):
    def __init__(self):
        super().__init__()
        self.cw, self.lh, self.ascent = CELL_W, CELL_H, ASCENT

        self.set_app_paintable(True)
        self.set_visual(self.get_screen().get_rgba_visual())
        self.set_decorated(False)
        width = Gdk.Display.get_default().get_monitor(0).get_geometry().width
        self.set_default_size(width, int(HEIGHT * self.lh))
        GtkLayerShell.init_for_window(self)
        GtkLayerShell.set_namespace(self, "elefant")
        GtkLayerShell.set_layer(self, GtkLayerShell.Layer.OVERLAY)
        GtkLayerShell.set_exclusive_zone(self, -1)
        GtkLayerShell.set_keyboard_mode(self, GtkLayerShell.KeyboardMode.NONE)
        for edge in (GtkLayerShell.Edge.LEFT, GtkLayerShell.Edge.RIGHT, GtkLayerShell.Edge.BOTTOM):
            GtkLayerShell.set_anchor(self, edge, True)
        GtkLayerShell.set_margin(self, GtkLayerShell.Edge.BOTTOM, MARGIN_BOTTOM)

        self.connect("realize", lambda w: w.get_window().input_shape_combine_region(cairo.Region(), 0, 0))
        area = Gtk.DrawingArea()          # ohne Kind-Widget zeichnet eine Layer-Surface nie
        area.set_size_request(width, int(HEIGHT * self.lh))
        area.connect("draw", self.on_draw)
        self.add(area)

        self.cols = int(width / self.cw)
        self.grid, self.dim = [], []
        self.k = 0                         # Laufbild
        self.x = self.cols                 # linke Kante des Elefanten im Zeichenraster
        self.phase = "in"                  # in (gespiegelt) -> turn -> out
        self.turn = build_turn()
        self.i = 0                         # aktuelles Drehbild
        self.turn_edge = 0
        self.ground = make_ground(self.cols + 1)
        self.stop = self.cols - self.cols // STOP_FRACTION   # Ruesselspitze (linke Kante) bleibt hier stehen
        GLib.timeout_add(int(FRAME_DELAY * 1000), self.tick)

    def tick(self):
        if self.phase == "in":
            self.x -= 1
            self.k += 1
            if self.x <= self.stop:
                self.phase, self.i = "turn", 0
                self.turn_edge = self.x - GRASS_AHEAD
                self.x += TURN_SHIFT_IN
        elif self.phase == "turn":
            self.i += 1
            if self.i == len(self.turn):
                self.phase = "out"
                self.x += TURN_SHIFT_OUT
        else:
            self.x += 1
            self.k += 1
            if self.x >= self.cols:
                Gtk.main_quit()
                return False
        self.build()
        self.queue_draw()
        delay = self.turn[self.i][1] if self.phase == "turn" else FRAME_DELAY
        GLib.timeout_add(int(delay * 1000), self.tick)
        return False

    def build(self):
        if self.phase == "turn":
            lines, dim = self.turn[self.i][0]
        else:
            lines, dim = compose(self.k)
            if self.phase == "in":
                lines, dim = mirror((lines, dim))
        g = [[" "] * self.cols for _ in range(HEIGHT)]
        d = [[False] * self.cols for _ in range(HEIGHT)]
        for r in range(HEIGHT):
            for i, ch in enumerate(lines[r]):
                c = self.x + i
                if ch != " " and 0 <= c < self.cols:
                    g[r][c] = ch; d[r][c] = dim[r][i]
        # Gras: nur auf freien Stellen der Fusszeile, eine halbe Elefantenlaenge voraus
        # (rein, nach links) bzw. nur noch vor dem Elefanten (raus, nach rechts: hinter
        # ihm verschwindet es).
        edge = {"in": self.x - GRASS_AHEAD, "turn": self.turn_edge, "out": self.x}[self.phase]
        row = g[-1]
        pad = [" "] + row + [" "]
        for c in range(self.cols):
            if c >= edge and pad[c:c + 3] == [" "] * 3:
                row[c] = self.ground[c]
        self.grid, self.dim = g, d

    def on_draw(self, _w, ctx):
        ctx.set_operator(cairo.OPERATOR_SOURCE)
        ctx.set_source_rgba(0, 0, 0, 0)
        ctx.paint()
        ctx.set_operator(cairo.OPERATOR_OVER)
        ctx.select_font_face(FONT); ctx.set_font_size(SIZE)
        for r, row in enumerate(self.grid):
            for c, ch in enumerate(row):
                if ch != " ":
                    ctx.set_source_rgba(*COLOR, DIM_ALPHA if self.dim[r][c] else 1.0)
                    ctx.move_to(c * self.cw, r * self.lh + self.ascent)
                    ctx.show_text(ch)

if __name__ == "__main__":
    win = Overlay()
    win.show_all()
    Gtk.main()
