#!/usr/bin/env bash
#
# Désinstalle namsdec : défait exactement ce qu'a fait `install.sh`.
#
# Les réglages (~/.config/namsdec) et les données (~/.local/share/namsdec :
# thèmes et séries d'exercices installés) sont conservés par défaut — on
# désinstalle souvent pour réinstaller, et perdre ses raccourcis ou les
# séries qu'on a écrites au passage serait une mauvaise surprise.
# `--purge` supprime les deux.

set -euo pipefail

APP=namsdec
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
ICON_DIR="$PREFIX/icons/hicolor"
DESKTOP_DIR="$PREFIX/applications"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/$APP"
# Ce que l'utilisateur a ajouté : thèmes et séries d'exercices installés.
DATA_DIR="$PREFIX/$APP"
MARKER="# ajouté par namsdec"

PURGE=0
[ "${1:-}" = "--purge" ] && PURGE=1

say() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }

# --- Dock (GNOME) ------------------------------------------------------
# À retirer avant le reste : une entrée épinglée qui pointe vers un
# lanceur disparu laisse une icône morte dans le dash.
if command -v gsettings >/dev/null && gsettings writable org.gnome.shell favorite-apps >/dev/null 2>&1; then
    favorites=$(gsettings get org.gnome.shell favorite-apps)
    if printf '%s' "$favorites" | grep -q "'$APP.desktop'"; then
        say "Retrait du dock"
        cleaned=$(printf '%s' "$favorites" \
            | sed -E "s/'$APP\.desktop', ?//; s/, ?'$APP\.desktop'//; s/'$APP\.desktop'//")
        gsettings set org.gnome.shell favorite-apps "$cleaned"
    fi
fi

# --- Lanceur, icônes, binaire ------------------------------------------
say "Suppression du lanceur"
rm -f "$DESKTOP_DIR/$APP.desktop"

say "Suppression des icônes"
rm -f "$ICON_DIR/scalable/apps/$APP.svg"
for size in 16 22 24 32 48 64 128 256 512; do
    rm -f "$ICON_DIR/${size}x${size}/apps/$APP.png"
done

say "Suppression du binaire"
rm -f "$BIN_DIR/$APP"

command -v update-desktop-database >/dev/null && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
command -v gtk-update-icon-cache   >/dev/null && gtk-update-icon-cache -qtf "$ICON_DIR" 2>/dev/null || true

# --- PATH --------------------------------------------------------------
# On ne retire que la ligne posée par l'installation, repérée par son
# marqueur : le fichier peut contenir d'autres ajouts au PATH, qui ne
# nous appartiennent pas.
for rc in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
    if [ -f "$rc" ] && grep -qF "$MARKER" "$rc"; then
        say "Retrait de la ligne de PATH dans $rc"
        # La ligne du marqueur, l'export qui la suit, et la ligne vide
        # qui la précédait.
        tmp=$(mktemp)
        awk -v m="$MARKER" '
            $0 == m { drop = 2; if (blank) blank = 0; next }
            drop > 0 { drop--; next }
            /^$/ { blank = 1; line = $0; next }
            { if (blank) { print line; blank = 0 } print }
            END { if (blank) print line }
        ' "$rc" > "$tmp"
        cat "$tmp" > "$rc"
        rm -f "$tmp"
    fi
done

# --- Réglages et données ------------------------------------------------
if [ "$PURGE" = 1 ]; then
    say "Suppression des réglages ($CONFIG_DIR)"
    rm -rf "$CONFIG_DIR"
    say "Suppression des données ($DATA_DIR)"
    rm -rf "$DATA_DIR"
else
    # `if` et non `[ … ] && …` : sous `set -e`, un test faux en fin de
    # script suffirait à le faire sortir en erreur avant son dernier mot.
    if [ -d "$CONFIG_DIR" ]; then
        say "Réglages conservés dans $CONFIG_DIR (--purge pour tout supprimer)."
    fi
    if [ -d "$DATA_DIR" ]; then
        say "Thèmes et séries d'exercices conservés dans $DATA_DIR."
    fi
fi

say "Terminé."
