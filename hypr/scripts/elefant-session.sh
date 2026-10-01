#!/usr/bin/env bash
# Elefant als Gesellschaft beim Lernen/Programmieren.
#   elefant-session.sh          Session an/aus (SUPER+E), bestaetigt mit dem Ruessel von
#                               rechts. Solange sie laeuft, schaut er alle
#                               VISIT_MIN-VISIT_MAX Minuten zufaellig vorbei.
#   elefant-session.sh besuch   Er kommt sofort (SUPER+ALT+E), ausser er ist eh schon da.
# Ein Besuch, egal woher, startet die Wartezeit bis zum naechsten neu.

VISIT_MIN=40   # Minuten
VISIT_MAX=70

RUN=${XDG_RUNTIME_DIR:-/tmp}
PIDFILE=$RUN/elefant-session.pid
LAST=$RUN/elefant-last-visit       # Zeitpunkt des letzten Besuchs (mtime)
LOCK=$RUN/elefant.lock             # immer nur ein Elefant gleichzeitig
OVERLAY=~/.config/hypr/scripts/elefant-overlay.py
SPRUECHE=~/.config/hypr/elefant-sprueche.json   # wird bei jedem Besuch neu gelesen
LAST_SPRUCH=$RUN/elefant-last-spruch

spruch() {
    # Zufaelliger Eintrag aus SPRUECHE, nicht derselbe wie beim letzten Mal. Ein String
    # ist eine Blase, ein Array mehrere nacheinander. Ausgabe: Blasen durch NUL getrennt.
    python3 - "$SPRUECHE" "$LAST_SPRUCH" <<'PY'
import json, random, sys
path, last = sys.argv[1:]
try:
    alle = [s if isinstance(s, list) else [s] for s in json.load(open(path))]
    assert alle and all(b and all(isinstance(t, str) for t in b) for b in alle)
except Exception:
    alle = [["Meine Sprüche-Datei ist kaputt, guck mal rein:", path]]
try:
    vorher = json.load(open(last))
except Exception:
    vorher = None
wahl = random.choice([s for s in alle if s != vorher] or alle)
json.dump(wahl, open(last, "w"))
sys.stdout.write("\0".join(wahl))
PY
}

besuch() {
    touch "$LAST"
    mapfile -t -d '' texte < <(spruch)
    flock -n "$LOCK" python3 "$OVERLAY" "${texte[@]}"
}

session() {
    touch "$LAST"
    wait_s=$(( (VISIT_MIN + RANDOM % (VISIT_MAX - VISIT_MIN + 1)) * 60 ))
    while sleep 60; do
        (( $(date +%s) - $(stat -c %Y "$LAST") < wait_s )) && continue
        pgrep -x hyprlock >/dev/null && continue    # gesperrt: warten, bis du wieder da bist
        besuch
        wait_s=$(( (VISIT_MIN + RANDOM % (VISIT_MAX - VISIT_MIN + 1)) * 60 ))
    done
}

case $1 in
    besuch) besuch ;;
    "")
        if pid=$(cat "$PIDFILE" 2>/dev/null) && kill -0 "$pid" 2>/dev/null; then
            kill -- -"$pid"                         # ganze Gruppe, samt sleep
            rm -f "$PIDFILE"
            python3 "$OVERLAY" --ruessel aus
        else
            setsid "$0" _session >/dev/null 2>&1 &
            echo $! > "$PIDFILE"
            python3 "$OVERLAY" --ruessel an
        fi ;;
    _session) session ;;
esac
