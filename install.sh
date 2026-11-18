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
# La commande d'analyse. Elle garde son nom tel quel : `namsdec` ouvre
# une fenêtre, `namsdec-cli` écrit un rapport, et deux noms pour deux
# gestes valent mieux qu'un sous-commande à retenir.
CLI_NAME=namsdec-cli
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
AVEC_CLI=1
AVEC_STRIP=1
AVEC_COMPILATION=1
AVEC_REDEMARRAGE=1
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
    ${CYAN}--no-exercises${FIN}    ne pas installer les séries d'exercices
    ${CYAN}--no-cli${FIN}          ne pas installer ${GRIS}namsdec-cli${FIN}, la commande qui écrit
                      l'analyse d'un binaire sans ouvrir de fenêtre
    ${CYAN}--no-strip${FIN}        garder les symboles de débogage dans le binaire
                      installé (deux mégaoctets de plus)
    ${CYAN}--no-restart${FIN}      ne pas redémarrer namsdec s'il tourne : la mise à jour
                      ne sera prise qu'à sa prochaine ouverture
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
    ${GRIS}./install.sh --no-restart${FIN}            mettre à jour sans fermer la fenêtre ouverte
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
        --no-cli)        AVEC_CLI=0; shift ;;
        --no-strip)      AVEC_STRIP=0; shift ;;
        --no-restart)    AVEC_REDEMARRAGE=0; shift ;;
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

