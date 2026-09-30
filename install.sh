#!/usr/bin/env bash
#
# Installe namsdec pour l'utilisateur courant : le binaire dans le PATH,
# l'icône et le lanceur là où le bureau va les chercher.
#
# Rien n'est écrit hors de $HOME, donc rien ne demande les droits
# d'administrateur. `uninstall.sh` défait exactement ce qui suit.
#
# Les noms de variables et de fonctions sont SANS ACCENT : bash n'accepte
# que des lettres ASCII, des chiffres et des tirets bas dans ce qu'il
# doit lire comme un identifiant.

set -euo pipefail

APP=namsdec
BIN_NAME=namsdec-gui           # nom produit par cargo
SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Marqueur pour retrouver — et n'ajouter qu'une fois — la ligne de PATH.
MARKER="# ajouté par namsdec"

# --- Couleurs ----------------------------------------------------------
# Éteintes hors d'un terminal, et quand NO_COLOR est posé : des codes
# d'échappement dans un journal d'intégration continue ou dans un `less`
# sans -R sont moins lisibles que pas de couleur du tout.
couleurs() {
    if [ -t 1 ] && [ -z "${NO_COLOR:-}" ] && [ "${COULEUR:-auto}" != "non" ]; then
        GRAS=$'\033[1m';  ROUGE=$'\033[1;31m'; VERT=$'\033[1;32m'
        JAUNE=$'\033[1;33m'; BLEU=$'\033[1;34m'; CYAN=$'\033[36m'
        GRIS=$'\033[2m';  FIN=$'\033[0m'
    else
        GRAS=''; ROUGE=''; VERT=''; JAUNE=''; BLEU=''; CYAN=''; GRIS=''; FIN=''
    fi
}

# --- Options -----------------------------------------------------------
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
AVEC_LANCEUR=1
AVEC_DOCK=1
AVEC_PATH=1
AVEC_EXERCICES=1
AVEC_STRIP=1
AVEC_COMPILATION=1
PROFIL=release
ESSAI=0
SILENCIEUX=0
COULEUR=auto

aide() {
    couleurs
    cat <<AIDE
${GRAS}namsdec — installation${FIN}

  ${GRAS}Usage${FIN}
    ./install.sh [options]

  Compile namsdec, pose le binaire dans le PATH, l'icône et le lanceur
  là où le bureau va les chercher. Rien n'est écrit hors de \$HOME : les
  droits d'administrateur ne sont jamais demandés.

  ${GRAS}Options${FIN}
    ${CYAN}--prefix DIR${FIN}      où vont icônes, lanceur et exercices
                      (défaut : ${GRIS}$PREFIX${FIN})
    ${CYAN}--bin-dir DIR${FIN}     où va le binaire
                      (défaut : ${GRIS}$BIN_DIR${FIN})
    ${CYAN}--debug${FIN}           installer la version de débogage : non optimisée,
                      mais prête en quelques secondes
    ${CYAN}--no-desktop${FIN}      ne pas poser le lanceur ni les icônes
    ${CYAN}--no-dock${FIN}         ne pas épingler au dock (GNOME)
    ${CYAN}--no-path${FIN}         ne pas toucher au fichier de configuration du shell
    ${CYAN}--no-exercises${FIN}    ne pas installer la série d'exercices livrée
    ${CYAN}--no-strip${FIN}        garder les symboles de débogage dans le binaire
                      installé (deux mégaoctets de plus)
    ${CYAN}--no-build${FIN}        ne pas compiler : poser le binaire déjà construit.
                      Pour qui compile lui-même — un empaqueteur, ou une
                      réinstallation après un simple ${GRIS}cargo build${FIN}
    ${CYAN}-n, --dry-run${FIN}     dire ce qui serait fait, sans rien écrire
    ${CYAN}-q, --quiet${FIN}       ne dire que l'essentiel
    ${CYAN}--no-color${FIN}        pas de couleurs (respecte aussi NO_COLOR)
    ${CYAN}-h, --help${FIN}        cette aide

  ${GRAS}Exemples${FIN}
    ${GRIS}./install.sh${FIN}
    ${GRIS}./install.sh --debug --no-dock${FIN}       essayer vite, sans toucher au dock
    ${GRIS}./install.sh --dry-run${FIN}               voir ce qui serait écrit
    ${GRIS}./install.sh --prefix /opt/namsdec${FIN}   ailleurs que dans ~/.local/share

  ${GRAS}Pour désinstaller${FIN}
    ${GRIS}./uninstall.sh${FIN}            garde vos réglages et vos séries
    ${GRIS}./uninstall.sh --purge${FIN}    retire tout
AIDE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --prefix)        PREFIX="${2:?--prefix demande un dossier}"; shift 2 ;;
        --prefix=*)      PREFIX="${1#*=}"; shift ;;
        --bin-dir)       BIN_DIR="${2:?--bin-dir demande un dossier}"; shift 2 ;;
        --bin-dir=*)     BIN_DIR="${1#*=}"; shift ;;
        --debug)         PROFIL=debug; shift ;;
        --no-desktop)    AVEC_LANCEUR=0; shift ;;
        --no-dock)       AVEC_DOCK=0; shift ;;
        --no-path)       AVEC_PATH=0; shift ;;
        --no-exercises)  AVEC_EXERCICES=0; shift ;;
        --no-strip)      AVEC_STRIP=0; shift ;;
        --no-build)      AVEC_COMPILATION=0; shift ;;
        -n|--dry-run)    ESSAI=1; shift ;;
        -q|--quiet)      SILENCIEUX=1; shift ;;
        --no-color)      COULEUR=non; shift ;;
        -h|--help)       aide; exit 0 ;;
        *)
            couleurs
            printf '%serreur:%s option inconnue : %s\n' "$ROUGE" "$FIN" "$1" >&2
            printf 'Essayez %s./install.sh --help%s.\n' "$GRAS" "$FIN" >&2
            exit 2
            ;;
    esac
