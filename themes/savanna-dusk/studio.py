#!/usr/bin/env python3
"""ASCII-Animationsstudio im Terminal (braucht kitty bzw. dessen Tastaturprotokoll,
sonst gibt es kein Loslassen und damit kein Gedrueckthalten).

    studio.py datei.txt [BREITExHOEHE]

Gibt es die Datei schon, kommen Frames und Groesse aus ihr. Sonst wird die Groesse aus
dem Argument genommen oder abgefragt. Jede Aenderung wird sofort gespeichert.

Tasten:
  Zeichen            schreiben, Cursor rueckt nach rechts
  Pfeile             Cursor bewegen
  Enter              eine Zeile runter, zurueck an die Spalte, wo du angefangen hast
  Backspace / Entf   links vom Cursor / unter dem Cursor loeschen
  Ctrl+Pfeil links/rechts   vorheriges / naechstes Frame (am Ende: neues als Kopie)
  Alt+Pfeil links/rechts    gedrueckt halten: vorheriges / naechstes Frame anzeigen
  Ctrl+P             abspielen in Schleife / anhalten
  Ctrl+Pfeil hoch/runter    schneller / langsamer (Frames pro Sekunde)
  Ctrl+B             Schleife hin und her statt von vorn
  Ctrl+X             Frame loeschen (zweimal druecken)
  Ctrl+Q             beenden
"""
import os, re, select, shutil, sys, termios, tty

FPS = 2        # Vorgabe fuer neue Dateien, danach steht es in der Datei
COLOR = 33     # ANSI-Farbe wie elefant.py
# Zeichen aus den Elefantenbildern (Laufen, Drehung), zum Nachschlagen unter dem Rahmen
CHARS = "& ' ( ) - . / : @ O \\ ^ _ o |"
SEP = "--- "   # Frame-Trenner in der Datei, danach die Nummer

# kitty-Tastaturprotokoll: 1 eindeutig, 2 Druecken/Wiederholen/Loslassen,
# 8 alle Tasten als Escape-Folge, 16 mit getipptem Text
KEYS_ON, KEYS_OFF = "\x1b[>27u", "\x1b[<u"
KEY_RE = re.compile(r"\x1b\[([\d:;]*)([u~ABCDHF])")
ARROWS = {"A": "up", "B": "down", "C": "right", "D": "left"}
SHIFT, ALT, CTRL = 1, 2, 4

def load(path):
    """Datei -> (Breite, Hoehe, Frames als Zeichenraster, fps, hin und her), None wenn
    es sie nicht gibt. Kopfzeile: # studio BREITExHOEHE [fps=N] [hinundher]"""
    if not os.path.exists(path):
        return None
    lines = open(path).read().split("\n")
    head = lines[0].split()
    w, h = map(int, head[2].split("x"))
    fps = next((int(t[4:]) for t in head if t.startswith("fps=")), FPS)
    frames, cur = [], None
    for l in lines[1:]:
        if l.startswith(SEP):
            cur = []; frames.append(cur)
        elif cur is not None and len(cur) < h:
            cur.append(list(l.ljust(w)[:w]))
    for f in frames:
        f += [[" "] * w for _ in range(h - len(f))]
    return w, h, frames or [blank(w, h)], fps, "hinundher" in head

def blank(w, h):
    return [[" "] * w for _ in range(h)]

def ask_size():
    while True:
        try:
            w, h = map(int, input("Groesse (BREITExHOEHE, z. B. 40x11): ").lower().split("x"))
            if w > 0 and h > 0:
                return w, h
        except ValueError:
            pass

