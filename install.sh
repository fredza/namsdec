#!/usr/bin/env bash
#
# Installe namsdec pour l'utilisateur courant : le binaire dans le PATH,
# l'icône et le lanceur là où le bureau va les chercher.
#
# Rien n'est écrit hors de $HOME, donc rien ne demande les droits
# d'administrateur. `uninstall.sh` défait exactement ce qui suit.

set -euo pipefail

APP=namsdec
BIN_NAME=namsdec-gui           # nom produit par cargo
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
ICON_DIR="$PREFIX/icons/hicolor"
DESKTOP_DIR="$PREFIX/applications"
SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Marqueur pour retrouver — et n'ajouter qu'une fois — la ligne de PATH.
MARKER="# ajouté par namsdec"

say()  { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m /!\\\033[0m %s\n' "$1"; }
die()  { printf '\033[1;31merreur:\033[0m %s\n' "$1" >&2; exit 1; }

# --- Prérequis ---------------------------------------------------------
command -v cargo >/dev/null || die "cargo est introuvable — installez Rust (https://rustup.rs)."

# nasm et ld ne sont pas nécessaires pour compiler, mais namsdec ne sait
# rien assembler sans eux : autant le dire maintenant plutôt que de le
# laisser découvrir au premier clic sur « Assembler ».
for tool in nasm ld; do
    command -v "$tool" >/dev/null || warn "$tool est introuvable : namsdec ne pourra pas assembler tant qu'il manquera (sudo dnf install nasm binutils)."
done

# --- Compilation -------------------------------------------------------
say "Compilation en mode release (la première fois, comptez quelques minutes)…"
cargo build --release --manifest-path "$SOURCE_DIR/Cargo.toml"

BUILT="$SOURCE_DIR/target/release/$BIN_NAME"
[ -x "$BUILT" ] || die "binaire introuvable après compilation : $BUILT"

# --- Binaire -----------------------------------------------------------
say "Installation du binaire dans $BIN_DIR/$APP"
install -Dm755 "$BUILT" "$BIN_DIR/$APP"

# --- Icônes ------------------------------------------------------------
# Le SVG d'abord : c'est lui que les thèmes modernes préfèrent, à
# n'importe quelle taille. Les PNG restent là pour ce qui ne sait pas
# rendre du vectoriel.
say "Installation des icônes dans $ICON_DIR"
install -Dm644 "$SOURCE_DIR/assets/$APP.svg" "$ICON_DIR/scalable/apps/$APP.svg"
for size in 16 22 24 32 48 64 128 256 512; do
    png="$SOURCE_DIR/assets/icons/$APP-$size.png"
    [ -f "$png" ] && install -Dm644 "$png" "$ICON_DIR/${size}x${size}/apps/$APP.png"
done

# --- Lanceur -----------------------------------------------------------
say "Installation du lanceur dans $DESKTOP_DIR"
mkdir -p "$DESKTOP_DIR"
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

# Les caches : sans eux, le lanceur et l'icône peuvent n'apparaître qu'au
# prochain démarrage de session.
command -v update-desktop-database >/dev/null && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
command -v gtk-update-icon-cache   >/dev/null && gtk-update-icon-cache -qtf "$ICON_DIR" 2>/dev/null || true

# --- PATH --------------------------------------------------------------
# ~/.local/bin est le chemin standard des binaires utilisateur, mais tous
# les shells ne l'ont pas d'office.
if ! printf '%s' ":$PATH:" | grep -q ":$BIN_DIR:"; then
    case "${SHELL##*/}" in
        zsh)  rc="$HOME/.zshrc" ;;
        bash) rc="$HOME/.bashrc" ;;
        *)    rc="$HOME/.profile" ;;
    esac
    if grep -qF "$MARKER" "$rc" 2>/dev/null; then
        say "PATH déjà complété dans $rc (ouvrez un nouveau terminal pour en profiter)."
    else
        say "Ajout de $BIN_DIR au PATH dans $rc"
        printf '\n%s\nexport PATH="%s:$PATH"\n' "$MARKER" "$BIN_DIR" >> "$rc"
        warn "Ouvrez un nouveau terminal, ou lancez : source $rc"
    fi
else
    say "$BIN_DIR est déjà dans le PATH."
fi

# --- Dock (GNOME) ------------------------------------------------------
# Épingler au dash est propre à GNOME ; ailleurs, le lanceur installé
# ci-dessus suffit à faire apparaître namsdec dans le menu.
if command -v gsettings >/dev/null && gsettings writable org.gnome.shell favorite-apps >/dev/null 2>&1; then
    favorites=$(gsettings get org.gnome.shell favorite-apps)
    if printf '%s' "$favorites" | grep -q "'$APP.desktop'"; then
        say "Déjà épinglé au dock."
    else
        say "Épinglage au dock"
        # Insère l'entrée en fin de liste, en gérant le cas d'une liste vide.
        if [ "$favorites" = "@as []" ] || [ "$favorites" = "[]" ]; then
            gsettings set org.gnome.shell favorite-apps "['$APP.desktop']"
        else
            gsettings set org.gnome.shell favorite-apps "${favorites%]}, '$APP.desktop']"
        fi
    fi
fi

say "Terminé. Lancez « $APP » depuis un terminal, ou depuis le dock."