# Un cadre autour d'un titre. Les caractères de dessin sont ceux que tout
# terminal moderne rend ; ailleurs, la ligne reste lisible même si le
# trait tombe sur un point d'interrogation.
# Compléter un texte à une largeur donnée, en colonnes.
#
# `printf '%-30s'` ne convient pas : il compte les OCTETS, si bien que
# « alléger » en fait huit et décale la colonne d'un cran. `${#texte}`,
# lui, compte les caractères — c'est ce qu'il faut ici.
caler() {
    local texte="$1"
    local largeur="$2"
    local reste=$((largeur - ${#texte}))
    printf '%s' "$texte"
    while [ "$reste" -gt 0 ]; do printf ' '; reste=$((reste - 1)); done
}

cadre() {
    local texte="$1" largeur=${2:-62} trait=''
    local i
    for ((i = 0; i < largeur; i++)); do trait+='─'; done
    printf '  %s┌%s┐%s\n' "$BLEU" "$trait" "$FIN"
    printf '  %s│%s %s' "$BLEU" "$FIN" "$GRAS"
    caler "$texte" "$((largeur - 2))"
    printf '%s %s│%s\n' "$FIN" "$BLEU" "$FIN"
    printf '  %s└%s┘%s\n' "$BLEU" "$trait" "$FIN"
}

# Une ligne de réglage : ✓ quand c'est actif, · quand ce ne l'est pas, et
# l'option qui l'inverse écrite à côté. C'est là toute l'idée — on voit
# d'un coup d'œil ce qui va se passer ET comment le changer, sans avoir à
# relire --help.
reglage() {
    local actif="$1" nom="$2" option="$3"
    [ "$SILENCIEUX" = 1 ] && return 0
    if [ "$actif" = 1 ]; then
        printf '    %s✓%s ' "$VERT" "$FIN"
        caler "$nom" 28
        printf ' %s%s%s\n' "$GRIS" "$option" "$FIN"
    else
        # Inactif : l'option qui l'annule n'a plus d'objet, et
        # l'afficher laisserait croire qu'il reste quelque chose à
        # couper.
        printf '    %s·%s %s%s%s\n' "$GRIS" "$FIN" "$GRIS" "$nom" "$FIN"
    fi
}

ETAPE=0
etape() {
    ETAPE=$((ETAPE + 1))
    [ "$SILENCIEUX" = 1 ] && return 0
    # Une barre plutôt qu'un rapport de deux nombres : sur une
    # compilation de plusieurs minutes, « où en est-on » se lit d'un
    # coup d'œil et non en faisant une division.
    local barre='' i
    for ((i = 1; i <= TOTAL; i++)); do
        if [ "$i" -le "$ETAPE" ]; then barre+='▰'; else barre+='▱'; fi
    done
    printf '\n  %s%s%s  %s%d/%d%s  %s%s%s\n' \
        "$BLEU" "$barre" "$FIN" "$GRIS" "$ETAPE" "$TOTAL" "$FIN" "$GRAS" "$1" "$FIN"
}
detail()  { [ "$SILENCIEUX" = 1 ] || printf '      %s%s%s\n' "$GRIS" "$1" "$FIN"; }
ok()      { [ "$SILENCIEUX" = 1 ] || printf '      %s✓%s %s\n' "$VERT" "$FIN" "$1"; }
avertir() { printf '%s  ⚠%s %s\n' "$JAUNE" "$FIN" "$1" >&2; }
mourir()  { printf '%serreur:%s %s\n' "$ROUGE" "$FIN" "$1" >&2; exit 1; }

# --- Poser un fichier sans qu'il manque une seule seconde -------------
#
# `install` SUPPRIME la cible avant d'écrire la nouvelle. Pour un
# exécutable en cours, c'est sans danger — le processus garde son inode
# —, mais il reste une fenêtre, courte et bien réelle, pendant laquelle
# LE FICHIER N'EXISTE PAS. Et quelqu'un regarde : GNOME Shell élague de
# lui-même les favoris dont il ne trouve pas le `.desktop`. On relançait
# donc l'installation, et l'icône disparaissait du dock — on concluait
# que l'installation avait désinstallé le programme.
#
# `cat > fichier` est pire encore : il tronque, et laisse le fichier
# VIDE le temps de le réécrire. C'est ce que faisait le lanceur.
#
# On écrit donc à côté, puis on renomme. `mv` dans le même dossier est
# un `rename(2)` : atomique, indivisible. À aucun instant la cible n'est
# absente ni incomplète — elle est l'ancienne, puis la nouvelle.
poser() {
    local mode="$1" source="$2" cible="$3"
    if [ "$ESSAI" = 1 ]; then
        printf '      %s· poser %s -> %s%s\n' "$GRIS" "$source" "$cible" "$FIN"
        return 0
    fi
    mkdir -p "$(dirname "$cible")"
    local temporaire="$(dirname "$cible")/.$(basename "$cible").neuf.$$"
    # Le temporaire part avec nous si quelque chose tourne mal : un
    # dossier semé de `.namsdec.neuf.1234` serait une trace de plus à
    # nettoyer à la main.
    trap 'rm -f "$temporaire"' RETURN
    cp -f "$source" "$temporaire"
    chmod "$mode" "$temporaire"
    mv -f "$temporaire" "$cible"
}

# Le même geste pour un fichier qu'on écrit au fil de l'eau.
poser_depuis_stdin() {
    local mode="$1" cible="$2"
    if [ "$ESSAI" = 1 ]; then
        cat >/dev/null
        printf '      %s· écrire %s%s\n' "$GRIS" "$cible" "$FIN"
        return 0
    fi
    mkdir -p "$(dirname "$cible")"
    local temporaire="$(dirname "$cible")/.$(basename "$cible").neuf.$$"
    trap 'rm -f "$temporaire"' RETURN
    cat > "$temporaire"
    chmod "$mode" "$temporaire"
    mv -f "$temporaire" "$cible"
}

# --- Qui tourne en ce moment ------------------------------------------
#
# Par l'EXÉCUTABLE et non par le nom : `pgrep namsdec` attrape le
# terminal dont le titre porte ce mot, et l'on tuerait sa propre
# session. `/proc/<pid>/exe` est un lien vers le fichier réellement
# exécuté — c'est exact, et cela ne peut désigner que lui.
instances_en_cours() {
    local cible="$1" pid exe
    [ -e "$cible" ] || return 0
    for pid in /proc/[0-9]*; do
        pid="${pid#/proc/}"
        exe=$(readlink "/proc/$pid/exe" 2>/dev/null) || continue
        # Un binaire remplacé voit son lien suffixé « (deleted) » :
        # c'est précisément le cas d'une instance lancée AVANT la mise à
        # jour, et c'est elle qu'il faut redémarrer.
        [ "${exe% (deleted)}" = "$cible" ] && printf '%s\n' "$pid"
    done
    return 0
}

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

# LA VERSION SE LIT TOUJOURS, et non seulement quand on parle : le mot
# de la fin l'emploie, et `--quiet` la laissait vide.
VERSION=$(awk -F'"' '/^version = /{print $2; exit}' "$SOURCE_DIR/Cargo.toml" 2>/dev/null || echo '?')

# --- Ce qui est déjà là ------------------------------------------------
#
# Une installation par-dessus une autre est une MISE À JOUR : rien n'est
# retiré, rien n'est remis à zéro, et les réglages, les séries ajoutées
# et les points d'arrêt restent où ils sont. Le dire change ce qu'on lit
# ensuite — « installé » et « remplacé » ne se vérifient pas de la même
# façon.
DEJA=""
if [ -x "$BIN_DIR/$APP" ]; then
    DEJA=$("$BIN_DIR/$APP" --version 2>/dev/null | awk '{print $NF}')
    [ -n "$DEJA" ] || DEJA="?"
fi

# Et qui tourne, s'il tourne. Relevé MAINTENANT : après la pose, le lien
# `/proc/<pid>/exe` porte « (deleted) », et l'on ne saurait plus dire si
# l'instance est d'avant ou d'après.
OUVERTS=()
if [ -n "$DEJA" ]; then
    while IFS= read -r pid; do
        [ -n "$pid" ] && OUVERTS+=("$pid")
    done < <(instances_en_cours "$BIN_DIR/$APP")
fi

# Combien d'étapes : le compte est annoncé, donc il doit être juste. Il
# se fait APRÈS le relevé des fenêtres ouvertes, puisque le
# redémarrage en est une et qu'il n'a lieu que s'il y a quelque chose à
# redémarrer.
TOTAL=2
[ "$AVEC_COMPILATION" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_EXERCICES" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_CLI" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_LANCEUR" = 1 ] && TOTAL=$((TOTAL + 2))
[ "$AVEC_PATH" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_DOCK" = 1 ] && TOTAL=$((TOTAL + 1))
[ "$AVEC_REDEMARRAGE" = 1 ] && [ ${#OUVERTS[@]} -gt 0 ] && TOTAL=$((TOTAL + 1))

if [ "$SILENCIEUX" != 1 ]; then
    printf '\n'
    # Pas de tiret cadratin ni de ✓ dans un cadre : ces caractères
    # comptent pour un et se rendent sur deux colonnes dans beaucoup de
    # terminaux, et le bord droit tombe alors à côté.
    if [ -z "$DEJA" ]; then
        cadre "Installation de namsdec $VERSION"
        printf '  %sun atelier pour apprendre l'"'"'assembleur x86-64%s\n' "$GRIS" "$FIN"
    elif [ "$DEJA" = "$VERSION" ]; then
        cadre "Mise à jour de namsdec $VERSION"
        printf '  %smême version : le binaire est remplacé, vos réglages ne bougent pas%s\n' "$GRIS" "$FIN"
    else
        cadre "Mise à jour de namsdec $DEJA vers $VERSION"
        printf '  %srien n'"'"'est retiré : réglages, séries ajoutées et points d'"'"'arrêt restent%s\n' "$GRIS" "$FIN"
    fi
    [ "$ESSAI" = 1 ] && printf '\n  %s— essai à blanc : rien ne sera écrit —%s\n' "$JAUNE" "$FIN"

    # Ce qui va être fait, et l'option qui l'annule. Le script le faisait
    # sans le dire : on découvrait qu'il avait touché au fichier du shell
    # ou épinglé au dock une fois que c'était fait. Montrer les réglages
    # retenus AVANT d'agir, c'est laisser le choix de couper court.
    printf '\n  %sCe qui va être fait%s   %s(-h pour toutes les options)%s\n' \
        "$GRAS" "$FIN" "$GRIS" "$FIN"
    reglage "$AVEC_COMPILATION" "compiler ($PROFIL)"        "--no-build, --debug"
    reglage "$AVEC_STRIP"       "alléger le binaire"        "--no-strip"
    reglage "$AVEC_EXERCICES"   "poser les exercices"       "--no-exercises"
    reglage "$AVEC_CLI"         "poser namsdec-cli"         "--no-cli"
    reglage "$AVEC_LANCEUR"     "lanceur et icônes"         "--no-desktop"
    reglage "$AVEC_PATH"        "ajouter au PATH"           "--no-path"
    reglage "$AVEC_DOCK"        "épingler au dock"          "--no-dock"
    if [ ${#OUVERTS[@]} -gt 0 ]; then
        reglage "$AVEC_REDEMARRAGE" "redémarrer namsdec à la fin" "--no-restart"
    fi
    printf '\n    %sbinaire%s    %s\n' "$GRIS" "$FIN" "$BIN_DIR/$APP"
    printf '    %sdonnées%s    %s/%s\n' "$GRIS" "$FIN" "$PREFIX" "$APP"
    # CE QUI TOURNE, ET CE QU'IL VA LUI ARRIVER. Une fenêtre qui se
    # ferme sans prévenir au milieu d'une installation est ce qui fait
    # croire qu'on vient de désinstaller quelque chose. On le dit
    # AVANT, pendant qu'il est encore temps de couper court — et la
    # fenêtre, elle, reste ouverte tout le temps du travail.
    if [ ${#OUVERTS[@]} -gt 0 ]; then
        printf '\n'
        if [ ${#OUVERTS[@]} -eq 1 ]; then
            printf '    %s◆%s namsdec tourne %s(pid %s)%s\n' \
                "$CYAN" "$FIN" "$GRIS" "${OUVERTS[0]}" "$FIN"
        else
            printf '    %s◆%s namsdec tourne %s(%d fenêtres)%s\n' \
                "$CYAN" "$FIN" "$GRIS" "${#OUVERTS[@]}" "$FIN"
        fi
        if [ "$AVEC_REDEMARRAGE" = 1 ]; then
            printf '      %sil reste ouvert pendant toute la mise à jour, et sera%s\n' "$GRIS" "$FIN"
            printf '      %srefermé puis rouvert à la toute fin, sur la version neuve.%s\n' "$GRIS" "$FIN"
            printf '      %sEnregistrez ce qui compte : la fermeture est propre, pas magique.%s\n' "$JAUNE" "$FIN"
        else
            printf '      %sil n'"'"'est pas touché — la mise à jour sera prise%s\n' "$GRIS" "$FIN"
            printf '      %sà sa prochaine ouverture.%s\n' "$GRIS" "$FIN"
        fi
    fi
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
    BUILT_CLI="$SOURCE_DIR/target/release/$CLI_NAME"
else
    BUILT="$SOURCE_DIR/target/debug/$BIN_NAME"
    BUILT_CLI="$SOURCE_DIR/target/debug/$CLI_NAME"
fi
# `-p` NOMMÉMENT, les deux : le workspace ne construit par défaut que
# l'interface (`default-members`), et sans cela `--no-build` trouvait un
# binaire et pas l'autre.
PAQUETS=(-p namsdec-gui)
[ "$AVEC_CLI" = 1 ] && PAQUETS+=(-p namsdec-cli)
if [ "$AVEC_COMPILATION" = 1 ]; then
    if [ "$PROFIL" = release ]; then
        etape "Compilation (la première fois, comptez quelques minutes)"
        faire cargo build --release --manifest-path "$SOURCE_DIR/Cargo.toml" "${PAQUETS[@]}"
    else
        etape "Compilation de la version de débogage"
        detail "non optimisée : namsdec y sera plus lent, mais prêt en quelques secondes"
        faire cargo build --manifest-path "$SOURCE_DIR/Cargo.toml" "${PAQUETS[@]}"
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
poser 755 "$BUILT" "$BIN_DIR/$APP"
# Les symboles pèsent deux mégaoctets et ne servent qu'à lire une trace
# de panique. Celui de `target/` les garde — c'est lui qu'on ouvre dans
# un débogueur ; celui qu'on installe s'en passe. Le BuildID reste, de
# quoi apparier les deux si besoin.
if [ "$AVEC_STRIP" = 1 ] && [ "$PROFIL" = release ] && command -v strip >/dev/null; then
    faire strip "$BIN_DIR/$APP" 2>/dev/null || true
fi
[ "$ESSAI" = 1 ] || ok "$(du -h "$BIN_DIR/$APP" | cut -f1) installé"

# --- La commande d'analyse ---------------------------------------------
# Une commande qui n'est pas dans le PATH ne sert à personne : elle
# s'installe à côté de l'application, sous son propre nom.
if [ "$AVEC_CLI" = 1 ]; then
    etape "Installation de la commande d'analyse"
    detail "$BIN_DIR/$CLI_NAME"
    if [ "$ESSAI" != 1 ] && [ ! -x "$BUILT_CLI" ]; then
        # ON LE DIT ET ON CONTINUE. L'application, elle, est installée :
        # s'arrêter ici laisserait l'installation à moitié faite pour
        # une commande annexe. C'est le cas d'un `--no-build` après un
        # `cargo build` qui ne nommait que l'interface.
        avertir "$CLI_NAME introuvable : $BUILT_CLI
    (compilez-le par « cargo build --release -p namsdec-cli », ou passez --no-cli)"
    else
        poser 755 "$BUILT_CLI" "$BIN_DIR/$CLI_NAME"
        if [ "$AVEC_STRIP" = 1 ] && [ "$PROFIL" = release ] && command -v strip >/dev/null; then
            faire strip "$BIN_DIR/$CLI_NAME" 2>/dev/null || true
        fi
        [ "$ESSAI" = 1 ] || ok "$(du -h "$BIN_DIR/$CLI_NAME" | cut -f1) installé"
    fi
fi

# --- Exercices ---------------------------------------------------------
# Le parcours n'est pas dans le binaire : c'est un fichier de données,
# comme un thème. On l'installe donc, et une correction d'énoncé ne
# demande plus de recompiler. Ce que l'utilisateur a ajouté à côté n'est
# pas touché : seul le fichier livré est remplacé.
if [ "$AVEC_EXERCICES" = 1 ]; then
    etape "Installation des séries d'exercices"
    detail "$EXERCISES_DIR/$APP.json"
    poser 644 "$SOURCE_DIR/crates/namsdec-exercises/packs/namsdec.json" \
        "$EXERCISES_DIR/$APP.json"

    # Les séries supplémentaires vivent dans `plugins/exercices/`. Ce
    # sont des fichiers de même format que le parcours livré, et le
    # dossier est le contrat : y déposer un fichier suffit à l'installer,
    # sans toucher à ce script.
    for serie in "$SOURCE_DIR"/plugins/exercices/*.json; do
        [ -e "$serie" ] || break
        detail "$EXERCISES_DIR/$(basename "$serie")"
        poser 644 "$serie" "$EXERCISES_DIR/$(basename "$serie")"
    done
fi

# --- Icônes ------------------------------------------------------------
# Le SVG d'abord : c'est lui que les thèmes modernes préfèrent, à
# n'importe quelle taille. Les PNG restent là pour ce qui ne sait pas
# rendre du vectoriel.
if [ "$AVEC_LANCEUR" = 1 ]; then
    etape "Installation des icônes"
    detail "$ICON_DIR"
    poser 644 "$SOURCE_DIR/assets/$APP.svg" "$ICON_DIR/scalable/apps/$APP.svg"
    posees=0
    for size in 16 22 24 32 48 64 128 256 512; do
        png="$SOURCE_DIR/assets/icons/$APP-$size.png"
        if [ -f "$png" ]; then
            poser 644 "$png" "$ICON_DIR/${size}x${size}/apps/$APP.png"
            posees=$((posees + 1))
        fi
    done
    ok "1 vectorielle, $posees matricielles"

    # --- Lanceur -------------------------------------------------------
    etape "Installation du lanceur"
    detail "$DESKTOP_DIR/$APP.desktop"
    faire mkdir -p "$DESKTOP_DIR"
    # ÉCRIT À CÔTÉ, PUIS RENOMMÉ. `cat >` tronque : le lanceur restait
    # VIDE le temps d'être réécrit, et GNOME Shell, qui élague les
    # favoris dont il ne trouve plus le fichier, retirait l'icône du
    # dock pendant qu'on installait. C'est de là que venait
    # l'impression que l'installation désinstallait le programme.
    poser_depuis_stdin 644 "$DESKTOP_DIR/$APP.desktop" <<DESKTOP
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

# Pose l'épingle en fin de liste, en gérant la liste vide.
#
# Une fonction, et non deux fois le même couple de lignes : elle sert
# une première fois pour poser, une seconde pour reposer si le bureau a
# élagué. La garde d'essai à blanc est DEDANS, là où l'écriture se fait
# — écrite vingt lignes plus haut, dans un `elif` qui englobe tout, elle
# était juste et illisible, et le contrôle qui relit ces scripts ne
# pouvait pas la voir.
epingler() {
    [ "$ESSAI" = 1 ] && return 0
    liste=$1
    if [ "$liste" = "@as []" ] || [ "$liste" = "[]" ]; then
        gsettings set org.gnome.shell favorite-apps "['$APP.desktop']"
    else
        gsettings set org.gnome.shell favorite-apps "${liste%]}, '$APP.desktop']"
    fi
}

if [ "$AVEC_DOCK" = 1 ]; then
    etape "Épinglage au dock"
    if command -v gsettings >/dev/null && gsettings writable org.gnome.shell favorite-apps >/dev/null 2>&1; then
        favorites=$(gsettings get org.gnome.shell favorite-apps)
        if printf '%s' "$favorites" | grep -q "'$APP.desktop'"; then
            ok "déjà épinglé"
        elif [ "$ESSAI" = 1 ]; then
            detail "serait épinglé au dash de GNOME"
        else
            epingler "$favorites"
            # ON VÉRIFIE QUE L'ÉPINGLE A TENU, et ce n'est pas du zèle :
            # GNOME Shell ÉLAGUE de lui-même les favoris dont il ne
            # trouve pas le fichier `.desktop`. Juste après une
            # réinstallation, le lanceur vient d'être réécrit et le cache
            # des applications n'est pas forcément relu : le dash retire
            # alors l'entrée qu'on vient de poser, sans rien dire — et
            # l'on conclut que l'installation a désinstallé l'icône.
            # On relit, on laisse au bureau le temps de voir le fichier,
            # et l'on repose une fois.
            relu=$(gsettings get org.gnome.shell favorite-apps)
            if ! printf '%s' "$relu" | grep -q "'$APP.desktop'"; then
                sleep 1
                command -v update-desktop-database >/dev/null \
                    && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
                epingler "$(gsettings get org.gnome.shell favorite-apps)"
                relu=$(gsettings get org.gnome.shell favorite-apps)
            fi
            if printf '%s' "$relu" | grep -q "'$APP.desktop'"; then
                ok "épinglé"
            else
                # Dit, et non tu : une épingle qui ne tient pas est
                # exactement ce qu'on vient de corriger, et la taire
                # ferait croire que c'est fait.
                detail "le dash a refusé l'épingle — rouvrez la session, ou épinglez à la main"
            fi
        fi
    else
        detail "pas de dash GNOME ici — le lanceur suffit"
    fi
fi

# --- Redémarrage ------------------------------------------------------
#
# LA FENÊTRE EST RESTÉE OUVERTE JUSQU'ICI, et c'est voulu : la fermer
# d'entrée aurait fait attendre devant un écran vide pendant les deux
# minutes de compilation, pour rien. Le binaire est remplacé sous elle
# sans qu'elle s'en aperçoive — elle garde l'inode d'avant, qui vit tant
# qu'elle le tient.
#
# Maintenant que tout est en place, on la referme et on la rouvre sur la
# version neuve. SIGTERM et non SIGKILL : c'est la fermeture ordinaire,
# celle du bouton de la barre de titre, et l'application a le temps
# d'écrire ses réglages.
REDEMARRE=0
if [ "$AVEC_REDEMARRAGE" = 1 ] && [ ${#OUVERTS[@]} -gt 0 ]; then
    etape "Redémarrage de namsdec"
    if [ "$ESSAI" = 1 ]; then
        detail "les ${#OUVERTS[@]} fenêtre(s) seraient refermées puis rouvertes"
    else
        for pid in "${OUVERTS[@]}"; do
            kill -TERM "$pid" 2>/dev/null || true
        done
        # On attend qu'elles partent, sans dépasser dix secondes :
        # au-delà, c'est qu'une boîte de dialogue retient la fermeture,
        # et l'utilisateur vaut mieux qu'un SIGKILL dans le dos.
        reste=()
        for _ in $(seq 1 50); do
            reste=()
            for pid in "${OUVERTS[@]}"; do
                [ -d "/proc/$pid" ] && reste+=("$pid")
            done
            [ ${#reste[@]} -eq 0 ] && break
            sleep 0.2
        done
        if [ ${#reste[@]} -gt 0 ]; then
            # DIT, ET NON TU. Une fenêtre qui refuse de partir est
            # presque toujours une question posée à l'écran — un
            # travail non enregistré. On ne l'exécute pas d'autorité :
            # on la laisse, et l'on dit pourquoi rien n'a redémarré.
            avertir "${#reste[@]} fenêtre(s) ne se sont pas refermées — elles attendent sans doute une réponse.
    namsdec n'a pas été redémarré : répondez-leur, puis relancez « $APP »."
        else
            ok "refermé proprement"
            # `setsid` : la nouvelle fenêtre ne doit RIEN devoir à ce
            # terminal. Sans lui, fermer le terminal emporterait
            # l'application qu'on vient de rouvrir.
            setsid "$BIN_DIR/$APP" >/dev/null 2>&1 < /dev/null &
            # On laisse à la fenêtre le temps de paraître avant de dire
            # qu'elle est là : annoncer « rouvert » sur un `&` qui a
            # réussi ne prouverait que le fork.
            sleep 1
            if [ -n "$(instances_en_cours "$BIN_DIR/$APP")" ]; then
                ok "rouvert sur la version $VERSION"
                REDEMARRE=1
            else
                avertir "namsdec n'a pas rouvert — lancez « $APP » à la main."
            fi
        fi
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
printf '\n'
if [ -z "$DEJA" ]; then
    cadre "namsdec $VERSION est en place"
elif [ "$DEJA" = "$VERSION" ]; then
    cadre "namsdec $VERSION est à jour"
else
    cadre "namsdec est passé de $DEJA à $VERSION"
fi
printf '\n'
# CE QU'ON VIENT DE POSER, avec son poids : deux lignes qui disent où
# regarder le jour où quelque chose ne va pas. Les tailles sont lues sur
# le disque, non annoncées d'avance — c'est ce qui est là qui compte.
# Trois colonnes, et elles se tiennent : ce que c'est, où c'est, ce que
# cela pèse. Les chemins n'ont pas la même longueur — on cale, sinon la
# dernière colonne fait des vagues et ne se lit plus d'un trait.
#
# Le chemin s'écrit à partir de `~` : `/home/fred/.local/bin/namsdec`
# prend dix caractères de plus pour ne rien dire de plus, et c'est dix
# caractères pris à la colonne qu'on veut aligner.
recapituler() {
    local nom="$1" chemin="$2" poids="$3"
    printf '  %s✓%s  ' "$VERT" "$FIN"
    caler "$nom" 15
    caler "${chemin/#$HOME/\~}" 40
    printf '%s%s%s\n' "$GRIS" "$poids" "$FIN"
}
if [ -x "$BIN_DIR/$APP" ]; then
    recapituler "l'application" "$BIN_DIR/$APP" "$(du -h "$BIN_DIR/$APP" | cut -f1)"
fi
if [ "$AVEC_CLI" = 1 ] && [ -x "$BIN_DIR/$CLI_NAME" ]; then
    recapituler "l'analyse" "$BIN_DIR/$CLI_NAME" "$(du -h "$BIN_DIR/$CLI_NAME" | cut -f1)"
fi
if [ "$AVEC_EXERCICES" = 1 ] && [ -d "$EXERCISES_DIR" ]; then
    series=$(find "$EXERCISES_DIR" -maxdepth 1 -name '*.json' 2>/dev/null | wc -l)
    recapituler "les exercices" "$EXERCISES_DIR" "$series série(s)"
fi
if [ "$AVEC_LANCEUR" = 1 ] && [ -f "$DESKTOP_DIR/$APP.desktop" ]; then
    recapituler "le lanceur" "$DESKTOP_DIR/$APP.desktop" ""
fi
# RIEN N'A ÉTÉ RETIRÉ, et il faut le dire à qui vient de mettre à jour :
# c'est la question qu'on se pose en relançant un installateur.
if [ -n "$DEJA" ]; then
    printf '\n  %sVos réglages, vos séries ajoutées et vos points d'"'"'arrêt n'"'"'ont pas bougé.%s\n' \
        "$GRIS" "$FIN"
fi
printf '\n'
if [ "$REDEMARRE" = 1 ]; then
    printf '  %sEn cours%s        namsdec vient de rouvrir sur cette version\n' "$GRAS" "$FIN"
else
    printf '  %sLancer%s          %s\n' "$GRAS" "$FIN" "$APP"
    [ "$AVEC_LANCEUR" = 1 ] && printf '  %sou%s              depuis le menu des applications\n' "$GRAS" "$FIN"
fi
printf '  %sPremiers pas%s    Aide ▸ Guide, ou la touche %sF1%s\n' "$GRAS" "$FIN" "$CYAN" "$FIN"
printf '  %sMettre à jour%s   ./install.sh %s(rien n'"'"'est retiré)%s\n' "$GRAS" "$FIN" "$GRIS" "$FIN"
printf '  %sDésinstaller%s    ./uninstall.sh\n' "$GRAS" "$FIN"
if [ -n "$RAPPEL_PATH" ]; then
    printf '\n  %s⚠ %s%s\n' "$JAUNE" "$RAPPEL_PATH" "$FIN"
fi
if [ ${#OUVERTS[@]} -gt 0 ] && [ "$AVEC_REDEMARRAGE" != 1 ]; then
    printf '\n  %s⚠ namsdec tourne encore sur la version d'"'"'avant : refermez-le et%s\n' "$JAUNE" "$FIN"
    printf '  %s  rouvrez-le pour prendre la mise à jour.%s\n' "$JAUNE" "$FIN"
fi
printf '\n'