class Studio:
    def __init__(self, path, w, h, frames, fps=FPS, pingpong=False):
        self.path, self.w, self.h, self.frames = path, w, h, frames
        self.fps, self.pingpong = fps, pingpong
        self.step = 1               # Abspielrichtung beim Hin und Her
        self.i = 0                  # aktuelles Frame
        self.x = self.y = 0         # Cursor im Raster
        self.home = 0               # Spalte, an die Enter zurueckspringt
        self.peek = None            # (Taste, Frame) solange Alt+Pfeil gedrueckt ist
        self.playing = False
        self.play_i = 0
        self.armed = False          # Ctrl+X einmal gedrueckt
        self.msg = ""

    def advance(self):
        """Abspielen: ein Frame weiter, am Ende von vorn oder umkehren."""
        n = len(self.frames)
        if not self.pingpong:
            self.play_i = (self.play_i + 1) % n
            return
        if not 0 <= self.play_i + self.step < n:
            self.step = -self.step
        self.play_i = max(0, min(n - 1, self.play_i + self.step))

    def save(self):
        with open(self.path, "w") as f:
            f.write(f"# studio {self.w}x{self.h} fps={self.fps}"
                    + " hinundher" * self.pingpong + "\n")
            for n, fr in enumerate(self.frames, 1):
                f.write(f"{SEP}{n}\n" + "".join("".join(r).rstrip() + "\n" for r in fr))

    # --- Zeichnen -------------------------------------------------------------------

    def draw(self):
        shown = self.frames[self.peek[1]] if self.peek else \
                self.frames[self.play_i] if self.playing else self.frames[self.i]
        n = self.play_i if self.playing else self.peek[1] if self.peek else self.i
        state = "  > spielt" if self.playing else "  (Vorschau)" if self.peek else ""
        loop = "hin und her" if self.pingpong else "von vorn"
        out = ["\x1b[H\x1b[2J",
               f" Frame {n + 1}/{len(self.frames)}{state}   {self.fps} fps, {loop}"
               f"   {self.w}x{self.h}   {self.path}\n",
               " +" + "-" * self.w + "+\n"]
        for row in shown:
            out.append(f" |\x1b[{COLOR}m" + "".join(row) + "\x1b[0m|\n")
        out.append(" +" + "-" * self.w + "+\n\n")
        out.append(f" Zeichen:  {CHARS}\n\n")
        out.append(" Ctrl+<- ->  Frame wechseln/neu   Alt+<- -> halten  Vorschau\n")
        out.append(" Ctrl+P  abspielen   Ctrl+hoch runter  fps   Ctrl+B  hin und her\n")
        out.append(" Ctrl+X  Frame loeschen   Ctrl+Q  beenden\n")
        if self.msg:
            out.append(f"\n \x1b[1m{self.msg}\x1b[0m\n")
        if self.playing or self.peek:
            out.append("\x1b[?25l")
        else:                       # echter Cursor an der Schreibstelle
            out.append(f"\x1b[{self.y + 3};{self.x + 3}H\x1b[?25h")
        sys.stdout.write("".join(out).replace("\n", "\r\n"))   # roh: \n allein kehrt nicht zurueck
        sys.stdout.flush()

    # --- Tasten ---------------------------------------------------------------------

    def key(self, name, text, mods, event):
        """name: Pfeil/Sondertaste, text: getipptes Zeichen, event 1/2/3 = runter/halten/hoch."""
        if event == 3:
            if self.peek and name == self.peek[0]:
                self.peek = None
            return True
        if self.peek:               # waehrend der Vorschau wird nicht gemalt
            return True
        if name != "ctrl-x":
            self.armed = False
        self.msg = ""
        if name == "ctrl-q":
            return False
        if name == "ctrl-p":
            self.playing = not self.playing
            self.play_i, self.step = self.i, 1
            return True
        if name in ("up", "down") and mods & CTRL:
            self.fps = max(1, min(30, self.fps + (1 if name == "up" else -1)))
            self.save()
            return True
        if name == "ctrl-b":
            self.pingpong = not self.pingpong
            self.save()
            return True
        if self.playing:
            return True
        f = self.frames[self.i]
        if name in ("left", "right") and mods & ALT:
            j = self.i + (1 if name == "right" else -1)
            if 0 <= j < len(self.frames) and event == 1:
                self.peek = (name, j)
        elif name in ("left", "right") and mods & CTRL:
            if name == "left":
                self.i = max(0, self.i - 1)
            elif self.i == len(self.frames) - 1:
                self.frames.append([r[:] for r in f]); self.i += 1; self.save()
                self.msg = "Neues Frame (Kopie)"
            else:
                self.i += 1
        elif name in ARROWS.values():
            dx, dy = {"left": (-1, 0), "right": (1, 0), "up": (0, -1), "down": (0, 1)}[name]
            self.x = min(self.w - 1, max(0, self.x + dx))
            self.y = min(self.h - 1, max(0, self.y + dy))
            self.home = self.x
        elif name == "enter":
            self.x, self.y = self.home, min(self.h - 1, self.y + 1)
        elif name == "backspace":
            if self.x > 0:
                self.x -= 1; f[self.y][self.x] = " "; self.save()
        elif name == "delete":
            f[self.y][self.x] = " "; self.save()
        elif name == "ctrl-x":
            if len(self.frames) == 1:
                self.msg = "Das letzte Frame bleibt"
            elif not self.armed:
                self.armed = True; self.msg = "Nochmal Ctrl+X loescht dieses Frame"
            else:
                del self.frames[self.i]; self.i = min(self.i, len(self.frames) - 1)
                self.armed = False; self.save(); self.msg = "Frame geloescht"
        elif text and text.isprintable() and not mods & (CTRL | ALT):
            f[self.y][self.x] = text
            self.x = min(self.w - 1, self.x + 1)
            self.save()
        return True