done

couleurs
ICON_DIR="$PREFIX/icons/hicolor"
EXERCISES_DIR="$PREFIX/$APP/exercises"
DESKTOP_DIR="$PREFIX/applications"

# --- Dire ce qu'on fait -------------------------------------------------
ETAPE=0
etape() {
    ETAPE=$((ETAPE + 1))
    [ "$SILENCIEUX" = 1 ] && return 0
    printf '%s[%d/%d]%s %s\n' "$BLEU" "$ETAPE" "$TOTAL" "$FIN" "$1"
}
detail()  { [ "$SILENCIEUX" = 1 ] || printf '      %s%s%s\n' "$GRIS" "$1" "$FIN"; }
ok()      { [ "$SILENCIEUX" = 1 ] || printf '      %s✓%s %s\n' "$VERT" "$FIN" "$1"; }
avertir() { printf '%s  ⚠%s %s\n' "$JAUNE" "$FIN" "$1" >&2; }
mourir()  { printf '%serreur:%s %s\n' "$ROUGE" "$FIN" "$1" >&2; exit 1; }

# `--dry-run` : tout ce qui écrit passe par ici, donc rien ne peut lui
# échapper. Une seule commande oubliée rendrait l'essai à blanc pire
# qu'absent — on l'emploie précisément quand on n'ose pas la vraie.
faire() {
    if [ "$ESSAI" = 1 ]; then
        printf '      %s· %s%s\n' "$GRIS" "$*" "$FIN"
        return 0
    fi
    "$@"
}

