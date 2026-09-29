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

# --- Couleurs ----------------------------------------------------------
# Éteintes hors d'un terminal et quand NO_COLOR est posé : des codes
# d'échappement dans un journal sont moins lisibles que pas de couleur.
couleurs() {
    if [ -t 1 ] && [ -z "${NO_COLOR:-}" ] && [ "${COULEUR:-auto}" != "non" ]; then
        GRAS=$'\033[1m'; ROUGE=$'\033[1;31m'; VERT=$'\033[1;32m'
        JAUNE=$'\033[1;33m'; BLEU=$'\033[1;34m'; CYAN=$'\033[36m'
        GRIS=$'\033[2m'; FIN=$'\033[0m'
    else
        GRAS=''; ROUGE=''; VERT=''; JAUNE=''; BLEU=''; CYAN=''; GRIS=''; FIN=''
    fi
}

PURGE=0
ESSAI=0
SILENCIEUX=0
COULEUR=auto

aide() {
    couleurs
    cat <<AIDE
${GRAS}namsdec — désinstallation${FIN}

  ${GRAS}Usage${FIN}
    ./uninstall.sh [options]

  Défait exactement ce qu'a fait ${GRIS}install.sh${FIN} : le binaire, les icônes,
  le lanceur, l'entrée du dock et la ligne de PATH.

  Vos ${GRAS}réglages${FIN} et vos ${GRAS}séries d'exercices${FIN} sont gardés par défaut — on
  désinstalle souvent pour réinstaller, et perdre ses raccourcis au
  passage serait une mauvaise surprise.

  ${GRAS}Options${FIN}
    ${CYAN}--purge${FIN}           retirer AUSSI les réglages (${GRIS}$CONFIG_DIR${FIN})
                      et les données (${GRIS}$DATA_DIR${FIN})
    ${CYAN}-n, --dry-run${FIN}     dire ce qui serait retiré, sans rien supprimer
    ${CYAN}-q, --quiet${FIN}       ne dire que l'essentiel
    ${CYAN}--no-color${FIN}        pas de couleurs (respecte aussi NO_COLOR)
    ${CYAN}-h, --help${FIN}        cette aide

  ${GRAS}Exemples${FIN}
    ${GRIS}./uninstall.sh${FIN}              garde vos réglages et vos séries
    ${GRIS}./uninstall.sh --dry-run${FIN}    voir ce qui partirait
    ${GRIS}./uninstall.sh --purge${FIN}      tout retirer, réglages compris
AIDE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --purge)      PURGE=1; shift ;;
        -n|--dry-run) ESSAI=1; shift ;;
        -q|--quiet)   SILENCIEUX=1; shift ;;
        --no-color)   COULEUR=non; shift ;;
        -h|--help)    aide; exit 0 ;;
        *)
            couleurs
            # Une option mal tapée était avalée en silence : `--purgé`
            # ne purgeait rien, et le script disait « Terminé ».
            printf '%serreur:%s option inconnue : %s\n' "$ROUGE" "$FIN" "$1" >&2
            printf 'Essayez %s./uninstall.sh --help%s.\n' "$GRAS" "$FIN" >&2
            exit 2
            ;;
    esac
done

couleurs
say()  { [ "$SILENCIEUX" = 1 ] || printf '%s==>%s %s\n' "$BLEU" "$FIN" "$1"; }
retirer() {
    if [ "$ESSAI" = 1 ]; then
        printf '    %s· %s%s\n' "$GRIS" "$*" "$FIN"
        return 0
    fi
    "$@"
}

# --- Dock (GNOME) ------------------------------------------------------
# À retirer avant le reste : une entrée épinglée qui pointe vers un
# lanceur disparu laisse une icône morte dans le dash.
if command -v gsettings >/dev/null && gsettings writable org.gnome.shell favorite-apps >/dev/null 2>&1; then
    favorites=$(gsettings get org.gnome.shell favorite-apps)
    if printf '%s' "$favorites" | grep -q "'$APP.desktop'"; then
        say "Retrait du dock"
        cleaned=$(printf '%s' "$favorites" \
            | sed -E "s/'$APP\.desktop', ?//; s/, ?'$APP\.desktop'//; s/'$APP\.desktop'//")
        retirer gsettings set org.gnome.shell favorite-apps "$cleaned"
    fi
fi

# --- Lanceur, icônes, binaire ------------------------------------------
say "Suppression du lanceur"
retirer rm -f "$DESKTOP_DIR/$APP.desktop"

say "Suppression des icônes"
retirer rm -f "$ICON_DIR/scalable/apps/$APP.svg"
for size in 16 22 24 32 48 64 128 256 512; do
    retirer rm -f "$ICON_DIR/${size}x${size}/apps/$APP.png"
done

say "Suppression du binaire"
retirer rm -f "$BIN_DIR/$APP"
# Les binaires des préversions, quand on en a essayé une à côté de la
# version installée. Il n'y en a plus qu'un aujourd'hui, mais une
# machine qui a connu les deux garderait l'autre sans que personne
# ne s'en souvienne.
for reste in "$BIN_DIR/$APP"-beta*; do
    [ -e "$reste" ] || continue
    say "Suppression de $(basename "$reste")"
    retirer rm -f "$reste"
done

if [ "$ESSAI" != 1 ]; then
    command -v update-desktop-database >/dev/null && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    command -v gtk-update-icon-cache   >/dev/null && gtk-update-icon-cache -qtf "$ICON_DIR" 2>/dev/null || true
fi

# --- PATH --------------------------------------------------------------
# On ne retire que la ligne posée par l'installation, repérée par son
# marqueur : le fichier peut contenir d'autres ajouts au PATH, qui ne
# nous appartiennent pas.
for rc in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.profile"; do
    if [ -f "$rc" ] && grep -qF "$MARKER" "$rc"; then
        say "Retrait de la ligne de PATH dans $rc"
        if [ "$ESSAI" = 1 ]; then
            printf '    %s· la ligne « %s » et son export%s\n' "$GRIS" "$MARKER" "$FIN"
            continue
        fi
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
    retirer rm -rf "$CONFIG_DIR"
    say "Suppression des données ($DATA_DIR)"
    retirer rm -rf "$DATA_DIR"
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

if [ "$ESSAI" = 1 ]; then
    [ "$SILENCIEUX" = 1 ] || printf '\n  %s— essai à blanc terminé : rien n'"'"'a été supprimé.%s\n\n' "$JAUNE" "$FIN"
else
    [ "$SILENCIEUX" = 1 ] || printf '\n  %s✓ namsdec est désinstallé.%s\n\n' "$VERT" "$FIN"
fi