def parse(buf):
    """Escape-Folgen -> [(name, text, mods, event)], dazu der unverbrauchte Rest."""
    out, pos = [], 0
    for m in KEY_RE.finditer(buf):
        pos = m.end()             # Unbekanntes dazwischen wird uebersprungen
        fields = (m.group(1).split(";") + ["", ""])[:3]
        code = int(fields[0].split(":")[0] or 1)
        mods, _, event = fields[1].partition(":")
        mods = (int(mods) - 1 if mods else 0) & (SHIFT | ALT | CTRL)
        event = int(event or 1)
        text = "".join(chr(int(c)) for c in fields[2].split(":") if c)
        end = m.group(2)
        name = None
        if end in ARROWS:
            name = ARROWS[end]
        elif end == "~":
            name = {3: "delete"}.get(code)
        elif end == "u":
            name = {13: "enter", 127: "backspace", 27: "escape"}.get(code)
            if mods & CTRL and not mods & ALT and 97 <= code <= 122:
                name = "ctrl-" + chr(code)
            if code == 32 and not text:
                text = " "
        out.append((name, text, mods, event))
    # angefangene Folge am Ende aufheben
    rest = buf[pos:]
    return out, rest if rest.startswith("\x1b") else ""

def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    path = sys.argv[1]
    loaded = load(path)
    if loaded:
        st = Studio(path, *loaded)
    else:
        w, h = map(int, sys.argv[2].lower().split("x")) if len(sys.argv) > 2 else ask_size()
        st = Studio(path, w, h, [blank(w, h)])
    st.save()

    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    sys.stdout.write("\x1b[?1049h" + KEYS_ON)
    try:
        tty.setraw(fd)
        buf = ""
        st.draw()
        while True:
            r, _, _ = select.select([fd], [], [], 1 / st.fps if st.playing else None)
            if not r:               # Abspielen: naechstes Frame
                st.advance()
                st.draw()
                continue
            buf += os.read(fd, 4096).decode(errors="ignore")
            keys, buf = parse(buf)
            if not all(st.key(*k) for k in keys):
                break
            st.draw()
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
        sys.stdout.write(KEYS_OFF + "\x1b[?25h\x1b[?1049l")
        sys.stdout.flush()

if __name__ == "__main__":
    main()