# Combien d'étapes : le compte est annoncé, donc il doit être juste.
TOTAL=2
[ "$AVEC_COMPILATION" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_EXERCICES" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_LANCEUR" = 1 ] && TOTAL=$((TOTAL + 2))
[ "$AVEC_PATH" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_DOCK" = 1 ] && TOTAL=$((TOTAL + 1))

if [ "$SILENCIEUX" != 1 ]; then
    VERSION=$(awk -F'"' '/^version = /{print $2; exit}' "$SOURCE_DIR/Cargo.toml" 2>/dev/null || echo '?')
    printf '\n%s  namsdec%s %s%s%s\n' "$GRAS" "$FIN" "$GRIS" "$VERSION" "$FIN"
    printf '  %sun atelier pour apprendre l'"'"'assembleur x86-64%s\n\n' "$GRIS" "$FIN"
    [ "$ESSAI" = 1 ] && printf '  %s— essai à blanc : rien ne sera écrit —%s\n\n' "$JAUNE" "$FIN"
fi

# --- Prérequis ---------------------------------------------------------
etape "Vérification des prérequis"
command -v cargo >/dev/null || mourir "cargo est introuvable — installez Rust (https://rustup.rs)."
ok "cargo $(cargo --version 2>/dev/null | awk '{print $2}')"

# nasm et ld ne sont pas nécessaires pour compiler, mais namsdec ne sait
# rien assembler sans eux : autant le dire maintenant plutôt que de le
# laisser découvrir au premier clic sur « Assembler ».
for outil in nasm ld; do
    if command -v "$outil" >/dev/null; then
        ok "$outil"
    else
        avertir "$outil est introuvable : namsdec ne pourra pas assembler tant qu'il manquera (sudo dnf install nasm binutils)."
    fi
done

# --- Compilation -------------------------------------------------------
if [ "$PROFIL" = release ]; then
    BUILT="$SOURCE_DIR/target/release/$BIN_NAME"
else
    BUILT="$SOURCE_DIR/target/debug/$BIN_NAME"
fi
if [ "$AVEC_COMPILATION" = 1 ]; then
    if [ "$PROFIL" = release ]; then
        etape "Compilation (la première fois, comptez quelques minutes)"
        faire cargo build --release --manifest-path "$SOURCE_DIR/Cargo.toml"
    else
        etape "Compilation de la version de débogage"
        detail "non optimisée : namsdec y sera plus lent, mais prêt en quelques secondes"
        faire cargo build --manifest-path "$SOURCE_DIR/Cargo.toml"
    fi
fi
if [ "$ESSAI" != 1 ]; then
    # Sans compilation, le binaire doit déjà être là — et le dire vaut
    # mieux que d'installer l'avant-dernier sans prévenir.
    [ -x "$BUILT" ] || mourir "binaire introuvable : $BUILT
    (compilez d'abord, ou retirez --no-build)"
    ok "$(du -h "$BUILT" | cut -f1) — $BUILT"
fi

# --- Binaire -----------------------------------------------------------
etape "Installation du binaire"
detail "$BIN_DIR/$APP"
faire install -Dm755 "$BUILT" "$BIN_DIR/$APP"
# Les symboles pèsent deux mégaoctets et ne servent qu'à lire une trace
# de panique. Celui de `target/` les garde — c'est lui qu'on ouvre dans
# un débogueur ; celui qu'on installe s'en passe. Le BuildID reste, de
# quoi apparier les deux si besoin.
if [ "$AVEC_STRIP" = 1 ] && [ "$PROFIL" = release ] && command -v strip >/dev/null; then
    faire strip "$BIN_DIR/$APP" 2>/dev/null || true
fi
[ "$ESSAI" = 1 ] || ok "$(du -h "$BIN_DIR/$APP" | cut -f1) installé"

# --- Exercices ---------------------------------------------------------
# Le parcours n'est pas dans le binaire : c'est un fichier de données,
# comme un thème. On l'installe donc, et une correction d'énoncé ne
# demande plus de recompiler. Ce que l'utilisateur a ajouté à côté n'est
# pas touché : seul le fichier livré est remplacé.
if [ "$AVEC_EXERCICES" = 1 ]; then
    etape "Installation du parcours d'exercices"
    detail "$EXERCISES_DIR/$APP.json"
    faire install -Dm644 "$SOURCE_DIR/crates/namsdec-exercises/packs/namsdec.json" \
        "$EXERCISES_DIR/$APP.json"
fi

# --- Icônes ------------------------------------------------------------
# Le SVG d'abord : c'est lui que les thèmes modernes préfèrent, à
# n'importe quelle taille. Les PNG restent là pour ce qui ne sait pas
# rendre du vectoriel.
if [ "$AVEC_LANCEUR" = 1 ]; then
    etape "Installation des icônes"
    detail "$ICON_DIR"
    faire install -Dm644 "$SOURCE_DIR/assets/$APP.svg" "$ICON_DIR/scalable/apps/$APP.svg"
    posees=0
    for size in 16 22 24 32 48 64 128 256 512; do
        png="$SOURCE_DIR/assets/icons/$APP-$size.png"
        if [ -f "$png" ]; then
            faire install -Dm644 "$png" "$ICON_DIR/${size}x${size}/apps/$APP.png"
            posees=$((posees + 1))
        fi
    done
    ok "1 vectorielle, $posees matricielles"

    # --- Lanceur -------------------------------------------------------
    etape "Installation du lanceur"
    detail "$DESKTOP_DIR/$APP.desktop"
    faire mkdir -p "$DESKTOP_DIR"
    if [ "$ESSAI" != 1 ]; then
        cat > "$DESKTOP_DIR/$APP.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Version=1.0
Name=Namsdec
GenericName=Assembly workbench
GenericName[fr]=Atelier d'assembleur
Comment=Learn NASM assembly and reverse engineering
Comment[fr]=Apprendre l'assembleur NASM et le reverse engineering
Exec=$BIN_DIR/$APP %f
Icon=$APP
Terminal=false
Categories=Development;IDE;
Keywords=assembly;nasm;asm;x86;debugger;disassembler;
Keywords[fr]=assembleur;nasm;asm;x86;débogueur;désassembleur;
MimeType=text/x-asm;
StartupNotify=true
StartupWMClass=$APP
DESKTOP
    fi

    # Les caches : sans eux, le lanceur et l'icône peuvent n'apparaître
    # qu'au prochain démarrage de session.
    if [ "$ESSAI" != 1 ]; then
        command -v update-desktop-database >/dev/null && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
        command -v gtk-update-icon-cache   >/dev/null && gtk-update-icon-cache -qtf "$ICON_DIR" 2>/dev/null || true
    fi
fi

# --- PATH --------------------------------------------------------------
# ~/.local/bin est le chemin standard des binaires utilisateur, mais tous
# les shells ne l'ont pas d'office.
RAPPEL_PATH=""
if [ "$AVEC_PATH" = 1 ]; then
    etape "Vérification du PATH"
    if ! printf '%s' ":$PATH:" | grep -q ":$BIN_DIR:"; then
        case "${SHELL##*/}" in
            zsh)  rc="$HOME/.zshrc" ;;
            bash) rc="$HOME/.bashrc" ;;
            *)    rc="$HOME/.profile" ;;
        esac
        if grep -qF "$MARKER" "$rc" 2>/dev/null; then
            ok "déjà complété dans $rc"
        else
            detail "ajout de $BIN_DIR au PATH dans $rc"
            if [ "$ESSAI" != 1 ]; then
                printf '\n%s\nexport PATH="%s:$PATH"\n' "$MARKER" "$BIN_DIR" >> "$rc"
            fi
        fi
        RAPPEL_PATH="Ouvrez un nouveau terminal, ou lancez : source $rc"
    else
        ok "$BIN_DIR y est déjà"
    fi
