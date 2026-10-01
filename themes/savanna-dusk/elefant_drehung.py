#!/usr/bin/env python3
"""Elefant dreht sich stapfend um (ueber die Rueckansicht).

Leertaste: Start/Pause, Pfeil links/rechts: ein Bild zurueck/vor, q oder Ctrl+C: beenden.

Bild 1 ist die Startstellung, 2-32 die Drehung (Bein heben, Vorderhaelfte dreht + stampft,
Hinterbein heben, Hinterteil zieht nach + stampft), 33 die gespiegelte Zielstellung.
Der Rueckweg (34-64) ist das Spiegelbild von 2-32."""
import os, select, shutil, sys, termios, time, tty

HOLD = 2.0      # Sekunden Pause in Start- und Zielstellung
LIFT = 0.12     # Sekunden: Bein angehoben
STOMP = 0.28    # Sekunden: Drehen + Aufstampfen
COLOR = 33      # ANSI-Farbe (33 = gelb, wie im fastfetch-Logo)
WIDTH, HEIGHT = 44, 11

# Bilder 1-33: (Art: H halten / L Bein heben / S stampfen, Bild, Maske mit "#" = abgedunkelt)
FRAMES = [
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
DELAY = {"H": HOLD, "L": LIFT, "S": STOMP}
MIRROR = str.maketrans("/\\()", "\\/)(")

def lines(text):
    rows = text.split("\n")[1:-1]
    return [l.ljust(WIDTH)[:WIDTH] for l in rows] + [" " * WIDTH] * (HEIGHT - len(rows))

def parse(art, mask):
    return lines(art), [[ch != " " for ch in l] for l in lines(mask)]

def mirror(frame):
    a, d = frame
    return [l[::-1].translate(MIRROR) for l in a], [r[::-1] for r in d]

def build_sequence():
    there = [(parse(a, m), DELAY[k]) for k, a, m in FRAMES]
    back = [(mirror(f), t) for f, t in there[1:-1]]          # Rueckweg = Spiegelbild von 2-32
    return there + back

GROUND = ("   \\|," + " " * 31 + "..\\).").ljust(WIDTH)     # Gras nur ausserhalb der Fusszone

def render(frame, cols, rows, status):
    a, d = frame
    a = a[:-1] + ["".join(ch if ch != " " else GROUND[c] for c, ch in enumerate(a[-1]))]
    a = a + ["", status.center(WIDTH)]                       # Bildnummer unter dem Boden
    d = d + [[False] * WIDTH, [True] * WIDTH]
    left, top = max(0, (cols - WIDTH) // 2), max(0, (rows - len(a)) // 2)
    out = [" " * cols] * top
    for line, dim in zip(a, d):
        s, cur = " " * left, False
        for ch, dd in zip(line, dim):
            if dd != cur:
                s += "\033[2m" if dd else "\033[22m"; cur = dd
            s += ch
        out.append(s + ("\033[22m" if cur else "") + " " * max(0, cols - left - WIDTH))
    out += [" " * cols] * (rows - len(out))
    return "\n".join(out[:rows])

def read_keys(fd):
    """Alle gerade anliegenden Tasten (schnell gedrueckte kommen gebuendelt an)."""
    data, keys = os.read(fd, 64).decode(errors="ignore"), []
    while data:
        if data.startswith(("\033[C", "\033OC")): keys.append("right"); data = data[3:]
        elif data.startswith(("\033[D", "\033OD")): keys.append("left"); data = data[3:]
        else: keys.append(data[0]); data = data[1:]
    return keys

def main():
    seq = build_sequence()
    out, fd = sys.stdout, sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    tty.setcbreak(fd)                                    # Tasten sofort lesen, Ctrl+C bleibt aktiv
    out.write("\033[?1049h\033[?25l\033[%dm" % COLOR)
    i, playing, due = 0, False, 0.0                      # startet pausiert auf Bild 1
    try:
        while True:
            cols, rows = shutil.get_terminal_size()
            status = "Bild %d/%d%s" % (i + 1, len(seq), "" if playing else "  (Pause)")
            out.write("\033[H" + render(seq[i][0], cols, rows, status))
            out.flush()
            wait = max(0.0, due - time.monotonic()) if playing else None
            if select.select([fd], [], [], wait)[0]:
                keys = read_keys(fd)
                if "q" in keys:
                    break
                for key in keys:
                    if key == " ":
                        playing = not playing
                        due = time.monotonic() + seq[i][1]
                    elif key in ("right", "left"):
                        playing = False
                        i = (i + (1 if key == "right" else -1)) % len(seq)
            else:                                         # Bild ist abgelaufen: weiter
                i = (i + 1) % len(seq)
                due = time.monotonic() + seq[i][1]
    except KeyboardInterrupt:
        pass
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
        out.write("\033[0m\033[?25h\033[?1049l")
        out.flush()

if __name__ == "__main__":
    main()