fi

# --- Dock (GNOME) ------------------------------------------------------
# Épingler au dash est propre à GNOME ; ailleurs, le lanceur installé
# ci-dessus suffit à faire apparaître namsdec dans le menu.
if [ "$AVEC_DOCK" = 1 ]; then
    etape "Épinglage au dock"
    if command -v gsettings >/dev/null && gsettings writable org.gnome.shell favorite-apps >/dev/null 2>&1; then
        favorites=$(gsettings get org.gnome.shell favorite-apps)
        if printf '%s' "$favorites" | grep -q "'$APP.desktop'"; then
            ok "déjà épinglé"
        elif [ "$ESSAI" = 1 ]; then
            detail "serait épinglé au dash de GNOME"
        else
            # Insère l'entrée en fin de liste, en gérant la liste vide.
            if [ "$favorites" = "@as []" ] || [ "$favorites" = "[]" ]; then
                gsettings set org.gnome.shell favorite-apps "['$APP.desktop']"
            else
                gsettings set org.gnome.shell favorite-apps "${favorites%]}, '$APP.desktop']"
            fi
            ok "épinglé"
        fi
    else
        detail "pas de dash GNOME ici — le lanceur suffit"
    fi
fi

# --- Le mot de la fin ---------------------------------------------------
if [ "$SILENCIEUX" = 1 ]; then
    exit 0
fi
# Un essai à blanc qui annoncerait « installé » serait le seul mensonge
# de ce script, et le pire : on le croirait.
if [ "$ESSAI" = 1 ]; then
    printf '\n  %s— essai à blanc terminé : rien n'"'"'a été écrit.%s\n' "$JAUNE" "$FIN"
    printf '  %sRelancez sans --dry-run pour installer.%s\n\n' "$GRIS" "$FIN"
    exit 0
fi
printf '\n  %s✓ namsdec est installé.%s\n\n' "$VERT" "$FIN"
printf '  %sLancer%s          %s\n' "$GRAS" "$FIN" "$APP"
[ "$AVEC_LANCEUR" = 1 ] && printf '  %sou%s              depuis le menu des applications\n' "$GRAS" "$FIN"
printf '  %sPremiers pas%s    Aide ▸ Guide, ou la touche %sF1%s\n' "$GRAS" "$FIN" "$CYAN" "$FIN"
printf '  %sDésinstaller%s    ./uninstall.sh\n' "$GRAS" "$FIN"
if [ -n "$RAPPEL_PATH" ]; then
    printf '\n  %s⚠ %s%s\n' "$JAUNE" "$RAPPEL_PATH" "$FIN"
fi
printf '\n'
