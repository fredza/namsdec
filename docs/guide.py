#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Le guide de l'utilisateur de namsdec, en PDF.

Rien n'est écrit en dur de ce que le dépôt sait déjà : la version vient
de Cargo.toml, le nombre d'exercices du fichier de la série, le nombre
de fiches du manuel. Un guide qui annonce quarante-sept exercices quand
il y en a quatre-vingt-seize est pire qu'une absence de guide.

Usage :
    python3 docs/guide.py          # écrit docs/guide-namsdec-<version>.pdf
"""

import io
import json
import re
import sys
from datetime import date
from pathlib import Path

from reportlab.lib import colors
from reportlab.lib.enums import TA_JUSTIFY
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import mm
from reportlab.platypus import (
    BaseDocTemplate,
    Frame,
    KeepTogether,
    NextPageTemplate,
    PageBreak,
    PageTemplate,
    Paragraph,
    Preformatted,
    Spacer,
    Table,
    TableStyle,
)

RACINE = Path(__file__).resolve().parent.parent

# --- Ce que le dépôt sait ------------------------------------------------

def version():
    texte = (RACINE / "Cargo.toml").read_text(encoding="utf-8")
    trouvé = re.search(r'^version\s*=\s*"([^"]+)"', texte, re.M)
    if not trouvé:
        raise SystemExit("version introuvable dans Cargo.toml")
    return trouvé.group(1)

def exercices():
    pack = json.loads(
        (RACINE / "crates/namsdec-exercises/packs/namsdec.json").read_text(encoding="utf-8")
    )
    paliers = {}
    for exercice in pack["exercises"]:
        paliers[exercice["level"]] = paliers.get(exercice["level"], 0) + 1
    avec_but = sum(1 for e in pack["exercises"] if e.get("goal"))
    return len(pack["exercises"]), avec_but, paliers

def atelier():
    """La seconde série, celle qui s'installe comme un greffon.

    Elle est comptée séparément parce qu'elle s'installe séparément :
    annoncer un total qui les mélange ferait croire qu'on obtient tout
    en une fois."""
    pack = json.loads(
        (RACINE / "plugins/exercices/atelier.json").read_text(encoding="utf-8")
    )
    paliers = {}
    for exercice in pack["exercises"]:
        paliers[exercice["level"]] = paliers.get(exercice["level"], 0) + 1
    return len(pack["exercises"]), paliers

def tests():
    """Combien de tests automatiques accompagnent le programme."""
    total = 0
    for source in (RACINE / "crates").rglob("*.rs"):
        total += len(re.findall(r"^\s*#\[test\]", source.read_text(encoding="utf-8"), re.M))
    return total


def fiches():
    texte = (RACINE / "crates/namsdec-doc/src/table.rs").read_text(encoding="utf-8")
    return texte.count("kind: Kind::")

def fonctions_de_la_calculatrice():
    """Combien de fonctions la calculatrice connaît, lues dans sa table.

    Comptées dans le code plutôt qu'écrites ici : le guide dirait
    « dix-huit » longtemps après qu'il y en a quarante-sept, et c'est
    exactement ce qui vient d'arriver.
    """
    texte = (RACINE / "crates/namsdec-gui/src/calc.rs").read_text(encoding="utf-8")
    debut = texte.index("pub const FUNCTIONS")
    fin = texte.index("\n];", debut)
    return len(re.findall(r'^\s*\(\s*\n?\s*"', texte[debut:fin], re.M))


def raccourcis():
    """Les raccourcis par défaut, lus dans le code qui les pose."""
    texte = (RACINE / "crates/namsdec-gui/src/keybindings.rs").read_text(encoding="utf-8")
    # Sans les modules de test : ils posent leurs propres tables pour
    # vérifier des règles, et le guide les lisait comme de vrais
    # raccourcis — on y trouvait « Stop : O », qui n'existe nulle part
    # dans l'application.
    #
    # Les accolades se comptent, faute de quoi on ne sait pas où un
    # module finit : il y en a un AVANT les définitions, et couper au
    # premier venu ne laissait plus rien.
    texte = _sans_les_tests(texte)
    trouvés = []
    for bloc in re.finditer(
        r"map\.insert\(\s*Action::(\w+),\s*Chord::(?:Simple\(KeyboardShortcut::new\(\s*"
        r"Modifiers::([\w:|+.()\s]+?),\s*Key::(\w+),?\s*\)\)|leader\(&\[([^\]]+)\]\))",
        texte,
    ):
        action, mods, touche, séquence = bloc.group(1), bloc.group(2), bloc.group(3), bloc.group(4)
        if séquence:
            touches = " puis ".join(
                _touche(t.strip().replace("Key::", ""))
                for t in séquence.split(",")
                if t.strip()
            )
            trouvés.append((action, touches))
        else:
            mods = re.sub(r"\s+", " ", mods.replace("Modifiers::", "")
                          .replace(".plus(", " + ").replace(")", "")
                          .replace("|", " + ").replace("NONE", "")).strip(" +")
            mods = "+".join(_touche(m) for m in mods.split(" + ") if m.strip())
            touche = _touche(touche)
            trouvés.append((action, f"{mods}+{touche}" if mods else touche))
    return trouvés


def _sans_les_tests(texte):
    """Le code, débarrassé de ses modules `#[cfg(test)]`."""
    out = []
    reste = texte
    while True:
        début = reste.find("#[cfg(test)]")
        if début < 0:
            out.append(reste)
            break
        out.append(reste[:début])
        # De la première accolade du module jusqu'à celle qui la ferme.
        ouvre = reste.find("{", début)
        if ouvre < 0:
            break
        profondeur, i = 0, ouvre
        while i < len(reste):
            if reste[i] == "{":
                profondeur += 1
            elif reste[i] == "}":
                profondeur -= 1
                if profondeur == 0:
                    break
            i += 1
        reste = reste[i + 1:]
    return "".join(out)


def _touche(nom):
    """Le nom d'une touche, tel qu'on le dit en français."""
    return {
        "CTRL": "Ctrl", "SHIFT": "Maj", "ALT": "Alt", "COMMAND": "Cmd",
        "Space": "Espace", "Escape": "Échap",
        # Les touches de ponctuation : egui les nomme, on les écrit.
        # « Slash » dans un guide ne dit pas sur quoi appuyer.
        "Slash": "/", "Backslash": "\\", "Semicolon": ";", "Quote": "'",
        "Comma": ",", "Period": ".", "Minus": "-", "Plus": "+", "Equals": "=",
        "OpenBracket": "[", "CloseBracket": "]",
    }.get(nom, nom)


# --- Le contenu ----------------------------------------------------------

def contenu(v, total_exos, avec_but, paliers, nb_fiches, nb_tests, nb_fonctions,
            total_atelier, atelier_paliers):
    """Le guide, section par section.

    Chaque élément est un couple (genre, texte) : 'h1' un chapitre, 'h2'
    une section, 'p' un paragraphe, 'code' un bloc de code, 'liste' une
    énumération, 'table' un tableau, 'note' un encadré.
    """
    return [
        ("h1", "Ce que namsdec fait"),
        ("p",
         "namsdec est un atelier pour <b>apprendre l'assembleur x86-64</b> sous Linux, et "
         "pour <b>lire des binaires qu'on n'a pas écrits</b>. Il assemble votre programme "
         "avec le vrai <font face='Courier'>nasm</font>, le lie avec le vrai "
         "<font face='Courier'>ld</font>, puis l'exécute instruction par instruction dans "
         "un processeur simulé dont il montre tout : les registres, les indicateurs, la "
         "pile, le tas, la mémoire et ses droits."),
        ("p",
         "La différence avec un débogueur tient en une phrase : un débogueur montre "
         "<i>ce qui se passe</i>, namsdec explique <i>pourquoi</i>. L'adresse qu'un opérande "
         "désigne est calculée avant l'exécution, un saut conditionnel annonce s'il sera "
         "pris et selon quelle règle, un appel système est écrit comme on l'écrirait en C, "
         "et une faute mémoire dit ce qu'il fallait faire à la place."),
        ("note",
         "Cette version est une <b>préversion</b>. Elle est utilisable et testée — "
         "%d tests automatiques l'accompagnent — mais son interface et ses formats de "
         "fichiers peuvent encore changer d'une version à l'autre." % nb_tests),

        ("h2", "À qui il s'adresse"),
        ("liste", [
         "À qui apprend l'assembleur et veut <b>voir</b> ce que chaque instruction fait.",
         "À qui connaît un langage de haut niveau et veut comprendre ce qu'il devient.",
         "À qui débute en rétro-ingénierie et cherche un outil qui explique au lieu "
         "de seulement afficher.",
        ]),

        ("h1", "Installer"),
        ("h2", "Ce qu'il faut avoir"),
        ("p",
         "namsdec appelle les outils du système plutôt que de les réécrire : vous "
         "obtenez donc exactement le binaire que la ligne de commande aurait produit."),
        ("table", [
            ["Outil", "Rôle", "Sur Fedora", "Sur Debian / Ubuntu"],
            ["nasm", "assembler", "sudo dnf install nasm", "sudo apt install nasm"],
            ["ld", "lier", "sudo dnf install binutils", "sudo apt install binutils"],
            ["cargo", "compiler namsdec", "https://rustup.rs", "https://rustup.rs"],
        ]),
        ("h2", "L'installation"),
        ("code",
         "git clone https://github.com/fredza/namsdec\n"
         "cd namsdec\n"
         "./install.sh"),
        ("p",
         "Le script compile en mode <i>release</i>, puis installe le binaire dans "
         "<font face='Courier'>~/.local/bin</font>, l'icône et le lanceur là où le bureau "
         "va les chercher, et <b>le parcours d'exercices</b> dans "
         "<font face='Courier'>~/.local/share/namsdec/exercises</font>. Rien n'est écrit "
         "hors de votre dossier personnel : aucun droit d'administrateur n'est demandé. "
         "<font face='Courier'>./uninstall.sh</font> défait exactement cela."),
        ("p",
         "<font face='Courier'>./install.sh --help</font> donne les options. Les deux qui "
         "servent le plus : <b>--dry-run</b> dit ce qui serait fait sans rien écrire — de "
         "quoi lire ce qu'un script va toucher avant de le laisser faire —, et "
         "<b>--debug</b> installe la version non optimisée, prête en quelques secondes au "
         "lieu de quelques minutes."),
        ("table", [
            ["Option", "Ce qu'elle fait"],
            ["--dry-run, -n", "dire ce qui serait fait, sans rien écrire"],
            ["--debug", "la version non optimisée, prête en quelques secondes"],
            ["--prefix DIR", "où vont icônes, lanceur et exercices"],
            ["--bin-dir DIR", "où va le binaire"],
            ["--no-desktop", "pas de lanceur ni d'icônes"],
            ["--no-dock", "ne pas épingler au dock (GNOME)"],
            ["--no-path", "ne pas toucher au fichier de configuration du shell"],
            ["--no-exercises", "ne pas installer la série livrée"],
            ["--no-strip", "garder les symboles de débogage"],
            ["--no-build", "poser le binaire déjà construit, sans recompiler"],
            ["--quiet, -q", "ne dire que l'essentiel"],
            ["--no-color", "pas de couleurs (NO_COLOR est respecté aussi)"],
        ]),
        ("note",
         "<b>L'épingle du dock est vérifiée, pas supposée.</b> GNOME Shell élague de "
         "lui-même les favoris dont il ne trouve pas le fichier "
         "<font face='Courier'>.desktop</font> ; juste après une réinstallation, le "
         "lanceur vient d'être réécrit et le cache des applications n'est pas forcément "
         "relu. Le dash retirait alors l'entrée qu'install.sh venait de poser, le script "
         "annonçait « épinglé », et l'on concluait que l'installation avait "
         "<i>désinstallé</i> l'icône. Il relit maintenant, laisse une seconde au bureau, "
         "repose une fois — et si l'épingle ne tient toujours pas, il le dit au lieu "
         "d'annoncer le contraire."),
        ("p",
         "<font face='Courier'>./uninstall.sh</font> prend <b>--prefix</b>, <b>--bin-dir</b>, "
         "<b>--dry-run</b>, <b>--quiet</b> "
         "et <b>--no-color</b> de la même façon. <b>--purge</b> retire en plus vos réglages "
         "et vos séries d'exercices ; sans lui, ils sont gardés — on désinstalle souvent "
         "pour réinstaller, et perdre ses raccourcis au passage serait une mauvaise "
         "surprise."),

        ("p",
         "En ligne de commande : <font face='Courier'>namsdec fichier.asm</font> ouvre "
         "ce fichier, <font face='Courier'>namsdec --version</font> donne la version et "
         "<font face='Courier'>namsdec --help</font> rappelle le reste. Le lanceur "
         "installé déclare namsdec pour les fichiers <font face='Courier'>.asm</font> : "
         "« Ouvrir avec » depuis le gestionnaire de fichiers marche donc aussi."),

        ("h1", "Le premier programme"),
        ("p",
         "Au premier lancement, l'éditeur contient un programme d'accueil. Trois gestes "
         "suffisent pour le voir tourner :"),
        ("liste", [
         "<b>Assembler</b> (Ctrl+Maj+B) : nasm et ld font leur travail. Les erreurs "
         "apparaissent dans l'onglet Diagnostics, chacune cliquable vers sa ligne.",
         "<b>Pas à pas</b> (Espace puis s puis i) : une instruction à la fois. Le bandeau "
         "du panneau droit montre ce qui vient, puis ce qui a changé.",
         "<b>Exécuter</b> (Espace puis p puis r) : jusqu'au bout, ou jusqu'au premier point "
         "d'arrêt.",
        ]),
        ("p",
         "Le programme s'arrête sur <font face='Courier'>hlt</font> ou sur l'appel système "
         "<font face='Courier'>exit</font>. Un programme que rien n'arrête est signalé dès "
         "l'assemblage : il continuerait dans ce qui suit son code, ce qui n'a aucun sens."),

        ("h1", "La fenêtre"),
        ("h2", "La barre de titre"),
        ("p",
         "namsdec dessine sa propre décoration : les menus <i>sont</i> la barre de "
         "titre, ce qui économise une rangée de trente pixels au-dessus d'une "
         "application qui a déjà une barre de menus. Trois pastilles au coin haut "
         "gauche — fermer, réduire, agrandir —, le nom du fichier à la "
         "suite du menu Aide, et le reste de la bande déplace la fenêtre ; le "
         "double-clic l'agrandit, les bords la redimensionnent."),
        ("p",
         "C'est là aussi que vivent les <b>sept indicateurs</b> (CF, PF, AF, ZF, SF, DF, "
         "OF), en pastilles : la barre de titre est toujours visible et ne se replie "
         "jamais, on les a donc sous les yeux à chaque pas, à droite du nom du fichier — toujours affichés, tous à zéro tant qu'aucun programme n'a tourné. "
         "Vert quand l'indicateur est levé, grisé sinon. Ce sont de simples <b>témoins</b> : "
         "on les lit, on ne les règle pas — ils disent ce que la dernière instruction a "
         "produit. Le survol les présente en entier — nom développé, rôle, et s'ils sont "
         "posés ou retombés. DF compris, que les instructions de chaîne lisent."),
        ("p",
         "Les fenêtres internes portent la <b>même pastille</b> de fermeture, au "
         "même endroit. Une seule façon de fermer quelque chose, partout."),
        ("h2", "Ce qui se clique, et comment on le sait"),
        ("p",
         "Beaucoup de textes mènent quelque part dans namsdec : une adresse va au "
         "listing ou aux octets, un nom de fonction se renomme, une empreinte se copie, "
         "un mnémonique du sommaire ouvre sa fiche, une ligne du désassemblage emmène à "
         "sa source. La question qui vient est « comment sait-on qu'un texte se clique, "
         "et qu'un autre non ? »"),
        ("p",
         "<b>Deux signes, toujours les mêmes.</b> Le pointeur devient une <b>main</b>, "
         "et le texte se <b>souligne</b> sous elle. Le soulignement est le signe que "
         "tout le monde connaît, et il ne coûte rien au repos : la fenêtre reste calme, "
         "et ce qu'on survole se déclare. Quand c'est une <b>rangée entière</b> qui se "
         "clique — une ligne de la carte mémoire, un résultat de recherche du manuel —, "
         "c'est la rangée entière qui se souligne, et non le seul mot coloré qui s'y "
         "trouve."),
        ("note",
         "La règle est tenue par un contrôle qui lit le code : une étiquette ne peut "
         "devenir cliquable que par les deux fonctions qui posent ces signes, et un "
         "panneau neuf qui l'oublierait ferait échouer la suite de tests. C'est la seule "
         "façon qu'une grammaire d'interface survive à ce qu'on ajoute après."),
        ("p",
         "Un panneau vide <b>dit ce qui manque et propose ce qui le corrige</b> : un "
         "panneau simplement vide se lit comme cassé. Au démarrage, le bandeau "
         "des registres annonce qu’aucun binaire n’est assemblé, dit ce que "
         "l’assemblage montrera, et porte le bouton qui le lance. Chaque onglet du "
         "panneau dit, lui, ce qu’il affichera — la pile, les pages de "
         "mémoire, les blocs alloués, les octets de chaque ligne. C’est "
         "l’écran qu’on a sous les yeux la première fois."),
        ("p",
         "Et chaque panneau dit ce qu’il montrera <b>dans ses propres mots</b> : la "
         "console annonce la sortie du programme, l’onglet Pile la pile et la chaîne "
         "des appels. Répéter le même constat à trois endroits d’un même "
         "écran n’apprend rien la deuxième fois, et agace la troisième."),
        ("p",
         "La <b>forme</b> est la même partout : le constat, puis l’explication, "
         "centrés l’un sur l’autre. Les fenêtres l’écrivaient de deux façons — un "
         "bloc centré ici, une phrase seule alignée à gauche là — et deux formes "
         "pour un même rôle se remarquent sans qu’on sache les nommer."),
        ("p",
         "Une fenêtre s’ouvre enfin <b>à la taille de ce qu’elle montre</b>. Sans "
         "binaire, le graphe prenait quatre-vingt-douze pour cent de l’écran pour "
         "une phrase. Ouverte vide puis remplie, elle garde sa petite taille : elle "
         "se redimensionne, et son contenu défile."),
        ("p",
         "Les <b>poignées</b> se disent de la même façon : le bord du panneau de "
         "droite, celui qui le partage entre le bandeau et la vue, celui de la "
         "console. Rien ne les distingue d'un filet décoratif — ils sont gris et "
         "larges d'un pixel comme les autres —, c'est le pointeur qui change de "
         "forme en passant dessus."),
        ("p",
         "<b>Ce qui se clique change le pointeur.</b> Une étiquette cliquable ne "
         "ressemble à rien — c'est du texte —, et namsdec en a douze : un nom de "
         "registre qu'on surveille, une valeur qu'on remplace, une ligne de pile où "
         "l'on va, une zone mémoire, un diagnostic de nasm. La main se voit en "
         "passant, là où une infobulle demande qu'on se soit déjà arrêté."),
        ("p",
         "Une commande <b>grisée dit pourquoi</b> elle l'est : survolez « Exécuter » "
         "avant d'avoir assemblé et namsdec répond « Rien à exécuter : assemblez "
         "d'abord le programme ». Une commande éteinte qui se tait passe pour cassée ; "
         "celle qui dit ce qui manque enseigne l'ordre des gestes."),
        ("p",
         "Les boutons sont <b>plats dans les menus et encadrés ailleurs</b>. Dans un "
         "menu, la platitude est le dessin ; dans une fenêtre, un bouton qui ne se "
         "distingue d’une étiquette qu’au survol ne se trouve pas. Ce qu’on "
         "peut cliquer doit se voir sans qu’on le cherche."),
        ("p",
         "L’<b>état du processeur ne défile jamais</b> : c’est ce qu’on "
         "regarde entre deux pas, et devoir le faire défiler pour voir R15 ou ZF "
         "coupe la lecture. Il garde sa hauteur entière quelle que soit la taille de "
         "la fenêtre ; quand la place manque, c’est la <b>prochaine "
         "instruction</b> qui cède — elle se replie d’un clic, les registres "
         "non. Ce qui se replie porte un <b>chevron</b> devant son titre, "
         "pointant vers le bas quand c’est ouvert et vers la droite quand "
         "c’est fermé. C’était naguère les lettres « v » et « > » : un « v » "
         "gris posé devant un titre se lit comme une lettre orpheline, et "
         "rien ne disait que le titre se repliait."),
        ("p",
         "Les couleurs sont tenues à un <b>plancher de contraste</b> : tout texte actif "
         "se détache de son fond d’au moins 3 pour 1, la mesure des recommandations "
         "d’accessibilité, dans chacun des trois thèmes livrés. Une commande "
         "inactive descend plus bas — c’est ce que « grisé » veut dire — mais reste "
         "visible. Ce n’est pas une intention : un test mesure ce qui est peint, "
         "compose chaque texte avec le fond qu’il a réellement sous lui, et refuse "
         "la compilation en dessous. Un thème installé ensuite n’y est pas soumis, "
         "et c’est à son auteur d’y veiller."),
        ("note",
         "Préférences › Général rend la barre du système à "
         "qui la préfère. Tout ce que le système faisait — déplacer, "
         "agrandir, redimensionner — revient à notre charge quand on la dessine "
         "soi-même, et il faut pouvoir revenir en arrière si un bureau s'y prend "
         "autrement."),

        ("h2", "L'éditeur"),
        ("p",
         "L'éditeur a deux modes, comme vim : <b>NORMAL</b> pour lire et se déplacer, "
         "<b>INSERTION</b> (touche i) pour écrire, Échap pour revenir. Le mode se lit dans "
         "la barre d'état."),
        ("p",
         "La gouttière, à gauche, porte trois choses : le numéro de ligne, le point d'arrêt "
         "qu'un clic pose ou retire, et une <b>bande de chaleur</b> qui dit combien de fois "
         "la ligne a tourné — verte pour un seul passage, rouge pour la plus chaude du "
         "programme, jaune et orange entre les deux. Le compte exact vient au survol, et la "
         "légende sous l'éditeur donne l'échelle."),
        ("p",
         "Quand le programme est arrêté quelque part, la ligne où il l'est porte une "
         "<b>bande pâle</b> sur toute la largeur, en plus de la flèche dans la gouttière. "
         "La flèche est au bord de l'écran ; la bande est là où l'œil lit le code."),
        ("p",
         "Avancer d'un pas <b>ramène le code sous les yeux</b>. Si l'on avait fait défiler "
         "l'éditeur vers la droite — pour lire le bout d'une longue ligne de commentaire —, "
         "la vue y restait, et le pas suivant laissait la ligne courante visible en hauteur "
         "mais les instructions et la gouttière sorties à gauche : on ne voyait plus que les "
         "commentaires. Chaque pas remet maintenant le défilement horizontal à zéro, le temps "
         "de quelques images, puis vous laisse défiler à la main."),
        ("p",
         "Survolez la gouttière et un <b>point pâle</b> paraît là où le point d'arrêt se "
         "posera ; le pointeur devient une main, et une infobulle dit ce que le clic fera. "
         "Rien de tout cela ne s'apprend dans un manuel qu'on n'a pas encore ouvert — la "
         "colonne le dit elle-même."),
        ("p",
         "<b>Fichier > Fermer le fichier</b> (Ctrl+W) ferme celui qu'on regarde, y compris "
         "quand c'est le seul ouvert : on revient alors à un éditeur vide. L'entrée "
         "disparaissait autrefois du menu dès qu'il ne restait qu'un fichier, c'est-à-dire "
         "au moment où on la cherchait."),
        ("p",
         "Plusieurs fichiers peuvent être ouverts à la fois : une barre d'onglets apparaît "
         "au-dessus de l'éditeur <b>dès le premier écran</b>, chacun gardant son "
         "texte, son curseur, ses points d'arrêt et son exécution en cours. Le "
         "<b>+</b> au bout de la file crée un <b>programme neuf</b> (Ctrl+N) dans son "
         "propre onglet ; le <b>×</b> ferme celui qu'il suit. Un onglet dont le texte "
         "n'est pas enregistré porte un <b>rond</b> à la place de sa croix — la croix "
         "revient quand on s'en approche. Les programmes neufs se numérotent : "
         "« sans titre 2 », « sans titre 3 ». Ctrl+Tab passe au suivant, Ctrl+W ferme, "
         "et <b>Fichier > Ouvrir un fichier…</b> (Ctrl+O) ouvre un fichier existant, "
         "lui aussi dans son onglet."),

        ("h2", "Deux rangées : le fichier, et la façon de le regarder"),
        ("p",
         "Sous la barre des fichiers court une seconde rangée, plus discrète : les "
         "<b>vues</b>. La première rangée dit <i>quel</i> programme on travaille, la "
         "seconde <i>comment</i> on le regarde. On les distingue à l'œil sans y penser : "
         "un onglet de fichier est un rectangle plein, marqué en haut et muni d'une "
         "croix ; un onglet de vue est une étiquette soulignée, et il ne se "
         "ferme pas — une façon de regarder ne s'abandonne pas, on en change."),
        ("p",
         "Neuf vues, rangées dans l'ordre d'une lecture et non dans celui du menu : ce "
         "qu'on écrit, puis ce que la machine en a fait, puis ce qu'on en comprend, puis "
         "ce qu'on observe quand ça tourne, et enfin ce qu'on y change."),
        ("table", [
            ["Onglet", "Ce qu'il montre"],
            ["Source", "l'éditeur : ce qu'on écrit"],
            ["Désassemblage", "le programme tel que le processeur le lit"],
            ["Déroulement", "les blocs et les sauts qui les relient"],
            ["Décompilation", "le pseudo-code d'un outil extérieur (module)"],
            ["Analyse", "les vingt-cinq rubriques de rétro-ingénierie"],
            ["Chronologie", "le chemin parcouru, pas à pas"],
            ["Points d'arrêt", "où l'on s'arrête, et à quelle condition"],
            ["Registres étendus", "SSE, AVX, AVX-512 et la pile du 8087"],
            ["Correctifs", "ce qu'on a changé dans les octets"],
        ]),
        ("p",
         "L'onglet porte un <b>nom court</b> — « Analyse », « Déroulement » — et dit son "
         "titre entier au survol : « Analyse du binaire », « Déroulement du programme ». "
         "Neuf titres entiers ne tiennent pas dans une rangée, et un onglet se cherche à "
         "sa place avant de se lire. <b>Décompilation</b> n'a d'onglet que si son module "
         "est allumé (Préférences › Modules) : un onglet qui ne mène à rien est pire "
         "qu'un onglet absent."),
        ("p",
         "Le <b>menu Outils</b>, la <b>palette</b> (Ctrl+Maj+P) et les raccourcis "
         "mènent aux mêmes vues : ils choisissent l'onglet. Le bouton de l'onglet RE du "
         "panneau latéral, « Analyse du binaire… », fait de même."),
        ("p",
         "Le bouton <b>Détacher</b>, en haut à droite de la rangée, sort la vue active "
         "dans une fenêtre à elle. C'est ce qu'il faut pour lire la source et le "
         "désassemblage <b>côte à côte</b>, ce qu'un onglet interdit par construction. "
         "L'onglet disparaît alors de la rangée : une vue est à un seul endroit à la "
         "fois. <b>Fermer la fenêtre la ramène</b> dans la rangée — elle n'est jamais "
         "perdue, elle est quelque part. La <b>source</b>, elle, ne se détache pas : "
         "c'est la vue qui reste quand les autres partent."),
        ("p",
         "Sur un écran étroit, la rangée <b>passe à la ligne</b> plutôt que de raboter "
         "ses onglets : à 800 x 520, le plus petit affichage que namsdec accepte, les "
         "neuf noms tiennent sur trois lignes, et aucun n'est coupé. Un onglet à moitié "
         "peint est pire qu'un onglet absent, parce qu'on croit l'avoir lu."),
        ("note",
         "Ces huit vues vivaient en fenêtres flottantes, et l'on pouvait les agrandir "
         "jusqu'à occuper tout l'établi : c'était un onglet qui s'ignorait, en moins "
         "bien. La fenêtre recouvrait l'éditeur sans le dire, il fallait la refermer "
         "pour retrouver sa source, et rien à l'écran ne disait qu'elle existait — il "
         "fallait retourner au menu pour le savoir. La rangée dit les deux choses d'un "
         "coup : ce qu'on regarde, et ce qu'on pourrait regarder. L'idée vient de "
         "Cutter, dont les onglets du bas sont les vues d'un même objet ; elle est ici "
         "en haut, parce que le bas de l'établi porte déjà la console et ses propres "
         "onglets."),

        ("p",
         "<b>Texte ou binaire&nbsp;?</b> namsdec le décide à la <b>signature</b> du "
         "fichier, et non à son nom ni à son encodage. Un ELF, un exécutable Windows, "
         "une archive, un PDF, une image partent au désassembleur et aux panneaux RE ; "
         "le reste va dans l'éditeur, à condition de ne contenir aucun octet nul et "
         "presque aucun caractère de contrôle. Le critère précédent — « tous les "
         "octets forment de l'UTF-8 valide » — ne démontrait rien&nbsp;: les octets "
         "nuls sont de l'UTF-8 valide, et un petit exécutable satisfait souvent ce "
         "test. Il atterrissait alors dans l'éditeur de source."),

        ("note",
         "Un ELF écrit pour un AUTRE PROCESSEUR est refusé en le nommant : "
         "« ceci est un ELF AArch64 : namsdec ne lit que x86-64 ». Ses octets restent "
         "consultables — l'éditeur binaire, les chaînes, les sections —, mais aucun "
         "listing ne prétend plus les comprendre. Auparavant, seuls « ELF 64 bits » et "
         "« petit-boutiste » étaient vérifiés : un binaire ARM passait, et se lisait "
         "en x86-64 sur toute sa longueur."),

        ("note",
         "<b>Ce qui se clique le dit sans qu'on le survole.</b> Un texte cliquable porte "
         "un trait sous lui, estompé au repos et franc au survol. Il ne paraissait "
         "qu'au survol, au nom d'une page calme&nbsp;; mais un signe qui n'apparaît qu'une "
         "fois qu'on est dessus ne sert qu'à celui qui savait déjà — on ne survole pas les "
         "textes au hasard pour voir lesquels se cliquent. Les RANGÉES entières d'une "
         "liste, elles, gardent le trait au survol seulement&nbsp;: un trait sous chacune "
         "des trois cents lignes d'un listing ne serait plus un signe, seulement du bruit."),

        ("h2", "Ce que l'éditeur sait faire"),
        ("p",
         "En mode NORMAL, les déplacements sont ceux de vim : "
         "<font face='Courier'>h j k l</font> pour un caractère ou une ligne, "
         "<font face='Courier'>w b e</font> pour un mot, <font face='Courier'>0</font> et "
         "Fin pour les bords de la ligne, <font face='Courier'>gg</font> et "
         "<font face='Courier'>G</font> pour ceux du fichier. La notion de mot est celle de "
         "vim, non celle d'un traitement de texte : dans "
         "<font face='Courier'>mov qword [rbp-0x18], rax</font>, "
         "<font face='Courier'>w</font> s'arrête sur le crochet puis sur "
         "<font face='Courier'>rbp</font>, au lieu de sauter la moitié de la ligne."),
        ("p",
         "<b>L'éditeur n'a pas de barre de défilement à demeure.</b> Elle prenait sa "
         "place deux fois&nbsp;: la largeur qu'elle réserve en permanence, et un liseré "
         "clair le long du code pour une information qu'on a déjà — la gouttière numérote "
         "les lignes, la barre d'état dit où est le curseur. Elle paraît au survol de la "
         "zone, de sorte que la souris reste le moyen le plus direct d'aller loin dans un "
         "gros listing. Ailleurs dans le programme, les barres restent&nbsp;: une liste qui "
         "continue sous le bord doit le dire."),
        ("p",
         "En mode NORMAL, un <b>clic dans le code y pose le curseur</b>, et la "
         "touche F1 ou le survol expliquent le mot visé. Le listing d'un binaire "
         "s'ouvre <b>immédiatement</b>, quelle que soit sa taille : seules les lignes "
         "à l'écran sont mises en page. Entrer en Insertion redonne l'éditeur "
         "complet — sélection à la souris, presse-papier, saisie."),
        ("p",
         "Un <b>compteur</b> se tape devant, comme dans vim : "
         "<font face='Courier'>5j</font> descend de cinq lignes, "
         "<font face='Courier'>3w</font> avance de trois mots, "
         "<font face='Courier'>12k</font> remonte de douze. Les chiffres s'accumulent et "
         "le mouvement les consomme ; le compteur en attente s'affiche <b>à côté de la "
         "pastille de mode</b>, dans la barre d'état — un état qui change ce que la "
         "touche suivante va faire doit se voir. Échap l'efface, comme tout le reste. "
         "<font face='Courier'>0</font> seul reste le début de la ligne : il ne devient un "
         "chiffre qu'une fois qu'un autre l'a ouvert (<font face='Courier'>10j</font>). "
         "Il s'écrit <font face='Courier'>×12</font> et non <font face='Courier'>12</font> "
         "tout court&nbsp;: le nombre seul, à côté de « NORMAL », se lisait comme un "
         "numéro de ligne, et l'on ne savait pas quoi en faire — le survol dit le reste."),
        ("p",
         "<b>Le mode VISUEL</b> — <font face='Courier'>v</font> depuis le mode NORMAL. "
         "Les mouvements y <b>étendent une sélection</b> au lieu de porter seulement le "
         "curseur&nbsp;: <font face='Courier'>v j j</font> prend trois lignes, "
         "<font face='Courier'>v 5 l</font> cinq caractères. La sélection est peinte dans "
         "le listing, et la pastille passe à <b>VISUAL</b>, en couleur d'avertissement — "
         "ce n'est pas un état neutre, la frappe suivante peut couper. "
         "<font face='Courier'>y</font> copie, <font face='Courier'>d</font> ou "
         "<font face='Courier'>x</font> coupe, <b>Échap</b> renonce sans rien toucher. "
         "Comme dans vim, le Visuel est <b>inclusif</b>&nbsp;: le caractère sous le "
         "curseur fait partie de la sélection, et <font face='Courier'>v y</font> copie "
         "une lettre plutôt que rien. "
         "Pendant tout le temps du Visuel, les lettres appartiennent à la sélection&nbsp;: "
         "<font face='Courier'>y</font> y copie, et n'arme pas la séquence "
         "<font face='Courier'>y y</font> qui duplique une ligne en mode NORMAL."),
        ("liste", [
         "<b>Surligner les occurrences</b> — le mot sous le curseur, ou la sélection, est "
         "montré partout où il revient. Il cherche des jetons et non du texte : le "
         "<font face='Courier'>rax</font> d'un commentaire n'est pas surligné, et la casse "
         "suit nasm — <font face='Courier'>MOV</font> est <font face='Courier'>mov</font>, "
         "mais <font face='Courier'>Boucle</font> n'est pas "
         "<font face='Courier'>boucle</font>.",
         "<b>Aller d'une occurrence à l'autre</b> — chacune arrive sélectionnée, donc prête "
         "à être remplacée. On tape, on repasse à la suivante, on retape.",
         "<b>Renommer les occurrences</b> — toutes d'un coup. À la différence d'un "
         "chercher-remplacer, les commentaires ne sont pas touchés.",
         "<b>Dupliquer la ligne</b> — quatre lignes qui ne diffèrent que par un registre se "
         "dupliquent plus vite qu'elles ne se retapent.",
         "<b>Fermeture automatique des paires</b> — et trois règles pour qu'elle ne se "
         "fasse pas détester : pas devant un mot, pas d'apostrophe fermée après une lettre "
         "(« l'adresse »), et taper le fermant déjà là le traverse.",
         "<b>Coller</b> (menu Édition) colle à l’endroit du curseur, en remplaçant "
         "la sélection s’il y en a une. Pour reprendre un programme entier d’un "
         "cours, l’entrée s’appelle <b>Remplacer tout le code</b> — et elle le "
         "dit, ce qui n’était pas le cas quand les deux gestes partageaient le mot "
         "« Coller ».",
        ]),
        ("p",
         "La <b>coloration</b> distingue ce qui existe de ce qui n'existe pas : "
         "<font face='Courier'>mav</font> ne reçoit pas la couleur de "
         "<font face='Courier'>mov</font>. La liste vient du décodeur que namsdec emploie "
         "déjà, donc ce qui se décode se colore. Les étiquettes du fichier sont peintes "
         "comme telles là où l'on y saute, et pas seulement où on les définit."),
        ("p",
         "L'<b>autocomplétion</b> propose selon l'endroit : une instruction en tête de "
         "ligne, un registre ou une étiquette dans une opérande. Tab accepte, les flèches "
         "choisissent, Échap referme."),

        ("h2", "Les corrections proposées"),
        ("p",
         "nasm répond « operation size not specified » et s'arrête. Celui qui apprend "
         "relit sa ligne sans savoir ce qu'on lui reproche ; celui qui sait perd trente "
         "secondes à réécrire ce qu'il savait déjà. namsdec regarde la ligne du curseur, "
         "dit en français ce qui cloche, et montre la ligne réécrite dans la barre d'état. "
         "<b>Alt</b> la pose — la même touche qui accepte une proposition de complétion, "
         "laquelle garde la priorité quand les deux s'offrent ensemble."),
        ("p",
         "Six règles, et six seulement : une correction dont on ne saurait pas dire "
         "pourquoi elle est juste apprendrait le faux, ce qui est pire que de se taire."),
        ("table", [
            ["Ce qui est écrit", "Ce qui est proposé", "Pourquoi"],
            ["mov rax, [rbp-8", "mov rax, [rbp-8]", "un crochet reste ouvert"],
            ["mvo rax, 1", "mov rax, 1", "deux lettres sous les doigts"],
            ["int 0x80", "syscall", "l'appel système 32 bits en code 64 bits"],
            ["jmp bouce", "jmp boucle", "aucune étiquette de ce nom"],
            ["mov rax, ebx", "mov rax, rbx", "deux tailles différentes"],
            ["mov [n], 1", "mov qword [n], 1", "la taille de l'écriture n'est pas dite"],
        ]),
        ("note",
         "Rien ne s'applique tout seul, et l'ordre des règles n'est pas arbitraire : on ne "
         "juge pas les opérandes d'une instruction dont le nom n'existe pas. La détection "
         "du mnémonique inconnu est d'ailleurs <b>celle de la coloration</b> — ce que "
         "l'éditeur peint en rouge est exactement ce qu'il propose de corriger, et les "
         "deux ne peuvent pas diverger."),
        ("p",
         "<font face='Courier'>movzx</font> et <font face='Courier'>movsx</font> ne sont "
         "jamais signalées pour leurs tailles : elles existent précisément pour aller "
         "d'une taille à une autre. Et la casse compte pour une étiquette — "
         "<font face='Courier'>Boucle</font> n'est pas "
         "<font face='Courier'>boucle</font> —, parce qu'elle compte pour nasm."),

        ("h2", "Le panneau de droite"),
        ("p",
         "Il se lit de haut en bas, et se partage en <b>deux zones</b> : le bandeau, "
         "qui dit ce qui vient, et la <b>vue choisie</b>, qui prend tout le reste. "
         "Entre les deux, la <b>rangée des onglets</b> : elle dit ce qu'on regarde et "
         "ce qu'on pourrait regarder, et elle se lit comme celle des vues de "
         "l'établi — une étiquette plate, soulignée quand c'est elle qu'on lit."),
        ("liste", [
         "<b>Prochaine instruction</b> — l’adresse et le pseudo-code "
         "(<font face='Courier'>RAX = RAX + RCX</font> plutôt que "
         "<font face='Courier'>add rax, rcx</font>). Sous le repli <b>Détail</b> : les "
         "octets découpés par champs, la fiche de l’instruction, et l’adresse "
         "effective qu’un calcul désigne avec l’octet qui s’y trouve. Deux lignes "
         "dépliées et le reste à un clic : c’est ce qu’on lit à chaque pas, et "
         "le bandeau doit tenir sans rien couper. Le repli s’ouvre de lui-même devant "
         "un appel, un saut conditionnel ou un appel système. Devant un <b>binaire "
         "ouvert</b>, la section entière part repliée : une instruction y porte deux "
         "lignes de plus, et ce qu’on lit d’abord est le listing et les registres. "
         "Un clic la rouvre. Tant qu'aucun programme n'est assemblé, elle annonce ce "
         "qu'elle montrera, comme les deux zones qui la suivent — une section ouverte "
         "ne reste jamais muette.",
         "Les registres se rangent sur <b>autant de colonnes que la largeur en "
         "permet</b>, et sur <b>une de plus quand le panneau est court</b> : ce qui "
         "bloque une colonne n'est pas la largeur des valeurs — seize chiffres, c'est "
         "irréductible — mais le blanc qui sépare deux colonnes, posé pour que "
         "« …003c RBX » ne se lise pas comme un seul mot. Sur un panneau court, ce "
         "confort coûte une colonne, c'est-à-dire deux rangées : on le rend, et les "
         "registres tiennent sans défiler. On ne resserre que là où cela gagne "
         "vraiment une colonne. Sur une fenêtre minimale, la largeur n'en permet que "
         "deux quoi qu'on fasse, et la vue défile alors comme les autres.",
         "<b>Processeur</b> (la première vue, et celle qu'on trouve en ouvrant) — "
         "seize registres, RIP, le nombre d'instructions "
         "exécutées (les sept indicateurs, eux, sont dans la barre de titre). Un clic sur "
         "un nom de registre pose une surveillance ; un clic sur une valeur permet de la "
         "remplacer. <b>Un registre qui change à un pas s'éclaire d'un fond orange</b> quelques secondes, qui "
         "s'estompe — un bloc de couleur, bien plus visible qu'une teinte de texte : une "
         "valeur qui bascule sans bouger à l'écran se raterait, et l'éclat la désigne sans "
         "encombrer la lecture d'après.",
         "<b>Les autres vues</b> — Pile, Mémoire, Tas, Octets, et RE si le module est "
         "activé. Devant un <b>binaire ouvert</b>, la vue Octets se récrit : un clic sur "
         "les octets d'une instruction ouvre un champ, Entrée valide, Échap annule. On y "
         "met <b>des octets</b> — « 48 31 c0 » — <b>ou l'instruction elle-même</b> — "
         "« xor rax, rax » : namsdec l'assemble avec le même nasm que le reste, donc avec "
         "la même syntaxe et les mêmes messages d'erreur. Ce qui peut se lire en octets se "
         "lit en octets, et la règle est dans l'infobulle. Le "
         "nombre d'octets doit rester le même — en récrire plus ou moins déplacerait "
         "tout ce qui suit, donc toutes les adresses et tous les sauts ; pour cela, "
         "Outils > Patcher le binaire propose « Neutraliser (nop) ». Ce qui a changé "
         "porte la couleur des changements, et la vue Correctifs sait le défaire. "
         "Les onglets sont <b>en tête du panneau</b>, juste au-dessus de ce qu'ils "
         "commandent, et l'onglet souligné dit ce qu'on regarde — il n'y a donc pas de "
         "titre sous eux qui le répéterait. Ils ont vécu dans la barre d'outils "
         "jusqu'au 26 octobre 2026 : une étiquette séparée de son contenu ne se lit "
         "plus comme son étiquette, et six onglets ne tenaient pas dans la barre en "
         "dessous de mille pixels de large, où ils la faisaient se dédoubler pour "
         "toute la séance.",
         "<b>Le processeur est une vue, et non plus un bandeau fixe</b> — depuis le "
         "13 octobre 2026, et c'est un retour en arrière assumé. Les registres "
         "occupaient une zone ancrée au milieu du panneau, au nom d'un principe "
         "juste : ce qu'on surveille entre deux pas ne doit pas se chercher. Le prix "
         "en était un panneau partagé en trois zones qui se disputaient la hauteur — "
         "sur un écran au zoom généreux, les registres prenaient près des trois "
         "cinquièmes du panneau et écrasaient la vue d'à côté. Une zone à la fois, et "
         "chacune a toute la place ; le passage de l'une à l'autre tient dans la "
         "rangée d'onglets, en haut à droite.",
         "La poignée entre le bandeau et la vue se tire pour donner plus de place à "
         "l'un ou à l'autre, et le bandeau ne garde que ce qu'il demande. Quand tout "
         "ne tient pas — une console ouverte, une fenêtre courte, un zoom généreux —, "
         "c'est la vue qui défile. Rien ne se peint par-dessus rien : un contrôle le "
         "vérifie sur les six vues, panneau à l'étroit.",
        ]),
        ("p",
         "<b>Survoler un registre</b> répond à « d'où vient ce qu'il contient ? », et pas "
         "seulement d'un pas : le pas qui a écrit RAX lisait RCX, et la chaîne remonte "
         "jusqu'à quatre maillons, chacun avec son rang et sa ligne. Elle s'appuie sur "
         "l'<b>historique</b> et non sur le code — celui-ci dit ce que le programme "
         "<i>pourrait</i> avoir fait, celui-là ce qu'il a fait, et devant plusieurs chemins "
         "qui mènent au même endroit, seul l'historique sait lequel a été pris."),
        ("p",
         "Quand l'historique est vide — un binaire qu'on ne peut pas exécuter n'en a "
         "pas —, namsdec <b>remonte le code à l'envers</b>, et il <b>traverse la "
         "mémoire</b> : une valeur rangée sur la pile puis relue se suit jusqu'à elle. "
         "C'est le chemin ordinaire de tout argument dans du code compilé sans "
         "optimisation — <font face='Courier'>mov [rbp-8], rdi</font> puis, trente lignes "
         "plus bas, <font face='Courier'>mov rax, [rbp-8]</font>. Le maillon qui remplit "
         "une case est marqué d'un point."),
        ("p",
         "Deux questions sont suivies, et non une. <b>Quelle</b> case ? Elle dépend des "
         "registres de l'adresse : <font face='Courier'>[rsi+8]</font> avec RSI reçu en "
         "argument veut dire « le champ +8 de l'objet qu'on m'a passé ». <b>Quoi</b> "
         "dedans ? Ce que la dernière écriture y a mis."),
        ("note",
         "La piste <b>se coupe</b> devant une écriture vers une case qu'on ne sait pas "
         "distinguer de la nôtre. <font face='Courier'>mov [rax], rcx</font> vise "
         "peut-être <font face='Courier'>[rbp-8]</font> si RAX vaut RBP-8, et rien dans le "
         "code ne le dit ; traverser quand même donnerait une provenance qui se lit bien "
         "et qui est fausse. Une écriture voisine dont on <i>prouve</i> qu'elle ne touche "
         "pas la nôtre — <font face='Courier'>[rbp-16]</font> quand on suit "
         "<font face='Courier'>[rbp-8]</font> — ne coupe rien."),
        ("p",
         "La séparation entre le bandeau et la vue se règle à la poignée, et Affichage › "
         "« Panneau latéral partagé tout seul » rend le partage automatique."),

        ("h2", "La console"),
        ("p", "En bas, trois onglets :"),
        ("liste", [
         "<b>Console</b> — ce que le programme écrit lui-même, par "
         "<font face='Courier'>write</font>. C'est aussi là qu'on lui parle.",
         "<b>Diagnostics</b> — ce que nasm a eu à dire, les avertissements que namsdec "
         "ajoute, et les deux commandes réellement lancées, prêtes à copier.",
         "<b>Profil</b> — les lignes classées par temps passé dedans : passages, part du "
         "total, et un clic qui y emmène. <b>Le numéro de ligne se clique</b>, comme la "
         "ligne elle-même&nbsp;: seul le texte y menait, tout à droite de la rangée et "
         "tronqué — on cliquait « 60&nbsp;749 », qui est pourtant la réponse à « où "
         "ça&nbsp;? », et le classement passait pour mort.",
        ]),

        ("p",
         "La console lit cette sortie <b>comme un terminal la lirait</b>. Un programme "
         "ne lui envoie pas que des lettres : il efface l'écran, remonte le curseur, "
         "passe en cyan, revient au normal — un crackme sur deux ouvre ainsi, par un "
         "logo en couleur. Montrées telles quelles, ces séquences se lisaient "
         "« <font face='Courier'>[2J[H[36m</font> » en tête du dessin, qui paraissait "
         "abîmé. namsdec les <b>interprète</b> : les couleurs se voient, le retour "
         "chariot recouvre sa ligne — c'est ainsi qu'une barre de progression s'anime —, "
         "l'effacement efface, et ce qui n'est pas reconnu disparaît au lieu de "
         "s'afficher."),
        ("note",
         "Les huit couleurs d'un terminal ne sont pas des valeurs, ce sont des "
         "<b>rôles</b> : chaque terminal les peint à sa façon. namsdec les prend dans "
         "son thème — le rouge est celui de ses erreurs, le vert celui de ses "
         "réussites —, ce qui fait qu'un logo s'accorde au reste de la fenêtre au lieu "
         "de jurer avec. Ce n'est pas un terminal complet et ne cherche pas à l'être : "
         "il n'y a ni hauteur d'écran fixe, ni curseur à remonter dans ce qui est déjà "
         "écrit."),
        ("p",
         "Quand le programme se met \u00e0 attendre, une <b>bo\u00eete de saisie</b> s'ouvre "
         "d'elle-m\u00eame, au premier plan, le curseur d\u00e9j\u00e0 dans le champ. Elle montre "
         "<b>ce que le programme vient d'\u00e9crire</b> — neuf fois sur dix c'est "
         "l'invite, et c'est la seule chose qui dise ce qu'on attend de nous. Entr\u00e9e "
         "envoie. Elle <b>reste ouverte</b> ensuite, et montre ce que le programme a "
         "r\u00e9pondu — « Incorrect password » — avant de s'arr\u00eater : la refermer "
         "aussit\u00f4t renverrait devant une console souvent repli\u00e9e, o\u00f9 l'on ne verrait "
         "rien de ce qu'on vient de provoquer. \u00c9chap ou OK la ferment. Elle sert "
         "pr\u00e9cis\u00e9ment quand la console est repli\u00e9e ou couverte par une autre "
         "fen\u00eatre : on reste sinon devant un programme qui ne r\u00e9pond plus, sans "
         "savoir qu'il attend <i>nous</i>."),
        ("p",
         "Elle montre ce que le programme a \u00e9crit <b>en entier</b>, et sans "
         "ascenseur : la bo\u00eete prend la largeur de la plus longue ligne et la "
         "hauteur de toutes. Un logo de sept lignes sur quatre-vingts colonnes — ce "
         "par quoi commence la moiti\u00e9 des crackmes — y tient donc entier. Quand "
         "la sortie d\u00e9passe l'\u00e9cran, c'est <b>la fin</b> qui est montr\u00e9e, "
         "puisque l'invite est toujours en bas, et une ligne dit ce qui reste au-dessus, "
         "dans la console."),
        ("note",
         "Un ascenseur dans une bo\u00eete qui s'ouvre toute seule est un pi\u00e8ge : on y "
         "lit quatre lignes d'un logo qui en fait sept, on croit que le programme n'a "
         "\u00e9crit que cela, et l'on cherche la suite dans une console qu'on ne "
         "regardait pas."),
        ("p",
         "La ligne <b>Entrée</b> est l\u00e0 d\u00e8s qu'un programme est assembl\u00e9, "
         "qu'il lise ou non. On \u00e9crit son programme, on a pos\u00e9 ses cha\u00eenes, "
         "et l'on veut pr\u00e9parer ce qu'il lira avant d'avoir tap\u00e9 le "
         "<font face='Courier'>read</font> : namsdec le dit alors en une phrase plut\u00f4t "
         "que de faire dispara\u00eetre le champ. Entr\u00e9e envoie, \u00c9chap rend le "
         "clavier \u00e0 l'\u00e9diteur, et « Fin d'entr\u00e9e » ferme l'entr\u00e9e pour "
         "un programme qui lit jusqu'au bout."),
        ("h1", "Exécuter, et revenir en arrière"),
        ("h2", "Avancer"),
        ("table", [
            ["Geste", "Ce qu'il fait"],
            ["Pas à pas détaillé", "une instruction ; entre dans les appels"],
            ["Pas à pas principal", "une instruction ; passe par-dessus un `call`"],
            ["Pas en arrière", "défait le dernier pas — registres, indicateurs, mémoire"],
            ["Exécuter", "jusqu'à un arrêt, un point d'arrêt ou la fin"],
            ["Jusqu'au curseur", "pose un point d'arrêt pour un seul voyage"],
            ["Jusqu'au prochain appel", "l'équivalent de `nextcall` de pwndbg"],
            ["Jusqu'au prochain retour", "`nextret`"],
            ["Jusqu'au prochain appel système", "`nextsyscall`"],
        ]),
        ("p",
         "Le <b>pas en arrière</b> est ce qu'un débogueur ordinaire rend laborieux et qu'un "
         "interpréteur offre : namsdec garde l'histoire de l'exécution, et la chronologie "
         "(la vue Chronologie) permet de cliquer un pas passé pour y revenir."),
        ("p",
         "Cette fenêtre montre deux choses distinctes. La <b>liste</b> répond à « qu'est-ce "
         "qui s'est passé » : un pas par ligne, avec sa nature, son adresse et sa ligne. La "
         "<b>barre</b>, en haut, répond à la question qui vient avant — « où j'en suis » : "
         "une colonne par tranche de temps, colorée par ce qui s'y est passé de plus "
         "notable. On clique ou l'on glisse dessus pour rembobiner, comme sur la piste d'un "
         "lecteur."),
        ("note",
         "La couleur de chaque colonne est celle du pas le plus notable de sa tranche, non "
         "du premier venu : un appel système passe devant un appel, qui passe devant une "
         "écriture, qui passe devant un saut. C'est ce classement qui fait qu'un "
         "<font face='Courier'>call</font> seul au milieu de deux mille instructions "
         "ordinaires reste visible — et c'est justement le rare qu'on cherche des yeux."),

        ("h2", "Le calcul vectoriel, et pourquoi il vous tombe dessus"),
        ("p",
         "On peut n'avoir jamais écrit une ligne de SSE et s'arrêter dessus au premier pas "
         "d'un vrai binaire. La raison tient en une instruction : "
         "<font face='Courier'>pminub</font>. La bibliothèque C ne cherche pas la fin d'une "
         "chaîne octet par octet — elle en compare <b>seize d'un coup</b> au vecteur nul, et "
         "ce minimum-là ouvre pratiquement tout programme compilé."),
        ("p",
         "namsdec exécute donc le calcul vectoriel entier au complet : minimum et maximum "
         "dans les deux signes et les trois largeurs, l'arithmétique qui <b>sature</b> au "
         "lieu de boucler (<font face='Courier'>paddusb</font> : 250 + 10 font 255, pas 4), "
         "le choix d'octets par table (<font face='Courier'>pshufb</font>), la fenêtre à "
         "cheval sur deux registres (<font face='Courier'>palignr</font>), l'emballage et "
         "l'élargissement, les produits et leurs deux moitiés, la moyenne, l'écart absolu, "
         "et <font face='Courier'>ptest</font> — la seule de la famille qui écrive des "
         "indicateurs. Les versions AVX suivent la même route : seul l'encodage change."),
        ("p",
         "Et les <b>instructions de chaîne</b> de SSE4.2 — "
         "<font face='Courier'>pcmpistri</font> et ses trois sœurs —, qui font en une "
         "instruction ce qu'une boucle fait en vingt : comparer seize octets à seize "
         "autres selon l'une de quatre questions, et dire où. C'est ainsi que la "
         "bibliothèque C écrit <font face='Courier'>strlen</font>, "
         "<font face='Courier'>strchr</font>, <font face='Courier'>strcmp</font> et "
         "<font face='Courier'>strstr</font>. Tout est dans leur immédiat — la question, "
         "la polarité de la réponse, la largeur des éléments, la forme de la sortie —, "
         "ce qui en fait l'instruction la plus paramétrée du jeu ; la fiche <b>F1</b> "
         "déplie les quatre questions."),
        ("note",
         "Une fiche <b>F1</b> existe pour chacune, et un contrôle empêche le manuel de dire "
         "qu'une instruction n'est pas simulée alors qu'elle s'exécute : la documentation "
         "d'un outil qui apprend ne peut pas être en retard sur l'outil."),

        ("h2", "Les points d'arrêt"),
        ("p",
         "Un clic dans la gouttière pose un point d'arrêt. Il peut porter une "
         "<b>condition</b>, auquel cas l'exécution ne s'arrête que si elle est vraie. "
         "Dans une boucle de cinq cents tours, s'arrêter à chaque passage n'est pas "
         "déboguer, c'est une punition : la condition fait le tri. La vue des points "
         "d'arrêt les liste tous, avec leur condition et le nombre de fois qu'ils ont "
         "arrêté le programme."),
        ("p",
         "Les deux côtés de la comparaison sont des <b>expressions de la "
         "calculatrice</b> : une grammaire de plus à apprendre serait une grammaire de "
         "trop."),
        ("table", [
            ["Condition", "Ce qu'elle attend"],
            ["rcx == 0", "le dernier tour de la boucle"],
            ["rax > 0x100", "un résultat qui dépasse"],
            ["(rax &amp; 0xff) == 'A'", "l'octet de bas vaut la lettre A"],
            ["[rsp+8] != 0", "ce qui est sur la pile n'est plus nul"],
            ["byte [rdi] == 'A'", "le premier caractère pointé par RDI"],
        ]),
        ("note",
         "Une condition s'écrit <b>avant</b> de lancer — c'est même le seul moment où "
         "l'on en écrit une. Le champ ne refuse donc que ce qui est mal écrit, et non ce "
         "qu'il ne sait pas encore calculer : à ce moment-là, il n'y a ni registre ni "
         "mémoire à lire."),
        ("p",
         "Une <b>surveillance</b> est l'autre façon de s'arrêter : sur un registre (« quand "
         "RAX change ») ou sur une adresse (« quand ces huit octets changent »). C'est la "
         "réponse à « qui a écrit ça ? », que relire son programme à l'envers ne donne pas."),

        ("h1", "Lire la mémoire"),
        ("h2", "Les quatre lectures"),
        ("p",
         "L'onglet Mémoire montre les mêmes octets de quatre façons, et choisir la lecture, "
         "c'est déjà savoir ce qu'on cherche :"),
        ("liste", [
         "<b>Octets</b> — le vidage hexadécimal, seize par ligne, avec la transcription. "
         "Ce qui vient de changer est mis en évidence.",
         "<b>Mots</b> — des blocs de huit octets, comme la pile les range.",
         "<b>Texte</b> — la chaîne qui commence là, comme `write` l'enverrait. Si ces "
         "octets ne sont pas du texte, namsdec le dit et liste les chaînes lisibles qu'il "
         "trouve alentour.",
         "<b>Pointeurs</b> — chaque mot suivi de pointeur en pointeur jusqu'au bout : "
         "c'est le `telescope` de pwndbg.",
        ]),
        ("note",
         "Dans ces quatre lectures, <b>les lignes d'octets s'étendent et les phrases se "
         "replient</b>. Tout le bloc s'étendait, ce qui est juste pour seize octets et leur "
         "transcription — qui ne doivent pas se replier à la largeur d'un panneau — et faux "
         "pour les explications qui les accompagnent&nbsp;: en lecture « Texte », deux "
         "phrases sur trois partaient hors de l'écran, « 1 octet, puis un zéro&nbsp;: c'est "
         "là que <font face='Courier'>write</font> s… ». La règle est la plus simple qui "
         "tienne&nbsp;: une ligne qui commence par une adresse EST du vidage, le reste est "
         "de la prose."),
        ("note",
         "Et <b>le bandeau ne garde plus de place qu'il n'emploie pas</b>. La hauteur de "
         "l'onglet, retenue d'un geste de poignée et enregistrée dans les réglages, servait "
         "de <i>plafond</i>&nbsp;: sur un grand écran, « Prochaine instruction » replié "
         "demandait une cinquantaine de pixels, l'onglet en recevait trois cents, et les "
         "deux mille du milieu étaient du vide — l'onglet Mémoire, qui est le plus dense de "
         "tous, s'y retrouvait illisible. La hauteur retenue est désormais un "
         "<i>plancher</i>&nbsp;: on agrandit l'onglet au détriment du bandeau, qui défile, "
         "mais on ne le rétrécit plus pour laisser du vide."),
        ("p",
         "L'adresse s'écrit en hexadécimal, en décimal, ou par un nom de registre : taper "
         "<font face='Courier'>rsi</font> et valider suit le pointeur. « Suivre » fixe la "
         "vue sur RSP ou RIP pas à pas."),
        ("h2", "Chercher dans la mémoire"),
        ("p",
         "C'est le premier réflexe devant un programme qui compare quelque chose. On "
         "vient de taper un mot de passe et l'on veut savoir dans quel tampon il a "
         "atterri ; on a lu une chaîne dans le module <b>Chaînes lisibles</b> et l'on cherche "
         "l'endroit où elle est écrite ; on a vu passer quatre octets dans un registre "
         "et l'on veut savoir s'ils sont aussi ailleurs."),
        ("p",
         "Le champ <b>Chercher</b> de l'onglet Mémoire accepte trois écritures, qui se "
         "mélangent dans la même case :"),
        ("table", [
            ["Écriture", "Ce qu'elle cherche"],
            ['"pass"', "les quatre octets du mot, tels qu'ils sont écrits"],
            ["41 42 43", "trois octets donnés en hexadécimal"],
            ["414243", "les mêmes, sans les blancs"],
            ["48 89 ?? 45", "un octet libre au milieu"],
            ['"pass" 00', "le mot suivi de son zéro final"],
        ]),
        ("p",
         "Le panneau annonce combien d'octets le motif mesure — c'est la première chose "
         "qu'on vérifie après l'avoir tapé —, puis les adresses trouvées, "
         "cliquables : le vidage s'y rend. La liste s'arrête à soixante-quatre et le "
         "dit ; un motif d'un seul octet dans un mégaoctet trouve des milliers de "
         "réponses, et une liste de milliers de lignes n'apprend rien."),

        ("h2", "Les outils de mémoire"),
        ("liste", [
         "<b>Carte de la mémoire</b> — les zones et leurs droits, à la façon de `vmmap`.",
         "<b>Chercher en mémoire</b> — un texte, des octets ou une valeur.",
         "<b>Écrire ici</b> — poser une valeur à la main, pour essayer une hypothèse.",
        ]),
        ("p",
         "Les trois sont sous un seul repli, et <b>le vidage garde sa place sous eux</b> : "
         "quoi qu'on déplie, il en reste de quoi lire — c'est la boîte à outils qui défile, "
         "pas la vue qui s'efface. Dépliée sans cette règle, elle prenait toute la hauteur "
         "du panneau et le vidage n'était plus peint du tout : on ouvrait la carte et l'on "
         "perdait ce qu'on était venu voir, sans qu'une barre de défilement ne le dise."),
        ("h2", "La pile et le tas"),
        ("p",
         "L'onglet Pile lit la pile <i>comme une pile</i> : un mot de huit octets par ligne, "
         "RSP et RBP repérés, les adresses de retour nommées avec leur étiquette, et ce qui "
         "est au-dessus de RSP grisé — ces octets sont encore là, mais le prochain "
         "<font face='Courier'>push</font> les écrase. La chaîne des appels, la convention "
         "System V et les entorses relevées suivent en dessous."),
        ("p",
         "L'onglet Tas montre ce que <font face='Courier'>brk</font> a donné au programme et "
         "ce qu'il en reste. Un débogueur ordinaire ne peut pas le montrer : il ne connaît "
         "pas la frontière. L'interpréteur, lui, la tient."),

        ("h1", "Comprendre ce qu'on lit"),
        ("h2", "Le manuel"),
        ("p",
         "<b>F1</b> ouvre la fiche de l'instruction sous le curseur : ce qu'elle fait, les "
         "indicateurs qu'elle écrit, un exemple qui s'assemble, et les renvois vers les "
         "fiches voisines. Le manuel compte <b>%d fiches</b> — instructions, directives et "
         "notions — et s'exporte en entier en Markdown (menu Aide), parce que ce qu'on "
         "apprend ici doit pouvoir se lire ailleurs." % nb_fiches),
        ("p",
         "Sans rien sous le curseur, la fenêtre d'aide ouvre son <b>sommaire</b>, rangé en "
         "quatre paliers (notions, instructions, directives, préprocesseur). Le sommaire "
         "des instructions liste <b>tous les mnémoniques</b>, et non une tête de famille par "
         "fiche : <font face='Courier'>jmp</font> s'y voyait, mais "
         "<font face='Courier'>je</font>, <font face='Courier'>jne</font>, "
         "<font face='Courier'>jz</font>… restaient invisibles tant qu'on ne savait pas les "
         "chercher. Chaque nom est maintenant présent et cliquable — une famille en couvre "
         "jusqu'à six —, et un compte en tête dit combien ils sont."),
        ("p",
         "Une garantie que peu d'outils donnent : un test parcourt soixante binaires de "
         "votre système et vérifie qu'il n'y reste <b>pas une seule instruction</b> que le "
         "manuel ne sache expliquer."),
        ("h2", "Le déroulement du programme"),
        ("p",
         "<b>Espace puis v puis g</b> ouvre le déroulement : le graphe des blocs. Un listing se lit "
         "de haut en bas ; un programme, non. Chaque boîte est une suite d'instructions "
         "qu'on ne peut qu'entrer par le haut et quitter par le bas ; les flèches disent le "
         "saut pris (vert), le saut non pris (rouge) et l'appel (bleu) ; une boucle se voit "
         "à ce qu'elle remonte. Le bloc où le programme se trouve est mis en avant, un clic "
         "y emmène dans l'éditeur, et Ctrl+molette recule pour voir la forme d'ensemble. "
         "<b>Tout voir</b> cadre le graphe entier, <b>Taille réelle</b> revient au "
         "grossissement d'origine."),
        ("p",
         "Chaque boîte porte, quand namsdec sait le dire, <b>une phrase qui résume ce "
         "que le bloc fait</b> : « demande au noyau : write », « boucle, a tourné 20 "
         "fois », « compare RAX et RBX, et part si c'est différent », « ne fait rien ». "
         "Rien n'est deviné — le numéro d'un appel système n'est nommé que s'il est posé "
         "dans le bloc même, et une comparaison n'est dite que si le saut la suit "
         "immédiatement. <b>Un bloc sans phrase n'est pas un échec</b> : c'est un bloc "
         "dont on ne sait rien dire de plus que ce que ses instructions disent déjà, et "
         "une explication fausse serait pire qu'une absence d'explication."),
        ("p",
         "Un saut dont la cible se calcule — table de sauts, pointeur de fonction — est "
         "signalé comme tel : aucune flèche n'est inventée vers une adresse inconnue."),
        ("h2", "Ce que fait la fonction entière"),
        ("p",
         "Quand <b>Fonction courante</b> est coché, une ligne en tête dit ce que namsdec "
         "sait de la fonction <i>dans son ensemble</i> : « Lit RDI et RSI, rend 0 ou 1. » "
         "C'est la première question devant une fonction sans symboles — que prend-elle, "
         "que rend-elle — et le dessin n'y répond pas."),
        ("liste", [
            "<b>Les arguments</b> : les registres de la convention System V lus avant "
            "d'être écrits, dans le bloc d'entrée. Un argument consommé plus loin est "
            "manqué ; aucun n'est inventé.",
            "<b>La valeur rendue</b> : dite seulement si <i>tous</i> les retours posent une "
            "constante. Un seul chemin muet suffit à se taire — « rend 0 ou 1 » serait "
            "faux s'il en existe un troisième.",
            "<b>Les appels système</b>, quand leur numéro est posé dans le même bloc.",
        ]),
        ("note",
         "La ligne <b>ne répète pas le dessin</b>. « Contient une boucle » devant un graphe "
         "où la boucle se voit n'apprend rien, et une phrase de trop fait cesser de lire "
         "celles qui apprennent quelque chose. Quand namsdec n'a que du banal à dire, il "
         "ne dit rien."),
        ("h2", "Le pseudo-code : les sauts rendus à leur forme"),
        ("p",
         "La case <b>Pseudo-code</b> remplace le dessin par le <b>texte</b> de la même "
         "fonction. Là où le listing dit <font face='Courier'>jne</font>, le pseudo-code "
         "dit « si égal : », et il met en retrait ce qui en dépend ; là où une flèche "
         "remonte, il dit « tant que » ou « répéter … tant que ». C'est le chaînon que "
         "tout le monde franchit de tête après quelques semaines, et qui manque les "
         "premiers jours."),
        ("p",
         "<b>La condition affichée est celle du programme, non celle du saut.</b> Un "
         "compilateur écrit « si NON condition, sauter par-dessus » : recopier le "
         "mnémonique dirait l'inverse de ce que fait le code — et se lirait très bien, ce "
         "qui est le pire des cas."),
        ("p",
         "<b>La comparaison et le saut se disent d'un trait.</b> Le pseudo-code écrivait "
         "deux lignes là où le programme pose une question : « comparer RCX et 0 », puis "
         "« tant que différent ». Différent de quoi ? La réponse était sur la ligne du "
         "dessus, et l'œil faisait l'aller-retour à chaque boucle. Dites ensemble, les "
         "deux lignes n'en font plus qu'une, « tant que RCX n'est pas nul : », qui est "
         "ce qu'on aurait écrit dans le langage d'où ce code vient. La comparaison "
         "disparaît alors du corps : la redire laisserait croire qu'il y a là deux "
         "tests."),
        ("p",
         "<b>Le signe est dit des deux côtés</b> — "
         "<font face='Courier'>RAX &lt; 5 (signé)</font>, "
         "<font face='Courier'>RAX &lt; 5 (non signé)</font>. Sur "
         "<font face='Courier'>0xFFFFFFFF</font>, « plus petit » est vrai en signé (c'est "
         "-1) et faux en non signé (c'est le plus grand des nombres) : taire lequel des "
         "deux, c'est laisser lire le contraire de ce qui se passe."),
        ("note",
         "Une <b>opération ordinaire</b> pose aussi les indicateurs, et prête sa "
         "condition sans disparaître. <font face='Courier'>dec ecx</font> suivi d'un "
         "<font face='Courier'>jnz</font> se lit « tant que ECX n'est pas nul » — c'est la boucle "
         "la plus courante qui soit, et il n'y a aucun <font face='Courier'>cmp</font> à "
         "y lire. Mais le <font face='Courier'>dec</font> décrémente pour de bon : il "
         "reste écrit, sans quoi on lirait une boucle dont rien ne changerait jamais. Et "
         "seulement « nul » ou « non nul » : <font face='Courier'>dec</font> ne touche "
         "pas à la retenue, donc un <font face='Courier'>jb</font> qui le suit lit celle "
         "d'une instruction d'avant, et namsdec s'y refuse plutôt que d'inventer un "
         "test."),
        ("p",
         "Chaque <b>instruction</b> est dite en clair sous la même règle : ce qui ne se "
         "paraphrase pas sans risque reste en syntaxe NASM, qui au moins est exacte. "
         "Trois familles qu'on lisait mal :"),
        ("liste", [
            "<b>Les instructions de chaîne</b> — "
            "<font face='Courier'>rep movsb</font> se lit « copier RCX octets de "
            "mémoire[RSI] vers mémoire[RDI] », c'est-à-dire "
            "<font face='Courier'>memcpy</font> ; sans le préfixe, c'est un seul octet "
            "et l'on avance. Le préfixe n'apparaît pas dans le mnémonique, et c'est "
            "pourtant lui qu'on cherche en lisant une boucle de copie. "
            "<font face='Courier'>rep stosq</font> remplit, "
            "<font face='Courier'>repne scasb</font> cherche : "
            "<font face='Courier'>memset</font> et "
            "<font face='Courier'>strlen</font> déroulés.",
            "<b><font face='Courier'>loop</font></b> fait deux choses d'un coup — "
            "« RCX = RCX - 1 ; si RCX n'est pas nul, aller à … » —, et c'est précisément ce qu'on "
            "oublie en la lisant.",
            "<b><font face='Courier'>imul</font> à trois opérandes</b> — "
            "<font face='Courier'>imul rax, rbx, 4</font> veut dire "
            "« RAX = RBX × 4 » et non « RAX = RAX × RBX ». namsdec disait la seconde : "
            "une phrase fausse, sur l'instruction que tout compilateur produit pour "
            "indexer un tableau.",
        ]),
        ("liste", [
            "<b>si … sinon</b> — deux branches qui se rejoignent plus bas.",
            "<b>tant que</b> — le test est en haut : le corps peut ne jamais tourner.",
            "<b>répéter … tant que</b> — le test est en bas : le corps passe au moins une "
            "fois. La distinction n'est pas cosmétique, c'est souvent la question qu'on "
            "se pose devant un binaire qu'on n'a pas écrit.",
        ]),
        ("p",
         "Ce qui ne se reconnaît pas garde son <b>« aller à »</b> et son étiquette. Un "
         "« si » inventé à la mauvaise place coûterait plus cher que le saut qu'il "
         "remplace : on relit un saut, on ne relit pas ce qu'on croit avoir compris. Les "
         "mots de structure sont d'une autre couleur que les instructions, parce qu'ils "
         "ne viennent d'aucune d'elles — ils viennent du graphe. La ligne où le "
         "processeur se trouve est surlignée, comme dans l'éditeur : un "
         "<font face='Courier'>si</font> qui a absorbé sa comparaison garde l'adresse de "
         "celle-ci, et s'allume donc quand le processeur l'atteint."),
        ("p",
         "Les commandes d'échelle — moins, plus, <b>Taille réelle</b>, <b>Tout voir</b> — "
         "sont <b>grisées</b> dans cette vue : un texte n'a pas d'échelle. Elles disent "
         "pourquoi au survol plutôt que de disparaître, et le bouton <b>Copier le "
         "pseudo-code</b> prend leur relais — le texte s'emporte comme le graphe, "
         "indentation comprise."),
        ("p",
         "Une fonction <b>sans aucun saut</b> le dit en une ligne : son pseudo-code est le "
         "listing, traduit en clair, et il n'y a pas de « si » à y chercher."),
        ("p",
         "Le graphe s’emporte : <b>Copier en DOT</b> le met dans le presse-papier, "
         "<b>Enregistrer le graphe…</b> l’écrit dans un fichier. "
         "<font face='Courier'>dot -Tsvg graphe.dot</font> en fait une image. C’est le "
         "graphe <i>qu’on regarde</i> qui part — la fonction seule ou le programme "
         "entier, selon la case cochée."),
        ("h2", "Le profil"),
        ("p",
         "La bande de chaleur dit <i>où</i> ça chauffe ; le profil dit <i>de combien</i>. "
         "Il classe les lignes par nombre de passages, donne la part du total, nomme le bloc "
         "de chaque ligne, et se copie en texte. Quand un exercice impose un budget "
         "d'instructions, le profil le rappelle et désigne la ligne par où reprendre."),

        ("h2", "La calculatrice"),
        ("p",
         "Son bouton est au bout de la barre d'outils, à droite — là où le besoin "
         "naît, à côté du panneau qui porte les valeurs —, Outils › "
         "Calculatrice l'ouvre aussi, et <b>Espace c a</b> la fait apparaître et "
         "disparaître sans quitter le clavier (Ctrl+Maj+A dans le jeu ordinaire). C'est "
         "le seul raccourci de namsdec qui BASCULE : elle s'ouvre au milieu d'autre "
         "chose, et l'ouvrir sans pouvoir la refermer de la même main obligerait à viser "
         "sa croix, c'est-à-dire à quitter le code des yeux. Le besoin revient tout le temps : on lit "
         "<font face='Courier'>and eax, 0x1f</font> et l'on veut savoir ce que ça garde ; "
         "on voit <font face='Courier'>shr rax, 3</font> et l'on cherche le diviseur ; on "
         "trouve <font face='Courier'>0x6c6c6568</font> dans un registre et l'on se demande "
         "quelles lettres c'est."),
        ("p",
         "Elle est taillée comme une calculatrice de programmeur : l'expression en "
         "haut, le résultat en grand, les <b>soixante-quatre bits</b> dessous — "
         "groupés par quartets, avec leurs index —, puis un clavier. Cliquer un bit "
         "le bascule, et l'expression devient la nouvelle valeur : on veut savoir ce que "
         "vaut <font face='Courier'>0x1f</font> avec le quatrième bit en moins, et "
         "l'écrire à la main est plus long que de le cliquer."),
        ("p",
         "Aucun filet horizontal : ni sous le titre, ni au-dessus du bouton. Une "
         "fenêtre qui en porte deux se lit comme un formulaire — trois bandes "
         "empilées —, et c'est l'espace qui sépare aussi bien. Les sept lectures "
         "du résultat sont rangeées deux par rangée, ce qui les fait tenir en "
         "quatre lignes et permet d'en comparer deux d'un coup d'œil."),
        ("p",
         "L'écran — l'expression, le résultat et les bits — vit dans son cadre, "
         "sur le fond de l'éditeur : c'est ce qui distingue ce que l'appareil "
         "<i>montre</i> de ce sur quoi on <i>appuie</i>. <b>On tape dans l'écran</b>, "
         "sur sa première ligne et en grand ; le résultat s'écrit dessous, plus "
         "grand encore. Les deux se lisent de gauche à droite, comme la ligne de code "
         "d'où l'expression sort — une calculatrice de bureau cale à droite parce "
         "qu'on y entre un nombre chiffre par chiffre, ce qui n'est pas ce qu'on fait ici. Les deux blocs ont la même largeur, au pixel, parce "
         "qu'un appareil dont l'écran et le clavier n'ont pas la même largeur a l'air "
         "monté de deux morceaux."),
        ("p",
         "<b>Effacer tout</b> et le <b>retour arrière</b> sont au bout de la ligne "
         "d'expression, comme la croix d'un champ de recherche — là où est ce qu'ils "
         "effacent. Ils tenaient un bandeau rouge pleine largeur au-dessus du clavier : "
         "la plus grande surface de l'appareil, et la plus criarde, pour ce qu'on fait "
         "quand on s'est trompé. Un appareil dont la plus grande touche est « Effacer "
         "tout » se trompe sur ce qu'on vient y faire. Discrets au repos, ils "
         "rougissent au survol, et disent pourquoi ils sont éteints quand l'expression "
         "est vide."),
        ("p",
         "Les touches sont peintes en couleurs <b>opaques et franches</b> : les chiffres "
         "sur un gris net, les op\u00e9rateurs sur la couleur d'accent du th\u00e8me. "
         "Deux familles, deux teintes — on vise une touche sans la lire. L'encre est celle des deux couleurs du th\u00e8me qui contraste le "
         "plus avec le fond de la touche — le contraste au sens des "
         "recommandations d’accessibilité, mesuré sur la clarté que "
         "l’œil reçoit vraiment, et non sur les nombres que reçoit "
         "l’écran. Les deux ne classent pas pareil, et namsdec s’y était "
         "trompé : les touches d’opérateur portaient l’encre la moins "
         "lisible des deux. La règle vaut pour un thème clair comme pour un "
         "sombre. Elles "
         "s'\u00e9claircissent au survol et s'assombrissent sous le clic, comme une vraie "
         "touche qui descend."),
        ("p",
         "<b>Le clavier physique appartient à l'expression, toujours.</b> Une touche "
         "cliquée, un bit basculé, une fonction choisie : la frappe revient aussitôt à "
         "la ligne où l'on tape, et le curseur au bout de ce qui vient d'être écrit. "
         "<b>Ctrl+C</b>, <b>Ctrl+V</b> et <b>Ctrl+X</b> y marchent donc comme dans "
         "n'importe quel champ — on colle une valeur relevée dans le listing, on emporte "
         "celle qu'on vient de calculer. Sans cette règle, la calculatrice perdait le "
         "presse-papiers dès la première touche cliquée, c'est-à-dire exactement au "
         "moment où l'on s'en sert. Le grand résultat et chacune de ses sept lectures se "
         "copient aussi d'un clic."),
        ("p",
         "Le clavier écrit <b>ce que l'évaluateur lit</b> — "
         "<font face='Courier'>&amp;</font>, <font face='Courier'>|</font>, "
         "<font face='Courier'>^</font>, <font face='Courier'>&lt;&lt;</font> — et non "
         "des symboles mathématiques qu'il faudrait retraduire pour les recopier dans "
         "du code. C'est la même raison qui fait écrire "
         "<font face='Courier'>qword [rsp]</font> plutôt qu'une notation à nous."),
        ("p",
         "<b>Base</b> et <b>Largeur</b> sont deux réglages jumeaux, et se lisent "
         "comme tels : même largeur d’étiquette, même largeur de liste, "
         "les deux flèches au même endroit. Les mesures sont prises sur les "
         "textes les plus longs de chaque colonne, dans la langue affichée — un "
         "chiffre écrit en dur alignerait le français en désalignant "
         "l’anglais. L’étiquette et sa liste se lisent sur la <b>même ligne</b> : "
         "les listes déroulantes de namsdec sont les siennes, et non celles de la "
         "bibliothèque d’interface, dont le cadre se posait trois pixels et demi "
         "trop bas."),
        ("p",
         "La <b>base</b> gouverne les deux sens : le grand résultat est écrit "
         "dedans, et un chiffre tapé au clavier de l'écran y entre avec le "
         "préfixe qui va avec — en hexadécimal, "
         "<font face='Courier'>1</font> puis <font face='Courier'>F</font> donnent "
         "<font face='Courier'>0x1F</font>. Les chiffres qui n'existent pas dans la base "
         "sont éteints : un clavier qui propose <font face='Courier'>9</font> en "
         "binaire ment sur ce qu'on peut taper. La <b>largeur</b> (8, 16, 32, 64) borne "
         "la lecture, non le calcul, qui se fait toujours sur soixante-quatre bits comme "
         "le processeur."),
        ("p",
         "Elle lit l'hexadécimal, le binaire, l'octal, le décimal, le suffixe "
         "<font face='Courier'>h</font> de nasm et les caractères "
         "(<font face='Courier'>'A'</font>), avec les opérateurs de bits à la priorité du "
         "C. Les <b>registres du programme en cours</b> sont des noms comme les autres, à "
         "toutes leurs largeurs — <font face='Courier'>rax</font>, "
         "<font face='Courier'>eax</font>, <font face='Courier'>ax</font>, "
         "<font face='Courier'>ah</font> : <font face='Courier'>rax &amp; 0xff</font> se lit "
         "comme on le pense."),
        ("p",
         "Elle lit aussi la <b>mémoire</b>, entre crochets et avec la grammaire de nasm : "
         "<font face='Courier'>[rsp+8]</font> donne les huit octets à cette adresse, "
         "<font face='Courier'>byte [rdi]</font> un seul, "
         "<font face='Courier'>dword [rbx + rcx*4]</font> les quatre d'une case de tableau. "
         "L'adresse est une expression entière, la même qu'on lit dans le listing "
         "juste au-dessus."),
        ("note",
         "La lecture est un <b>coup d'œil</b>, non une instruction : elle ne déclenche "
         "aucune surveillance, ne compte pas comme un accès du programme, et ne bute pas "
         "sur une page sans droit de lecture. Regarder ne doit pas changer ce qu'on "
         "regarde."),
        ("p",
         "S'y ajoutent <b>%d fonctions</b> : celles qu'on écrit à la main en lisant un "
         "désassemblage, rangées en six familles qu'on ouvre une à une. Elles sont en "
         "boutons, et <b>survoler l'une d'elles écrit ce qu'elle fait</b> juste en "
         "dessous — l'infobulle seule ne suffisait pas, elle se mérite après une "
         "demi-seconde d'immobilité alors qu'on parcourt les noms des yeux. Un clic "
         "écrit l'appel dans l'expression. Le résultat s'affiche en bas dans "
         "<b>toutes les bases en même temps</b>, chaque ligne se copiant d'un clic." %
         nb_fonctions),
        ("liste", [
            "<b>Bits</b> — lire, poser, retirer, retourner un bit ; "
            "<font face='Courier'>bits(x, h, b)</font> extrait un champ, ce qu'on fait à "
            "la main pour décoder un ModR/M ; <font face='Courier'>mask(n)</font> et "
            "<font face='Courier'>byte(x, n)</font>.",
            "<b>Ordre</b> — permuter les octets, renverser les bits "
            "(<font face='Courier'>bitrev8</font> et ses largeurs), tronquer en haut ou "
            "en bas.",
            "<b>Signe</b> — étendre le signe, le complément à deux "
            "(<font face='Courier'>twos</font>, l'opposé), le complément à un, la valeur "
            "absolue.",
            "<b>Compter</b> — bits à 1, zéros de tête et de queue, "
            "<font face='Courier'>parity</font> (celle du x86 : l'octet de BAS, et 1 "
            "quand le compte est PAIR), le premier bit à 1, le log entier, les "
            "puissances de deux, l'alignement.",
            "<b>Tourner</b> — rotations sur 8, 16, 32 ou 64 bits — "
            "<font face='Courier'>rol8(0x80, 1)</font> fait un, "
            "<font face='Courier'>rol(0x80, 1)</font> fait 0x100 — et "
            "<font face='Courier'>sar</font>, le décalage arithmétique, celui qui "
            "recopie le signe là où <font face='Courier'>&gt;&gt;</font> fait entrer des "
            "zéros.",
            "<b>Calculer</b> — le plus petit et le plus grand, signés ou non (c'est la "
            "différence entre <font face='Courier'>jb</font> et "
            "<font face='Courier'>jl</font>), et "
            "<font face='Courier'>mulhi</font> : les 64 bits de haut du produit, ce que "
            "<font face='Courier'>mul</font> laisse dans RDX et qui explique les "
            "divisions par constante d'un compilateur.",
        ]),
        ("note",
         "Une famille à la fois, et celle qu'on consulte est celle qu'on retrouve au "
         "retour. Les %d pastilles ensemble tenaient douze lignes, et douze lignes de "
         "plus poussaient le bas de la fenêtre — les bases, le bouton — hors de "
         "l'écran." % nb_fonctions),
        ("note",
         "La lecture ASCII suit l'ordre de la <b>mémoire</b>, non celui de l'écriture : "
         "<font face='Courier'>0x6c6c6568</font> donne « hell » et non « lleh ». Une valeur "
         "devient une chaîne en passant par la mémoire, c'est donc la mémoire qu'il faut "
         "montrer."),

        ("h1", "Le parcours d'exercices"),
        ("p",
         "Le parcours compte <b>%d étapes</b>, rangées en quatre paliers de difficulté "
         "croissante : Découverte (%d), Pratique (%d), Structures (%d) et Maîtrise (%d). "
         "Chacune n'introduit qu'une chose et suppose les précédentes." %
         (total_exos, paliers.get("discovery", 0), paliers.get("practice", 0),
          paliers.get("structures", 0), paliers.get("mastery", 0))),
        ("p",
         "<b>%d des %d</b> portent un <b>but vérifiable</b> — par exemple "
         "<font face='Courier'>rax == 0xf</font> — que namsdec dit « atteint » ou « pas "
         "encore », au lieu de laisser vérifier à l'œil. Ce qui a été réussi une fois le "
         "reste, d'une session à l'autre." % (avec_but, total_exos)),
        ("p",
         "<b>Les autres sont des démonstrations</b> : des programmes à regarder tourner, "
         "sans rien à y réussir — le tout premier du parcours en est un, qui montre ce "
         "qu'une écriture 32 bits fait à la moitié haute d'un registre. Une "
         "démonstration <b>se termine quand on l'a regardée jusqu'au bout</b>, c'est-à-"
         "dire quand le programme s'arrête, et le panneau le dit au lieu de se taire. "
         "Jusqu'au 27 octobre 2026 il n'en disait rien : on allait jusqu'au "
         "<font face='Courier'>hlt</font>, la pastille restait creuse, « en cours » ne "
         "partait jamais, et rien n'expliquait pourquoi. Un parcours dont la première "
         "étape ne peut pas se finir n'invite pas à faire la deuxième."),
        ("p",
         "Chaque palier porte une <b>barre d'avancement</b> à côté de son compte, et le "
         "parcours entier la sienne en tête de la série. Un chiffre se lit, une barre se "
         "voit : c'est toute la différence quand on hésite à rouvrir une liste de %d "
         "étapes. Pas de flamme, pas de série de jours, pas de récompense — la seule "
         "distinction de namsdec reste l'<b>anneau du budget</b>, qui se gagne en "
         "réécrivant son programme plus court que la solution livrée." % total_exos),
        ("p",
         "Le but et la progression se lisent dans la <b>fenêtre des exercices</b> "
         "(Aide › Exercices), sous le titre de l'étape en cours — et non plus dans le "
         "bandeau du processeur. Ce bandeau est toujours visible : le but y restait "
         "affiché quand on chargeait un binaire sans rapport, comme si l'exercice portait "
         "sur lui. Les exercices ont désormais leur fenêtre, ouverte quand on en fait un, "
         "fermée sinon."),
        ("p",
         "Un but manqu\u00e9 dit <b>laquelle</b> de ses conditions manque, et ce que "
         "l\u2019expression vaut \u00e0 la place — <font face='Courier'>rax == 0xf</font> "
         "suivi de <font face='Courier'>rax vaut 0xc</font>. Un point plein marque ce qui "
         "tient, un cercle vide ce qui manque. \u00ab Pas encore \u00bb tout seul est un "
         "verdict : sur un but \u00e0 plusieurs conditions, on ne savait pas laquelle "
         "regarder, et l\u2019on relan\u00e7ait au hasard. Ce que namsdec sait — la "
         "valeur — il le dit ; comment y arriver, il se tait, parce que c\u2019est le "
         "geste qu\u2019on est venu apprendre."),
        ("p",
         "Il dit aussi <b>ce que le programme a fait</b> : « la boucle de la ligne 6 a "
         "tourné 4 fois », « le saut de la ligne 9 n'a jamais été pris ». La condition "
         "dit ce que RAX vaut, elle ne dit pas pourquoi ; ces phrases-là si, et ce sont "
         "des <b>compteurs</b>, non des déductions. Un saut dont la suite a d'autres "
         "entrées ne dit rien du tout : son compteur mélangerait les passages venus "
         "d'ailleurs, et la phrase serait fausse exactement dans les cas intéressants."),
        ("p",
         "La plupart portent aussi un <b>budget</b> : le nombre d'instructions exécutées "
         "qu'il faut tenir. Il est <i>mesuré</i>, jamais estimé — un test exécute les %d "
         "exercices et vérifie que chacun atteint son but dans son budget. Un programme "
         "juste peut être deux fois plus court, et c'est en cherchant comment qu'on apprend "
         "<font face='Courier'>lea</font>, les décalages, ou qu'une boucle se remplace par "
         "une formule." % total_exos),
        ("note",
         "Le module est éteint par défaut : Préférences › Modules l'allume. Chaque exercice "
         "ouvert prend son propre onglet, sous son titre — on passe au suivant sans "
         "abandonner ce qu'on a écrit dans le précédent."),
        ("p",
         "Le parcours <b>n'est pas dans le binaire</b> : c'est un fichier de données, "
         "<font face='Courier'>~/.local/share/namsdec/exercises/namsdec.json</font>. "
         "Corriger un énoncé ne demande donc pas de recompiler, et une série que vous "
         "écrivez s'installe exactement de la même façon (Outils › Exercices)."),

        ("h2", "L'Atelier : la seconde série"),
        ("p",
         "Une <b>seconde série</b> est livrée à part, dans "
         "<font face='Courier'>plugins/exercices/atelier.json</font> : <b>%d exercices</b> "
         "qui reprennent là où le parcours s'arrête — %d au palier Structures, %d au palier "
         "Maîtrise. Elle s'installe avec le reste (<font face='Courier'>./install.sh</font>) "
         "et apparaît dans la même fenêtre, sous son propre nom." %
         (total_atelier, atelier_paliers.get("structures", 0),
          atelier_paliers.get("mastery", 0))),
        ("p",
         "Le palier <b>Structures</b> y monte ce qu'un langage cache : une matrice et sa "
         "multiplication d'indice, un tri par insertion, une liste chaînée, un cadre de "
         "pile monté à la main, une table de sauts. Le palier <b>Maîtrise</b> sort du "
         "calcul entier : quatre additions en une instruction "
         "(<font face='Courier'>paddd</font>), seize octets comparés d'un coup "
         "(<font face='Courier'>pcmpeqb</font> puis <font face='Courier'>pmovmskb</font>), "
         "un appel système qui écrit vraiment, le débordement signé et le non signé qui "
         "ne répondent pas à la même question, un produit qui ne tient pas dans un "
         "registre, un canari de pile monté à la main."),
        ("note",
         "Ces derniers ne sont pas des curiosités : ce sont les formes qu'on rencontre en "
         "lisant un binaire réel. Un <font face='Courier'>bswap</font> trahit du réseau ou "
         "un format de fichier ; un <font face='Courier'>mov rax, fs:[0x28]</font> en tête "
         "de fonction trahit la protection de pile. Les reconnaître, c'est la moitié du "
         "travail d'analyse."),

        ("h1", "Comment une instruction est faite"),
        ("p",
         "La vue <b>Octets</b> et le bandeau « Prochaine instruction » montrent les mêmes "
         "octets sous deux formes. Ce chapitre explique la seconde : ce qu'un décodeur "
         "lit, dans l'ordre où il le lit. On n'a pas besoin de le savoir pour se servir de "
         "namsdec — on en a besoin pour croire ce qu'il affiche, et pour voir où il "
         "s'arrête."),
        ("p",
         "Une instruction x86-64 n'a pas de longueur fixe : elle va d'un octet "
         "(<font face='Courier'>c3</font>, c'est-à-dire <font face='Courier'>ret</font>) à "
         "quinze. Elle se lit de gauche à droite, et chaque morceau décide de ce qui "
         "suit :"),
        ("table", [
            ["Morceau", "Taille", "Ce qu'il dit"],
            ["Préfixes", "0 à 4 octets", "répétition, verrou, changement de taille"],
            ["REX", "0 ou 1", "on travaille sur 64 bits, ou sur r8 à r15"],
            ["Opcode", "1 à 3", "le verbe : déplacer, ajouter, sauter"],
            ["ModR/M", "0 ou 1", "sur quoi : quels registres, ou quelle adresse"],
            ["SIB", "0 ou 1", "l'adresse quand elle est un calcul"],
            ["Déplacement", "0, 1, 2 ou 4", "la constante ajoutée à l'adresse"],
            ["Immédiat", "0, 1, 2, 4 ou 8", "la valeur écrite dans l'instruction"],
        ]),

        ("h2", "L'octet ModR/M"),
        ("p",
         "C'est le plus instructif des sept, et le plus court à expliquer. Huit bits, "
         "trois champs : <b>mod</b> sur deux bits dit le type d'accès, <b>reg</b> sur "
         "trois bits désigne un registre, <b>r/m</b> sur trois bits désigne un registre ou "
         "une mémoire. L'opcode dit « déplacer » ; il ne dit pas encore d'où vers où, et "
         "c'est ModR/M qui l'ajoute — comme l'adresse écrite sur une enveloppe complète le "
         "verbe « livrer »."),
        ("code",
         "0xE5  =  1110 0101\n"
         "\n"
         "   mod = 11   (3)   deux registres, aucune memoire\n"
         "   reg = 100  (4)\n"
         "   r/m = 101  (5)"),
        ("p",
         "Reste à nommer les numéros, et l'ordre est le premier piège de "
         "x86-64 : il n'est ni alphabétique ni celui qu'on récite. C'est "
         "<font face='Courier'>rax, rcx, rdx, rbx, rsp, rbp, rsi, rdi</font> — donc "
         "le 4 est <font face='Courier'>rsp</font> et le 5 est "
         "<font face='Courier'>rbp</font>."),
        ("p",
         "Avec l'opcode <font face='Courier'>89</font> (« déplacer vers r/m ») "
         "et le préfixe <font face='Courier'>48</font> (« sur 64 bits »), les "
         "trois octets <font face='Courier'>48 89 e5</font> disent donc "
         "<font face='Courier'>mov rbp, rsp</font> : le premier geste de presque toutes "
         "les fonctions compilées. On le reconnaît à l'œil une fois "
         "qu'on l'a décodé une fois."),
        ("note",
         "<font face='Courier'>mod = 3</font> veut dire « pas de mémoire ». Les trois "
         "autres valeurs annoncent une adresse, et c'est là qu'arrivent le SIB et le "
         "déplacement : <font face='Courier'>[rbp-0x18]</font> est un ModR/M qui dit "
         "« mémoire, base rbp, déplacement d'un octet », suivi de l'octet "
         "<font face='Courier'>0xe8</font> — soit -24 en complément à deux."),

        ("h2", "Un registre, quatre tailles"),
        ("p",
         "Le même registre porte quatre noms selon la largeur qu'on en prend. Ce n'est pas "
         "une convenance d'écriture : écrire dans "
         "<font face='Courier'>eax</font> met à zéro les 32 bits hauts de "
         "<font face='Courier'>rax</font>, tandis qu'écrire dans "
         "<font face='Courier'>ax</font> ou <font face='Courier'>al</font> les laisse "
         "intacts. C'est une source d'erreurs constante, et le panneau des registres la "
         "montre en toutes lettres."),
        ("code",
         "RAX  [63 ............................ 0]\n"
         "EAX              [31 ............... 0]\n"
         "AX                        [15 ...... 0]\n"
         "AH                        [15 .. 8]\n"
         "AL                                [7 . 0]"),

        ("h2", "Ce que le décodeur ne peut pas savoir"),
        ("p",
         "Un décodeur linéaire lit les octets dans l'ordre et suppose qu'ils sont tous du "
         "code. C'est faux dès qu'une table de données est posée au milieu d'une fonction, "
         "ou qu'un octet a été glissé pour désaligner la lecture. Le listing repart alors "
         "sur un mauvais pied et invente des instructions qui n'existent pas."),
        ("p",
         "D'où le classement de namsdec : ce qu'il <b>sait</b> — les octets, leur adresse — "
         "et ce qu'il <b>déduit</b> — les fonctions, les chaînes, les figures. Les seconds "
         "sont signalés comme tels, et l'entropie se lit avant tout le reste : sur un "
         "binaire empaqueté, il n'y a rien à décoder tant qu'on n'a pas déballé."),

        ("h1", "Lire un binaire qu'on n'a pas écrit"),
        ("p",
         "Fichier › Ouvrir accepte un ELF64 : ses sections vont à leur adresse, son point "
         "d'entrée est celui qu'il déclare, et seul le code est désassemblé. Le listing "
         "obtenu remplit l'éditeur, une instruction par ligne — ce qui donne aussi la "
         "correspondance ligne/adresse dont dépendent la gouttière, le suivi de RIP et les "
         "points d'arrêt."),
        ("h2", "Trois façons de lire la même instruction"),
        ("p",
         "La vue Désassemblage — l'onglet de la rangée, ou <b>Outils › Désassemblage</b>, "
         "qui l'y emmène — offre un choix de "
         "<b>moteur</b>, en tête, parce que c'est lui qui décide de tout ce qu'on lit en "
         "dessous :"),
        ("table", [
            ["Moteur", "Ce qu'il montre", "Exemple"],
            ["Pseudo-code", "l'instruction dite en français", "RAX = RBX"],
            ["NASM", "la syntaxe qu'on écrit dans l'éditeur", "mov rax, rbx"],
            ["Système", "celle de gdb, objdump et as", "mov %rbx, %rax"],
        ]),
        ("note",
         "La syntaxe <b>Système</b> est une vue, non une écriture : nasm ne la lit pas, "
         "et un listing montré ainsi ne se réassemble pas. Elle est là parce que tout ce "
         "qu'on trouve hors de namsdec — un rapport d'outil, une session gdb, un tutoriel "
         "en ligne — est écrit dans celle-là, et que la traduire de tête est la marche où "
         "l'on abandonne. La source vient d'abord, les registres portent un "
         "<font face='Courier'>%</font>, et la taille se colle au mnémonique "
         "(<font face='Courier'>movq</font>)."),
        ("p",
         "D'autres moteurs s'ajoutent en posant un fichier de description : voir "
         "« Brancher son propre outil »."),
        ("p",
         "Le module <b>RE</b> (Préférences › Modules) ajoute un onglet au panneau "
         "latéral et la vue Analyse. La douzaine de panneaux vit dans la vue&nbsp;: "
         "empilés dans une colonne de trois cents pixels, ils se parcourraient à l'ascenseur."),
        ("p",
         "L'onglet, lui, porte la <b>fiche d'identité</b> du fichier — ce qu'on veut savoir "
         "sans rien ouvrir&nbsp;: sa forme (<font face='Courier'>ELF64 · PIE · dyn</font>) et "
         "sa taille, le <b>langage et le compilateur</b> avec leur degré de certitude, les "
         "quatre <b>protections</b> cochées ou barrées, les quatre nombres (fonctions, "
         "chaînes, fonctions empruntées, appels système), et <b>ce qu'il sait faire</b> par "
         "familles. Le bouton « Analyse du binaire… » est dessous, et mène au détail. "
         "L'onglet ne portait auparavant que ce bouton, et deux mille pixels de vide sous "
         "lui&nbsp;: un onglet dont le seul contenu est « ouvrir une fenêtre » ne mérite pas "
         "la place qu'il occupe."),
        ("note",
         "La fiche est relevée <b>une fois par fichier</b>, et non par image&nbsp;: le scan "
         "ELF, la reconnaissance de chaîne d'outils et le portrait des appels lisent le "
         "fichier entier, et les refaire soixante fois par seconde coûterait une image par "
         "seconde sur un gros binaire. Elle se refait quand le fichier change — un détour "
         "posé, par exemple."),
        ("p",
         "La fenêtre se lit <b>en largeur</b> : une colonne à gauche nomme les "
         "modules, celui qu'on choisit prend tout le reste. « Tout » les remet à la "
         "file, qui est la forme d'un compte rendu — c'est cette vue-là que "
         "« Copier » et « Enregistrer » exportent en Markdown. Le module choisi est un "
         "réglage : on le retrouve au retour."),
        ("p",
         "La colonne est rangée <b>par familles</b>, dans l'ordre où l'on lit un "
         "binaire inconnu : <b>Le fichier</b> (ce qu'il dit de lui-même — son "
         "identité, sa carte, ses protections, sa densité), <b>Le code</b> (où il "
         "est, comment il s'enchaîne, ce qu'il fait de remarquable), "
         "<b>Décompiler</b> (le pseudo-code, et les outils des autres), <b>Ce qu'il "
         "dit</b> (ses chaînes, et la règle qu'on en tire), <b>Ce qu'il emprunte</b> "
         "(appels système et fonctions d'autrui), <b>Les octets</b> (les lire, les "
         "retrouver, les changer). Vingt-quatre rubriques à plat se parcouraient une "
         "à une ; rangées ainsi, on sait où regarder avant d'avoir lu — et l'on "
         "trouve une rubrique qu'on n'a jamais ouverte, parce qu'on sait de quelle "
         "question elle relève."),
        ("p", "Ce qu'on y trouve :"),
        ("liste", [
         "<b>En bref</b> — la première réponse, avant d'ouvrir les autres panneaux : taille "
         "du code, point d'entrée, nombre de fonctions et de chaînes, ce qu'il <b>emprunte</b> "
         "(fonctions de bibliothèque et appels système). Puis <b>ce qu'il sait faire</b> — le "
         "portrait en six familles (réseau, exécution, anti-débogage, mémoire, chiffrement, "
         "fichiers), chacune présente avec le nombre d'appels qui la tiennent et une phrase "
         "d'explication : c'est là qu'on lit d'un coup d'œil <i>si le programme a accès au "
         "réseau</i>, par la table de liaison ou par un numéro d'appel système. Enfin les "
         "<b>protections</b> qui décident de tout — PIE, NX, RELRO, canari —, cochées ou "
         "barrées. Tout est mesuré, jamais déclaré. "
         "Et, depuis le 20 octobre, <b>ce qui l'a produit</b> : le langage et le "
         "compilateur, <b>chacun avec son degré de certitude</b>. C'est la première "
         "question qu'on pose devant un fichier inconnu et la plus rentable — un binaire Go "
         "ne se lit pas comme un binaire C, et savoir qu'on a du C++ devant soi change la "
         "façon dont on lit un <font face='Courier'>call</font> indirect&nbsp;: c'est "
         "peut-être une table virtuelle. "
         "Quatre degrés, et la nuance est tout le sujet&nbsp;: « déclaré par le fichier » "
         "quand la section <font face='Courier'>.comment</font> porte la version exacte, "
         "écrite par le compilateur lui-même&nbsp;; « quasi certain » quand plusieurs "
         "indices de nature différente concordent&nbsp;; « probable »&nbsp;; « un seul "
         "indice ». Dix symboles au même décor ne valent pas mieux qu'un&nbsp;: ils disent "
         "tous la même chose. Ce qui renforce une conclusion, c'est qu'un indice d'une "
         "AUTRE nature la confirme. "
         "<b>« Ce qui le dit »</b> se déplie et montre chaque indice — où on l'a vu, ce "
         "qu'on y a lu, ce qu'on en tire. C'est en les lisant qu'on apprend à les "
         "reconnaître soi-même&nbsp;: le hachage de seize chiffres qui distingue un symbole "
         "Rust d'un symbole C++, la table de lignes que Go emporte, la libc seule qui "
         "désigne le C par élimination. Un script, lui, déclare son interprète sur sa "
         "première ligne, et le noyau le croit&nbsp;: namsdec aussi. "
         "Et tout cela SE FALSIFIE, ce que le panneau dit sous les indices&nbsp;: "
         "<font face='Courier'>.comment</font> s'écrit, les symboles s'effacent, les "
         "chaînes se remplacent. Ces indices disent ce que le fichier <i>montre</i>, non "
         "ce qu'il est — et c'est aussi pourquoi ils sont montrés plutôt que résumés en "
         "un verdict&nbsp;: un outil qui affiche « compilateur&nbsp;: GCC » sans dire "
         "d'où il le tient apprend à croire plutôt qu'à lire.",
         "<b>Empreintes</b> — la taille exacte, le SHA-256 et le CRC-32 du fichier. "
         "C'est la première ligne d'un compte rendu : de quel octet parle-t-on ? Deux "
         "personnes qui lisent « le crackme » n'ont pas forcément le même fichier "
         "sous les yeux. Les deux se copient d'un clic, et le panneau dit laquelle prouve "
         "quelque chose — CRC-32 ne fait que dire si le fichier est arrivé entier.",
         "<b>Carte du fichier</b> — les <b>segments</b>, ce que le noyau charge, avec "
         "leurs droits (un <font face='Courier'>rwx</font> se voit en rouge) ; et les "
         "<b>sections</b>, ce que l'éditeur de liens a rangé, avec leur place dans "
         "le fichier, leur adresse et leur entropie. Savoir à quel endroit on regarde "
         "change ce qu'on y cherche : une entropie haute dans "
         "<font face='Courier'>.text</font> est normale, la même dans "
         "<font face='Courier'>.rodata</font> annonce des données chiffrées.",
         "<b>Signaux</b> — ce que tous les autres ont trouvé, rassemblé : listing trompé, "
         "segment inscriptible <i>et</i> exécutable, entropie qui monte sur une grande "
         "tranche, empaqueteur reconnu, anti-débogage, protections absentes, code qui peut "
         "se déballer, indicateurs dans les chaînes. Il n'analyse rien de neuf — il dit par "
         "où commencer. Chaque ligne donne un <b>fait</b> (une adresse, un nom, une "
         "mesure), <b>ce que ce fait change</b>, et <b>le panneau</b> qui l'explique. "
         "Rien n'est additionné en score : un score donnerait l'illusion d'un verdict, "
         "et c'est le lecteur qui conclut. Quand rien ne ressort, le panneau le dit — un "
         "panneau blanc ressemble à un panneau cassé.",
         "<b>Protections du binaire</b> — PIE, NX, RELRO, canari de pile, FORTIFY, IBT et pile "
         "fantôme, segments W+X, symboles, bibliothèques, RPATH — avec, pour chacune, ce "
         "que son état change concrètement.",
         "<b>Fonctions importées</b> — chaque entrée de la table de liaison retrouve son "
         "nom, avec le <b>nombre d'appels</b> qui y mènent et une phrase d'explication pour les "
         "plus courantes. En tête, le <b>portrait</b> : six familles de comportement — "
         "réseau, exécution, anti-débogage, mémoire, chiffrement, fichiers — chacune avec "
         "le nombre de fonctions qui la tiennent. La question n'est pas « quelles "
         "fonctions » mais « qu'est-ce que ce binaire fait », et trois familles allumées "
         "sont déjà un portrait. "
         "Chaque <b>nom est un lien</b> : le survol donne la phrase, le clic déplie sa "
         "<b>fiche</b> — la signature telle qu'une page de manuel l'écrit, "
         "<font face='Courier'>read(fd, buf, count)</font>, et le <b>registre qui porte "
         "chaque argument</b> (RDI, RSI, RDX, RCX, R8, R9). C'est ce qui transforme "
         "« RDI vaut 0x402000 » en « dest vaut 0x402000 », et c'est la moitié du travail "
         "quand on lit un appel. Une fonction que namsdec ne connaît pas <b>le dit</b> "
         "plutôt que de se taire, et donne ce que son NOM apprend : un préfixe appartient "
         "à une convention — <font face='Courier'>__cxa_</font> est le support "
         "d'exécution C++, <font face='Courier'>_Unwind_</font> le dérouleur de pile des "
         "exceptions et des paniques Rust, <font face='Courier'>_ZN</font> un nom encodé "
         "par le compilateur.",
         "<b>Appels système</b> — le numéro remonté depuis le RAX qui précède, et le nom "
         "qui va avec, avec le même portrait par familles et les mêmes couleurs : un "
         "<font face='Courier'>socket</font> est un <font face='Courier'>socket</font>, "
         "qu'il arrive par la table de liaison ou par le numéro 41 dans RAX — et un "
         "programme qui veut passer inaperçu emploie justement le second. "
         "<b>Le nom est un lien&nbsp;: on le clique, sa fiche se déplie.</b> "
         "« 00400016 1 write » est une ligne juste et muette&nbsp;— elle suppose qu'on "
         "sache ce que <font face='Courier'>write</font> attend, dans quels registres, et "
         "ce qu'il rend. La fiche donne la <b>signature avec ses registres</b> "
         "(<font face='Courier'>write(fd: RDI, buf: RSI, count: RDX)</font>, et RAX pour "
         "la réponse&nbsp;: savoir où mettre les arguments sans savoir où lire le retour "
         "ne sert qu'à moitié), dit si <b>namsdec l'exécute vraiment</b> ou si l'exécution "
         "s'arrêtera là, et porte le <b>renvoi</b> à la page de manuel qui fait autorité — "
         "<font face='Courier'>man 2 write</font> sur man7.org. "
         "La référence d'un appel SYSTÈME est le noyau, et non le manuel du processeur&nbsp;: "
         "Intel décrit l'instruction <font face='Courier'>syscall</font>, qui ne dit rien "
         "de ce que le numéro 1 demande. Quelques appels n'ont pas de page à leur nom et "
         "renvoient à celle d'un parent — <font face='Courier'>exit_group</font> vers "
         "<font face='Courier'>exit(2)</font> — plutôt que vers un lien mort&nbsp;: un "
         "renvoi qui tombe à côté use plus vite la confiance qu'une absence de renvoi.",
         "<b>Chaînes lisibles</b> — cherchées dans les sections de données, comme "
         "<font face='Courier'>strings -d</font>, avec la possibilité de ne montrer que "
         "celles que le code désigne. En tête, les <b>indicateurs</b> : adresses web, "
         "adresses IP, noms de domaine, courriels, chemins absolus, clés de registre. Ce "
         "sont les seules chaînes qu'on puisse aller vérifier <i>ailleurs</i> — dans un "
         "journal réseau, sur un autre poste — et chaque rubrique se copie d'un clic, une "
         "par ligne. Une forme reconnue n'est pas une intention : un chemin peut n'être "
         "qu'un exemple compilé dans un message d'aide, et la règle est écrite pour se "
         "taire plutôt que de se tromper.",
         "<b>Règles YARA</b> — le geste qui suit l'analyse : une règle prête à coller dans "
         "un fichier <font face='Courier'>.yar</font> pour retrouver les frères de cet "
         "échantillon dans un tas. YARA est le langage que lisent les antivirus et les "
         "plateformes d'échantillons. namsdec prend les chaînes les plus distinctives — en "
         "écartant le bruit de bibliothèque, qui répondrait « oui » sur tout ELF compilé "
         "par GCC —, y ajoute le nombre magique du format et l'empreinte en méta, et pose "
         "un seuil prudent (un sous-ensemble des chaînes, non toutes, pour tenir une "
         "famille sans la perdre au premier changement). C'est un <b>point de départ</b>, "
         "pas un verdict : on la resserre ou on l'élargit, et un clic la copie.",
         "<b>Les branchements dont la destination se calcule</b> — "
         "<font face='Courier'>call qword [rip+0x2f3a]</font> ne porte pas sa destination&nbsp;: "
         "elle est dans une case mémoire, et le désassembleur s'arrête là. Sur les seize "
         "binaires d'épreuve, <b>un branchement sur dix est dans ce cas</b> — 16&nbsp;260 "
         "contre 147&nbsp;301 directs. Le listing les nomme désormais en commentaire, par "
         "quatre chemins, du plus sûr au moins sûr&nbsp;: la <b>table de liaison</b> (la "
         "relocation porte le nom de la fonction, écrit par l'éditeur de liens — aucune "
         "déduction)&nbsp;; un <b>pointeur constant</b> dans une case en lecture seule&nbsp;; "
         "un <b>registre chargé</b> quelques instructions plus haut&nbsp;; un "
         "<font face='Courier'>lea</font> relatif à RIP. "
         "Les <b>tables de sauts</b> — ce qu'un <font face='Courier'>switch</font> devient "
         "une fois compilé — sont lues aussi, avec leur nombre de cas&nbsp;: 318 tables et "
         "9&nbsp;631 cas sur le même corpus. "
         "Ces destinations ne servent pas qu'au commentaire&nbsp;: un appel calculé dont on "
         "connaît la cible compte comme un appel direct dans la <b>découverte des "
         "fonctions</b> et dans le <b>graphe d'appels</b>. Le gain se mesure, et il n'est "
         "pas où l'on croirait&nbsp;: <b>994 arêtes d'appel</b> retrouvées pour <b>une seule "
         "fonction</b> de plus. Ce qui se gagne n'est donc pas la découverte de fonctions "
         "inconnues — elles l'étaient déjà par ailleurs — mais la réponse à « qui appelle "
         "qui ». "
         "Ce qui n'entre dans aucun de ces chemins <b>reste inconnu, et le dit</b>. Une "
         "table dont les bornes ne se lisent pas n'est pas lue&nbsp;: le nombre de cas vient "
         "du <font face='Courier'>cmp</font> que le compilateur pose devant tout "
         "<font face='Courier'>switch</font>, jamais d'une estimation — lire une table dont "
         "on ignore la fin, c'est lire ce qui la suit comme si c'était elle. De même, la "
         "résolution IFUNC de la glibc, qui choisit une implémentation selon le processeur, "
         "n'est PAS déterminable avant l'exécution&nbsp;: elle représente le gros des sauts "
         "indirects d'un binaire statique, et prétendre la résoudre produirait un graphe "
         "plausible et faux — pire qu'un graphe incomplet, parce qu'on ne vérifie pas ce "
         "qu'on croit savoir.",
         "<b>Références croisées</b> — qui appelle quoi, qui saute où, qui désigne quelle "
         "adresse. Une adresse qu'un `call` vise est une fonction, même dépouillée de son "
         "nom. Chaque adresse se <b>clique pour y aller</b> dans le listing — la cible d'une "
         "fonction comme l'endroit d'où elle est appelée ; le petit triangle, lui, déplie "
         "d'où viennent les appels sans bouger l'œil.",
         "<b>Fonctions</b> — par où commencer à lire : le <font face='Courier'>main</font> "
         "retrouvé par la convention de démarrage, puis les fonctions classées par nombre "
         "d'appelants — celle qu'on appelle de quarante endroits est un utilitaire, celle "
         "qui n'est appelée qu'une fois est le programme. Le <b>nom se change sur place</b> : "
         "on clique dessus, on tape, Entrée. Il vaut alors partout — listing compris — et "
         "survit à la fermeture ; la flèche de retour rend son "
         "<font face='Courier'>sub_xxxx</font> à la fonction. L'adresse, elle, mène au "
         "listing.",
         "<b>Graphe d'appels</b> — la structure du programme, pas sa liste d'instructions : "
         "l'<b>arbre des appels</b> déplié depuis le point d'entrée, <b>à la façon d'un "
         "diagramme UML</b>. Chaque fonction est une boîte à <b>deux compartiments</b>&nbsp;: "
         "son nom au-dessus du filet, et sous lui ce qu'on mesure — son adresse, sa taille, "
         "combien l'appellent et combien elle en appelle. Ce qu'elle EST s'écrit au-dessus "
         "en <b>stéréotype</b>, entre guillemets comme UML les écrit&nbsp;: "
         "« entrée », « main », « renvoi ». Les traits entre les boîtes sont des "
         "<b>dépendances</b> — en tirets, à pointe ouverte —, parce que c'est exactement ce "
         "qu'un appel est&nbsp;: « celle-ci a besoin de celle-là ». Un trait plein à pointe "
         "pleine dirait une association, un lien durable entre deux objets, ce qui ne veut "
         "rien dire ici&nbsp;; la forme porte donc une information juste au lieu d'en porter "
         "une fausse. Les boîtes sont à angles droits, comme une classe&nbsp;: c'est le "
         "premier signe qui dit qu'on lit un diagramme et non une liste de pastilles. "
         "Une fonction déjà dépliée ailleurs devient un <b>renvoi</b>, pour que l'arbre reste "
         "fini même quand le code boucle. On clique une boîte pour "
         "<b>aller la voir</b> dans le listing, et un champ <b>Aller à</b> mène à n'importe "
         "quelle adresse tapée. Sous l'arbre, chaque fonction se déplie sur son <b>nom</b> "
         "(qu'on peut <b>changer</b> — <font face='Courier'>sub_10450</font> devient "
         "« vérifie_clé », et « Défaut » lui rend son nom d'origine), son <b>code "
         "binaire</b> — les octets bruts — et son <b>pseudo-code</b> — ce que chaque "
         "instruction fait, en clair, côte à côte. Le nom qu'on donne <b>suit l'adresse "
         "partout</b> où elle paraît — la liste des fonctions, les références croisées, le "
         "listing, le déroulement — et une adresse rebaptisée porte un <b>léger fond</b> qui "
         "la distingue des anonymes : on repère d'un coup d'œil ce qu'on a déjà nommé, dans "
         "une liste qui peut compter des centaines de fonctions. Le nom est <b>gardé d'une "
         "séance à l'autre</b>, car c'est la même table de « notes » que le reste de "
         "namsdec. C'est la carte qu'on lit avant de descendre dans une fonction.",
         "<b>Décompilation DZF</b> — les fonctions en <b>pseudo-C</b>, quand le moteur "
         "maison DZF les a rendues. C\'est la suite du graphe d\'appels : du squelette des "
         "appels au corps en clair. Une fonction y porte le <b>nom que vous lui avez donné</b> "
         "dans namsdec, pas celui de DZF — rebaptisée dans l\'arbre, elle l\'est ici aussi. "
         "Les <b>adresses du pseudo-code sont cliquables</b> — <font face='Courier'>0x401030</font>, "
         "<font face='Courier'>sub_401030</font>, <font face='Courier'>loc_401030</font> — et "
         "mènent au listing, à l\'endroit exact visé. "
         "DZF est en cours d\'écriture (bâti à part, en C++23) ; tant qu\'il n\'a rien rendu, "
         "le panneau dit ce qu\'il montrera, avec un bouton <b>Décompiler</b> qui lance le "
         "décompilateur actif et lit sa sortie ici — si c\'est DZF, elle s\'y affiche. "
         "namsdec et DZF échangent par un contrat JSON figé, pour grandir chacun sans "
         "casser l\'autre.",
         "<b>Décompilation par Ghidra</b>, <b>Décompilation par RetDec</b>, "
         "<b>Décompilation par radare2</b>, <b>Désassemblage par objdump</b>, "
         "<b>Désassemblage par ndisasm</b> — un panneau par outil extérieur, et "
         "chacun garde sa sortie. C'est ce qui permet de <b>comparer</b> : Ghidra et "
         "RetDec ne rendent pas la même chose de la même fonction, et c'est en lisant "
         "les deux qu'on apprend ce qu'un décompilateur devine. Les trois premiers "
         "rendent du pseudo-C, les deux derniers des instructions — d'où les deux mots, "
         "qui ne sont pas interchangeables. "
         "Trois règles, les mêmes que pour la fenêtre Décompilation : <b>rien ne "
         "s'exécute tout seul</b> (l'outil ne part qu'au clic sur « Lancer »), la "
         "<b>commande est écrite en toutes lettres</b> avant d'être lancée, et "
         "<b>namsdec n'installe rien à votre place</b> — un outil absent dit ce qu'il "
         "faut taper pour l'obtenir, et laisse taper. La sortie se <b>cherche</b> et se "
         "lit par pages de trois cents lignes ; ses <b>adresses sont cliquables</b> et "
         "mènent au listing, comme ailleurs. "
         "Un outil qui <b>échoue</b> le dit désormais en tête de son panneau — "
         "« l'outil a échoué (code de sortie 7) », et ce qui suit est ce qu'il a tout "
         "de même écrit. Le code de sortie était jeté : un programme qui se plaignait "
         "puis sortait en erreur rendait un texte que rien ne distinguait d'un "
         "résultat normal. La même ligne dit, le cas échéant, que la sortie vient de "
         "la sortie d'erreur, ou qu'elle a été <b>coupée à 4 Mio</b> et qu'elle est "
         "donc incomplète. Cette limite est maintenant une limite de mémoire et plus "
         "seulement d'affichage : la sortie est bornée <i>pendant</i> sa lecture. "
         "Et le <b>délai</b> vaut pour tout ce que l'outil a lancé : il part dans son "
         "propre groupe de processus, et c'est le groupe qu'on arrête — un sous-"
         "processus qui survivait à son père prolongeait l'attente bien au-delà du "
         "délai demandé.",
         "<b>Fragments réutilisables (ROP)</b> — les fins de séquence utiles à une "
         "chaîne ROP : chacune finit par <font face='Courier'>ret</font>, et les "
         "enchaîner revient à écrire un programme avec les morceaux d'un autre. La "
         "récolte va jusqu'à deux mille fragments et se parcourt <b>page par page</b> "
         "— quarante à la fois, avec « 41–80 sur 1 274 » pour dire où l'on en est : la "
         "liste s'arrêtait à quarante sans jamais dire qu'il y avait une suite. Un "
         "champ de recherche les filtre (« pop rdi »), et toute recherche ramène en "
         "première page.",
         "<b>Place libre (bourrage)</b> — le bourrage que l'éditeur de liens laisse pour aligner : "
         "entre deux fonctions, entre deux sections, en fin de segment, il comble avec un "
         "octet neutre (un zéro, un <font face='Courier'>nop</font>, un "
         "<font face='Courier'>int3</font>) jusqu'au prochain multiple de seize. Le panneau "
         "les liste, la plus grande d'abord, avec l'adresse, l'octet qui remplit et la "
         "<b>section</b> qui la porte — une place en fin de <font face='Courier'>.text</font> "
         "ne raconte pas la même chose qu'une place entre deux entrées d'une table. C'est de "
         "la lecture seule, et cela répond à deux questions : pourquoi un binaire pèse plus "
         "que son code, et pourquoi « garder la longueur exacte » n'est pas la seule façon "
         "de le modifier — il y a de la place ailleurs.",
         "<b>Vidage du fichier</b> — les octets tels qu'ils sont sur le disque — et qu'on peut changer, voir « Patcher un binaire » —, et pour "
         "la tranche qu'on choisit : sa taille, son SHA-256, son CRC-32 et son entropie. "
         "Il se copie et s'enregistre au format de <font face='Courier'>xxd</font>, celui "
         "qu'un <font face='Courier'>diff</font> sait lire — c'est ce qui permet de "
         "comparer deux binaires, ou deux états du même. "
         "C'est la réponse à « ces octets-là, qu'est-ce que c'est ? » — un "
         "bloc à 7,9 bits par octet est chiffré ou compressé, un bloc à "
         "4,5 est du code. Cliquer un octet pose le début de la tranche.",
         "<b>Éditeur binaire</b> — le fichier en grille hexadécimale, octet par octet, avec "
         "sa colonne de texte. Le vidage porte une <b>règle de colonnes</b> — "
         "<font face='Courier'>00 01 02 … 0f</font> au-dessus des octets, alignée au "
         "pixel — et ses colonnes portent leur nom : <b>fichier</b>, où l'octet est "
         "écrit, et <b>mémoire</b>, où il sera lu. Sans la règle on comptait les colonnes "
         "avec le doigt pour savoir à quelle position un octet se trouve ; sans les noms, "
         "les deux colonnes d'adresses se ressemblaient assez pour être prises l'une pour "
         "l'autre — sur une page où segment et fichier coïncident, elles sont identiques "
         "chiffre pour chiffre. On patche à la première et l'on raisonne avec la "
         "seconde. On clique un octet, on en pose un autre : remplacer "
         "<font face=\'Courier\'>75</font> (jne) par <font face=\'Courier\'>74</font> (je) "
         "fait lire ce que le programme ferait de l\'autre côté du saut. Ce qu\'on a touché "
         "porte un fond, pour le retrouver. Chaque écriture <b>se défait</b> depuis "
         "Outils › Correctifs, avec l\'octet d\'avant, et <b>rien n\'atteint le disque</b> "
         "tant qu\'on n\'enregistre pas soi-même, vers un fichier qu\'on choisit — jamais "
         "par-dessus l\'original.",
         "<b>Plusieurs octets à la fois.</b> <b>Maj+clic</b> et <b>Maj+flèches</b> tendent "
         "une sélection ; le panneau dit ce qu\'on tient — « 3 octets, 0x2050 … 0x2052 » "
         "—, et le curseur garde un trait sous sa cellule pour qu\'on sache lequel des "
         "trois la frappe touchera. <b>Écrire</b> remplace toute la sélection d\'un seul "
         "correctif, donc d\'un seul geste pour défaire. <b>Remplir de 90</b> la recouvre "
         "de <font face='Courier'>nop</font> : c\'est le geste le plus fréquent devant un "
         "binaire — neutraliser une instruction entière — et il se faisait octet par "
         "octet, en tapant « 9 0 » autant de fois qu\'il le fallait.",
         "<b>Insérer</b> ajoute des octets sans rien retirer, et c\'est le seul geste de "
         "namsdec qui SORTE du cadre de la longueur exacte. Tout ce qui suit se décale : "
         "chaque position devient fausse, et dans un exécutable, chaque en-tête qui "
         "désignait un segment ou une section désigne désormais autre chose. namsdec le "
         "dit <b>avant</b> — « Écrire ferait grandir le fichier : tout ce qui suit se "
         "décale » paraît dès que la longueur ne correspond plus — et refuse après coup "
         "si le fichier ne se relit plus, sans rien perdre. Défaire une insertion la "
         "RETIRE, et rend le fichier exactement : c\'est la condition de tout le reste, "
         "et un contrôle compare le fichier entier, octet par octet.",
         "Le détour reste la bonne réponse quand on veut <i>ajouter du code</i> : il "
         "écrit dans le bourrage que l\'éditeur de liens a laissé, et ne décale rien.",
         "La sélection <b>s\'emporte</b> : <b>Ctrl+A</b> prend tout le fichier, "
         "<b>Ctrl+C</b> copie dans l\'écriture de la colonne active — des octets "
         "hexadécimaux à gauche, le texte à droite, la même règle que pour la frappe : "
         "on copie ce qu\'on voit. <b>Ctrl+V</b> rapporte le presse-papiers <b>dans le "
         "champ</b>, et n\'écrit rien : c\'est le seul endroit du panneau où la frappe "
         "n\'atteint pas le fichier, et c\'est voulu — un collage peut porter quatre "
         "mille octets venus d\'on ne sait où. Posé dans le champ, il se lit avant de "
         "partir, la ligne qui dit ce que l\'écriture ferait à la taille s\'affiche, et "
         "« Écrire » reste un geste qu\'on fait.",
         "<b>Un octet de l\'en-tête ne s\'écrit pas, et namsdec le dit.</b> L\'octet zéro "
         "d\'un ELF porte le <font face=\'Courier\'>7f</font> du nombre magique, et les "
         "soixante-trois suivants disent l\'architecture, le point d\'entrée, où sont les "
         "segments. Les changer rend le fichier illisible — namsdec ne saurait plus le "
         "désassembler —, et le correctif n\'est donc pas posé&nbsp;: le fichier reste "
         "exactement ce qu\'il était. C\'est ce qu\'on voit à l\'ouverture de la grille, "
         "puisqu\'elle commence à l\'octet zéro. Jusqu\'au 21 octobre 2026, ce refus était "
         "<b>muet</b>&nbsp;: on cliquait, on tapait, l\'octet reprenait sa valeur à l\'image "
         "suivante, et rien ne disait pourquoi — ce qui se résume en « dans l\'éditeur "
         "binaire on ne sait rien modifier ». Il s\'explique désormais, et renvoie vers le "
         "geste qui le permet : enregistrer d\'abord une copie, et abîmer l\'en-tête de "
         "celle-là, qui n\'a plus à être relue.",
         "<b>Détourner (ajouter du code)</b> — le champ « Détourner depuis » est "
         "<b>prérempli</b> de la première adresse du listing&nbsp;: il montrait un exemple "
         "en gris, et un exemple en gris dans un champ se lit comme une valeur — on "
         "cliquait « Calculer le plan » pour recevoir « Adresse illisible, attendu de "
         "l\'hexadécimal, par exemple 0x401136 » en regard d\'un champ qui semblait porter "
         "exactement cela. Un champ VIDE le dit désormais en toutes lettres, au lieu de se "
         "déclarer illisible. "
         "C\'est la réponse à ce que l\'éditeur binaire ne "
         "peut pas faire. Tous les correctifs d\'au-dessus gardent la <b>longueur exacte</b> "
         "— inverser un saut, neutraliser par des <font face='Courier'>nop</font>, changer un "
         "octet. C\'est la règle qui les rend sûrs, et elle interdit d\'<b>ajouter</b> quoi "
         "que ce soit : un octet de plus décalerait tout ce qui suit. Le détour ne combat pas "
         "la règle, il la contourne — et c\'est une des plus jolies idées de la "
         "rétro-ingénierie. On pose un <font face='Courier'>jmp</font> à l\'endroit visé vers "
         "une place libre que le binaire contient <b>déjà</b> (voir « Place libre »), on y "
         "écrit le code neuf, on y rejoue les instructions qu\'il a fallu recouvrir pour "
         "loger ce saut, et l\'on revient juste après. Le programme fait un détour, exécute "
         "ce qu\'on a ajouté, retrouve ce qu\'il faisait, et continue. Le fichier n\'a pas "
         "changé de taille : on a seulement rempli des trous.",
         "Le code à ajouter s\'écrit en <b>assembleur</b> — « "
         "<font face='Courier'>mov byte [r11], 0x42</font> » — et namsdec l\'assemble à "
         "l\'adresse où il atterrira vraiment, ce qui compte dès qu\'il s\'adresse "
         "relativement. Qui a déjà ses octets les donne en hexadécimal. Laissé <b>vide</b>, le "
         "détour déplace le code et le ramène sans rien changer : c\'est le cas d\'école, "
         "celui où l\'on voit le mécanisme seul.",
         "<b>Rien ne s\'écrit avant d\'être montré.</b> « Calculer le plan » ne touche pas au "
         "fichier : il affiche ce qui <b>part</b>, ce qui le <b>remplace</b>, et le corps du "
         "détour en entier — adresse, octets, instructions —, désassemblé comme le listing le "
         "montrerait. On pose ensuite, ou pas. Les <b>deux</b> changements partent ensemble et "
         "se défont ensemble depuis Outils › Correctifs : défaire le corps seul laisserait le "
         "saut partir dans du bourrage, et le programme sauterait dans des zéros. namsdec ne "
         "propose donc pas la demi-mesure.",
         "<b>Ce qu\'il refuse, et pourquoi c\'est la moitié de la leçon.</b> Les instructions "
         "recouvertes sont réécrites <b>ailleurs</b>, et une instruction n\'est pas toujours "
         "déplaçable. « <font face='Courier'>lea rsi, [rip+0x2b8a]</font> » ne dit pas « va "
         "chercher à telle adresse » : elle dit « va chercher 0x2b8a octets après la fin de "
         "cette instruction-ci ». Déplacée telle quelle, elle compte depuis un autre endroit "
         "et lit autre chose — <b>sans rien signaler</b>. namsdec la <b>relocalise</b> : il "
         "recalcule son écart pour que la somme retombe sur la même cible. Il en va de même "
         "d\'un <font face='Courier'>call</font> ou d\'un saut relatif de quatre octets. Un "
         "saut <b>court</b>, lui, ne porte qu\'à cent vingt-sept octets : il n\'atteint pas "
         "la place libre, et l\'élargir changerait sa longueur. Celui-là est refusé, et le "
         "refus <b>nomme l\'instruction</b> et dit la taille du champ — parce que c\'est "
         "elle qui explique le non, et qu\'un outil qui sert à apprendre doit dire pourquoi "
         "il ne peut pas.",
         "La place libre où atterrir se choisit juste en dessous, son étiquette "
         "<b>au-dessus</b> de la liste et non à côté. Elle était à côté, et la liste "
         "réclamait une largeur fixe plus grande que le panneau n\'est étroit&nbsp;: la "
         "rangée débordait de cent sept pixels, et <i>tout ce qui suivait</i> — la "
         "consigne du code, l\'avertissement de nasm, le message de refus — se repliait "
         "sur une largeur invisible et se faisait couper par le bord. L\'étiquette du "
         "trou est désormais <b>tronquée</b> si le panneau est étroit, et le survol en "
         "rend le texte entier&nbsp;: l\'adresse, la taille et la section ne s\'obtiennent "
         "nulle part ailleurs.",
         "<b>Et il refuse aussi d\'écrire sur ce qu\'on a déjà corrigé.</b> Un détour "
         "écrit à deux endroits : cinq octets à l\'endroit visé, et son corps dans la "
         "place libre. Si un correctif occupe l\'un ou l\'autre — un autre détour, un "
         "saut inversé, des octets posés à la main depuis l\'éditeur binaire —, le "
         "second passe <b>par-dessus</b> le premier, et c\'est l\'une des rares façons "
         "d\'abîmer un fichier sans que rien ne le dise. Le cas le plus tentant est le "
         "<font face='Courier'>nop</font> qu\'un détour laisse derrière lui : le listing "
         "le montre comme une instruction ordinaire, et le viser ne semble rien coûter. "
         "Mesuré sur l\'émulateur : le programme ne s\'arrête <b>plus du tout</b> — son "
         "<font face='Courier'>hlt</font> a été recouvert, le retour du premier détour y "
         "revient quand même, au milieu d\'un saut. Et il y a pire que le dégât, il y a "
         "la <b>perte du retour en arrière</b> : deux correctifs qui se chevauchent ne se "
         "défont plus dans un ordre unique, et le fichier obtenu dépend de celui dans "
         "lequel on les retire. Le refus nomme donc le correctif qu\'il aurait abîmé, et "
         "invite à le défaire d\'abord — ou à viser ailleurs.",
         "<b>Et combien de fois refuse-t-il ?</b> La question mérite un chiffre plutôt "
         "qu\'une impression, parce qu\'un outil qui refuserait presque partout ne "
         "servirait à rien sans que personne ne s\'en aperçoive. Mesuré sur les onze "
         "binaires d\'épreuve du projet, à toutes les adresses d\'instruction de leur "
         "code — 3,4 millions d\'endroits : <b>deux sur trois acceptent un détour</b>. "
         "L\'accès relatif à <font face='Courier'>rip</font>, qui est la forme normale "
         "d\'un accès à une variable globale en code compilé, ne fait <b>jamais</b> "
         "refuser : la relocalisation le couvre en entier. Le saut court est le seul "
         "refus de principe, et il vaut <b>un site sur dix-huit</b>. Le reste des non "
         "ne vient pas des instructions mais de la <b>place</b> : trois de ces binaires "
         "n\'ont aucun trou exécutable, et un quatrième n\'en a qu\'un, trop petit. "
         "C\'est là que le détour butera, et non sur ce qu\'il faut déplacer.",
         "Deux colonnes en tête de chaque ligne : la <b>position</b> dans le fichier et "
         "l\'<b>adresse</b> à laquelle elle sera lue en mémoire. On patche à la première "
         "et l\'on raisonne avec la seconde, et c\'est pour avoir à les rapprocher de "
         "tête qu\'on se trompe de ligne. La colonne des adresses ne paraît que si elle "
         "apprend quelque chose — dans un exécutable indépendant de la position, le "
         "premier segment est chargé à zéro et les deux colonnes seraient identiques. "
         "« Aller à » accepte les deux : un nombre qui dépasse le fichier mais que les "
         "segments portent est lu comme une adresse.",
         "C\'est aussi là qu\'on atterrit quand on clique une adresse dans un autre "
         "panneau et qu\'on choisit « dans l\'éditeur binaire » : la vue est amenée sur "
         "la position, l\'octet est choisi, et l\'adresse d\'où l\'on vient est écrite en "
         "tête — on vient de cliquer dessus, et la retrouver est ce qui dit qu\'on est au "
         "bon endroit.",
         "<b>Le fichier défile d\'un bout à l\'autre</b>, et il n\'a pas de pages. Il se "
         "montrait par pages de 256 octets, avec « ‹ » et « › » pour tourner — ET une "
         "zone défilante par-dessus, parce que seize lignes ne tenaient pas dans la "
         "hauteur accordée : deux façons de se déplacer dans la même chose, dont l\'une "
         "s\'arrêtait tous les 256 octets sans raison. Seules les lignes visibles sont "
         "construites, si bien qu\'un fichier de plusieurs mégaoctets se parcourt sans "
         "que rien ne soit peint hors de l\'écran.",
         "<b>Huit, seize ou trente-deux octets par ligne</b>, au choix. Trente-deux "
         "remplit un panneau large et fait sauter aux yeux un motif qui se répète ; huit "
         "tient dans un panneau étroit. Des puissances de deux seulement, et c\'est tout "
         "l\'intérêt : chaque ligne commence alors à une adresse ronde, et la règle donne "
         "directement le dernier chiffre de la position. Le choix ne se devine pas tout "
         "seul de la largeur disponible, et c\'est délibéré — un panneau qui lit la place "
         "qu\'on lui laisse pour décider de ce qu\'il occupe se mord la queue, et "
         "celui-ci a déjà payé cette leçon.",
         "L\'octet choisi dit <b>où il est</b> : sa position, son adresse en mémoire, et "
         "la <b>section</b> qui le porte — <font face='Courier'>.text</font>, "
         "<font face='Courier'>.rodata</font>, <font face='Courier'>.data</font> — avec "
         "ce que cela veut dire : du code, des données modifiables, des données en "
         "lecture seule. C\'est la première question qu\'on se pose devant un octet qu\'on "
         "s\'apprête à changer, parce que la réponse décide de tout le reste ; il fallait "
         "jusqu\'ici aller la poser à la carte du fichier, dans un autre panneau.",
         "Sous l\'octet choisi, un <b>inspecteur</b> — comme celui de 010 Editor — lit les "
         "octets à partir de là dans chaque format : <font face=\'Courier\'>int8</font> à "
         "<font face=\'Courier\'>int64</font>, signés et non signés, flottants, caractère, "
         "et les huit bits de l\'octet. En petit-boutien, celui du x86 — l\'octet de poids "
         "faible d\'abord, ce qui déroute tant qu\'on ne l\'a pas vu une fois : "
         "<font face=\'Courier\'>00 01</font> se lit 256, pas 1. Chaque lecture se copie "
         "d\'un clic.",
         "On y <b>cherche</b> aussi : un texte (« ELF »), ou une suite d\'octets en "
         "hexadécimal (<font face=\'Courier\'>7f 45 4c 46</font>), en avant ou en arrière. "
         "La recherche fait le tour quand il n\'y a plus rien devant, et la trouvaille est "
         "surlignée — on saute d\'une occurrence à la suivante comme dans un éditeur.",
         "Et le <b>clavier</b>, une fois un octet choisi : les flèches déplacent le curseur "
         "(gauche/droite d\'un octet, haut/bas d\'une ligne, Page préc./suiv. d\'un écran, "
         "Origine et Fin aux deux bouts de la ligne — et <b>Maj</b> tenue, le "
         "déplacement TIRE la sélection au lieu de la défaire), "
         "et les chiffres hexadécimaux l\'écrivent <b>en place</b>, quartet par quartet — "
         "taper « 7 » puis « 4 » pose <font face=\'Courier\'>74</font> et avance tout seul, "
         "comme dans 010 Editor. Échap annule un quartet commencé. <b>Tab</b> passe à la "
         "<b>colonne de texte</b>, où l\'on tape alors un caractère pour poser son octet — "
         "l\'autre fenêtre sur les mêmes octets, et l\'autre façon d\'éditer. "
         "Sous l\'octet choisi, l\'inspecteur bascule entre <b>petit-boutien</b> (x86, par "
         "défaut) et <b>gros-boutien</b> (formats réseau) : le même quadruplet d\'octets se "
         "lit alors dans l\'autre sens.",
         "<b>Entropie</b> — combien de bits d'information porte un octet, sur tout le "
         "fichier puis par tranches. C'est ce qui répond avant tout le reste à « y a-t-il "
         "seulement quelque chose à lire ici ? » : un binaire empaqueté montre un petit "
         "déchiffreur en clair et un gros bloc illisible, et les autres modules "
         "analyseraient le déchiffreur en croyant analyser le programme. Et quand "
         "l'empaqueteur se <b>nomme</b> — un nom de section, un nombre magique —, le "
         "panneau le nomme aussi, avec la preuve et ce qu'on en fait : "
         "<font face='Courier'>upx -d</font> rend l'original, "
         "<font face='Courier'>pyinstxtractor</font> sort l'archive. « Compressé, chiffré "
         "ou empaqueté » est juste et frustrant ; « c'est de l'UPX » est actionnable.",
         "Le <b>découpage suit la taille du binaire</b> : la tranche vaut 256 octets sur "
         "un petit fichier et grandit par puissances de deux, de façon qu'il y ait "
         "toujours quelques centaines de tranches. Soixante-quatre tranches quoi qu'il "
         "arrive — ce que faisait le panneau — en donnaient seize kilo-octets chacune sur "
         "un fichier d'un méga-octet, et un déchiffreur de deux cents octets s'y noyait. "
         "La taille retenue est écrite en tête : une bosse sur 256 octets et une bosse "
         "sur 64 Ko ne disent pas la même chose.",
         "Le <b>profil</b> prend toute la largeur et monte jusqu'à deux cent quarante "
         "pixels, <b>sans jamais dépasser la moitié de la fenêtre</b> : il faut de la "
         "hauteur pour lire une courbe — sur trente-six pixels, l'écart entre du code "
         "dense et un bloc chiffré en faisait trois — et il reste quatre rubriques à lire "
         "dessous. Des <b>lignes de niveau</b> le graduent en bits par octet, un trait "
         "rouge marque le seuil d'empaquetage (7,2), et une <b>règle d'adresses</b> court "
         "sous le dessin : une bosse « au tiers du fichier » se retrouve enfin quelque "
         "part. Chaque colonne porte la plus haute des tranches qu'elle recouvre — prendre "
         "la moyenne effacerait exactement ce qu'on cherche. Le survol dit la tranche, sa "
         "mesure et son verdict ; le clic emmène le listing à l'adresse.",
         "L'<b>échelle de couleurs</b> est graduée de 0 à 8 sous le profil, avec ses neuf "
         "arrêts — vert, vert-jaune, jaune, orangé, orange, rouge, magenta, violet — et "
         "les cinq verdicts écrits sous la zone qui leur correspond. Neuf arrêts et non "
         "cinq : avec cinq, tout l'intervalle du code ordinaire tenait dans une teinte, et "
         "deux blocs qu'un demi-bit sépare paraissaient identiques.",
         "Enfin les <b>plages</b> : les tranches voisines qui disent la même chose, "
         "réunies, avec leur position dans le fichier, leur taille, leur <b>adresse en "
         "mémoire</b> — cliquable — et leur verdict. Une plage trop courte rejoint sa "
         "voisine la plus proche : un gros bloc chiffré oscille autour du seuil, et sans "
         "cela il se présentait en quatorze morceaux là où il n'y a qu'une chose à voir. "
         "Une position que rien ne projette en mémoire — l'en-tête, la table des "
         "symboles — le dit, plutôt que de laisser une case vide.",
         "<b>UPX — déballer l'original</b> — la suite du geste que le panneau "
         "d'entropie annonce. Il disait « c'est de l'UPX, et <font face='Courier'>upx -d"
         "</font> le rendra » ; cette rubrique le fait, sans qu'on sorte de namsdec. Deux "
         "boutons, et <b>aucun des deux ne touche le fichier ouvert</b> : "
         "<b>Identifier</b> lance <font face='Courier'>upx -l</font> sur une copie "
         "temporaire — lecture seule, rien n'est écrit —, et <b>Déballer "
         "vers\u2026</b> lance <font face='Courier'>upx -d -o &lt;choisi&gt; &lt;copie&gt;"
         "</font>, qui écrit l'original déballé dans un fichier que vous nommez, puis "
         "l'ouvre dans un onglet à côté. L'échantillon empaqueté reste dans le sien, et "
         "l'on compare les deux.",
         "C'est le <font face='Courier'>-o</font> qui fait toute la différence avec la "
         "commande qu'on tape de mémoire. <font face='Courier'>upx -d fichier</font> "
         "écrase l'échantillon <b>en place</b> : on a déballé, et l'on a perdu ce qu'on "
         "analysait — sans sauvegarde, et sans que rien l'ait annoncé. namsdec écrit "
         "donc les octets ouverts dans un fichier temporaire, lance upx là-dessus, et lui "
         "demande d'écrire ailleurs. Votre fichier n'est pas même ouvert en écriture.",
         "Et ce que la rubrique dit <b>quand elle ne trouve rien</b> compte autant que le "
         "reste. Pas de marque <font face='Courier'>UPX!</font>, pas de section "
         "<font face='Courier'>UPX0</font> : cela ne prouve pas l'absence d'UPX. La "
         "signature et les noms de section s'effacent en trois octets, et c'est la "
         "première chose qu'un protecteur fait pour qu'upx refuse de déballer. De même, "
         "« <font face='Courier'>NotPackedException</font> » ne veut pas dire « pas "
         "d'UPX » : elle veut dire que les en-têtes de bloc ne sont pas là où upx les "
         "attend. Sur un binaire ordinaire c'est la bonne réponse ; sur un binaire à "
         "forte entropie, c'est le signe qu'on les a abîmés exprès — et la suite "
         "passe alors par le vidage mémoire, après que le programme s'est déballé "
         "lui-même. La rubrique écrit ces deux nuances sous la réponse, parce qu'une "
         "réponse lue de travers coûte une soirée.",
         "Si upx n'est pas sur la machine, la rubrique le dit et donne la ligne à taper "
         "(<font face='Courier'>sudo dnf install upx</font>) — elle ne l'installe "
         "pas à votre place. C'est la même règle que pour Ghidra, RetDec ou radare2 : "
         "exécuter du code au nom de quelqu'un n'est pas une commodité, c'est une "
         "décision.",
        ]),
        ("p",
         "<b>Les adresses de ces panneaux sont cliquables.</b> Une entrée de liaison, une "
         "zone libre, une figure, un fragment : cliquer l'adresse amène le listing dessus. "
         "C'est la seule chose qu'on veuille faire d'un endroit qu'un panneau désigne, et "
         "il fallait jusqu'ici relever l'adresse à la main pour la retaper dans « aller "
         "à ». Le suivi de RIP s'arrête au passage — sans quoi le pas suivant ramènerait "
         "le listing là où l'exécution en est, et le clic paraîtrait n'avoir rien fait. "
         "Une adresse qui tombe entre deux lignes mène à la ligne juste avant ; une "
         "adresse qui n'est dans aucune partie désassemblée le dit."),
        ("p",
         "<b>Le listing y défile vraiment, et l'analyse s'efface.</b> Deux "
         "défauts se cachaient l'un derrière l'autre. Poser le curseur sur une ligne ne "
         "déplace pas la vue — egui ne défile vers le curseur d'un champ de texte que "
         "lorsque la sélection change pendant que le champ a le focus, ce qui n'arrive "
         "pas quand c'est l'application qui l'écrit : on lisait « Lig 60749 » dans la "
         "barre d'état tandis que le listing restait sur la ligne 1. Et la fenêtre "
         "d'analyse, large par nature, recouvrait ce qu'on venait de demander à voir. "
         "Désormais le listing <b>défile jusqu'à la ligne et la centre</b>, la fenêtre se "
         "ferme (« Analyse du binaire… » la rouvre à la même rubrique), et la barre "
         "d'état dit où l'on vient d'arriver. C'est aussi ce qui rend la <b>barre de "
         "chaleur</b> lisible dans un gros binaire : les lignes chaudes sont à soixante "
         "mille lignes du haut, et l'on n'y arrivait pas — ni depuis l'onglet Profil, ni "
         "depuis un panneau d'analyse."),
        ("p",
         "<b>Le clic demande où aller.</b> Il y a deux façons de regarder le même "
         "endroit, et elles ne répondent pas à la même question : le <b>code</b> dit ce "
         "que la machine y fera, les <b>octets</b> disent ce qui y est écrit. Une "
         "fonction se lit désassemblée ; une chaîne, une table, un bourrage n'ont aucun "
         "sens dans un listing et se lisent dans l'éditeur binaire. Cliquer une adresse "
         "ouvre donc un menu de deux entrées — « dans le code désassemblé », « dans "
         "l'éditeur binaire ». Les deux destinations ne se comportent pas pareil, et "
         "c'est voulu : aller au code <b>quitte</b> la vue Analyse, qui recouvrait "
         "ce qu'on venait demander à voir ; aller aux octets la garde ouverte, puisque "
         "la destination est dedans."),
        ("note",
         "Là où le contexte dit déjà de quoi il s'agit, le choix ne se pose pas : une "
         "boîte du graphe d'appels est une fonction, une adresse lue dans du pseudo-code "
         "ou dans la sortie d'objdump désigne une instruction. Ces clics-là vont droit "
         "au code."),
        ("p",
         "L'analyse, quand on la détache, retrouve son bouton <b>agrandir/restaurer</b> : la "
         "pastille verte, à côté de la rouge. Elle l'avait perdu parce qu'agrandie, elle "
         "recouvrait tout — adresse cliquée, listing derrière un panneau opaque. Les "
         "deux moitiés de ce défaut sont réparées : le saut au listing efface la "
         "fenêtre, et l'agrandissement s'arrête sous la barre de menus, qui reste donc "
         "accessible. Un vidage hexadécimal et un graphe d'appels se lisent mieux en "
         "grand."),
        ("h2", "Les raccourcis, et les deux jeux côte à côte"),
        ("p",
         "<b>Préférences › Raccourcis</b> montre désormais les <b>deux jeux</b>, action "
         "par action : celui qui est actif en clair, l'autre à côté pour comparer. C'est "
         "ainsi qu'on voit ce qu'on vient y chercher — ce qui DIFFÈRE d'un jeu à l'autre, "
         "ce qu'on a soi-même déplacé, et deux actions qui répondraient à la même frappe."),
        ("liste", [
         "Quand les deux jeux disent la même chose — une frappe sur trois : "
         "<font face='Courier'>Ctrl+S</font>, <font face='Courier'>Ctrl+O</font>, "
         "<font face='Courier'>Maj+F5</font> —, la seconde colonne écrit « la même » au "
         "lieu de recopier. L'œil descend alors la colonne et ne s'arrête que sur les "
         "écarts.",
         "Un raccourci qu'on a <b>déplacé soi-même</b> est marqué, et le survol dit ce que "
         "le jeu proposait. Sans cette marque, on croirait que le jeu livré propose ce "
         "qu'on a posé — et l'on chercherait longtemps pourquoi la même touche ne fait pas "
         "la même chose sur une autre machine.",
         "Deux actions qui répondent à la <b>même frappe</b> se voient en rouge, et le "
         "survol nomme l'autre. Les jeux livrés n'en ont aucune — un contrôle le tient — "
         "mais un raccourci qu'on déplace peut très bien en écraser un autre, et rien ne "
         "le disait.",
        ]),
        ("p",
         "Le <b>motif cyclique</b> (Outils › Motif cyclique) est celui de pwntools, octet "
         "pour octet : on l'envoie au programme, et le décalage relu depuis un registre dit "
         "exactement de combien un débordement a dépassé."),
        ("h2", "Écrire ce qu'on a compris"),
        ("p",
         "Un binaire dépouillé n'a que des adresses. On passe une soirée à comprendre que "
         "<font face='Courier'>0x1d491</font> vérifie le mot de passe, et le lendemain il "
         "n'y a plus que <font face='Courier'>0x1d491</font>."),
        ("p",
         "<b>Nommer cette adresse</b> et <b>Commenter cette ligne</b> gardent ce qu'on a "
         "trouvé. Le nom apparaît ensuite partout où cette adresse se montre — pile, "
         "adresses de retour, cibles de saut, graphe —, et il l'emporte sur le symbole du "
         "binaire : renommer <font face='Courier'>sub_1d491</font> en "
         "<font face='Courier'>vérifie_mdp</font>, c'est vouloir voir le second. Le "
         "commentaire se lit à droite de sa ligne. Vider le champ retire la note."),
        ("p",
         "<b>Le listing porte le nom, lui aussi.</b> Il est écrit une fois, à "
         "l'ouverture du binaire, à partir des symboles du fichier : une fonction "
         "rebaptisée paraissait donc partout <i>sauf</i> là où l'on passe ses journées. "
         "Il se récrit maintenant à chaque nom posé — l'étiquette "
         "<font face='Courier'>vérifie_mdp:</font> apparaît au-dessus de l'instruction, "
         "et les points d'arrêt suivent leur adresse plutôt que leur numéro de ligne, "
         "puisqu'une étiquette nouvelle décale tout ce qui la suit. Les symboles "
         "d'origine ne sont pas touchés : c'est ce qui permet de se dédire. Un listing "
         "qu'on a <b>édité à la main</b> n'est pas récrit — on n'efface pas ce travail "
         "pour poser une étiquette."),
        ("note",
         "Les notes sont rangées à côté du binaire, <b>jamais dedans</b> : on n'écrit pas "
         "dans l'exécutable de quelqu'un pour y noter ce qu'on en pense. Et leur clé est "
         "le <b>contenu</b> du fichier, non son chemin — un binaire déplacé ou renommé "
         "retrouve ses notes, un binaire recompilé n'hérite pas de celles d'un autre. Un "
         "repère qui désigne autre chose que ce qu'on croit est pire qu'un repère absent."),
        ("p",
         "Le rapport d'analyse se copie en Markdown ou s'enregistre, pour être collé dans "
         "un compte rendu. Il porte les <b>signaux</b>, le <b>portrait</b> par familles, "
         "les fonctions empruntées avec leur famille et les <b>indicateurs</b> rangés par "
         "nature : de quoi joindre à un compte rendu ce que le destinataire pourra aller "
         "vérifier de son côté. C'est la même fonction qui produit les signaux du rapport "
         "et ceux du panneau — deux synthèses qui diraient des choses différentes "
         "seraient pires qu'une."),

        ("h1", "Quatre séances, et ce qu'elles apprennent"),
        ("p",
         "Quatre crackmes, analysés avec namsdec et racontés en entier. Ils sont rangés "
         "par ce qu'ils enseignent, non par difficulté — et le dernier n'est pas résolu, "
         "ce qui est aussi une leçon. Chacun tient en trois questions : que dit le "
         "fichier de lui-même, où se prend la décision, et que retenir pour la fois "
         "suivante."),
        ("note",
         "Un compte rendu se relit. Tout ce qu'on trouve — l'adresse de "
         "<font face='Courier'>main</font>, le nom d'une fonction, la raison d'un saut — "
         "se note dans namsdec (« Nommer cette adresse », « Commenter cette ligne ») et "
         "ressort en Markdown par le rapport d'analyse. Une soirée de lecture qui ne "
         "laisse rien derrière elle est à refaire."),

        ("h2", "Un XOR contre une constante"),
        ("p",
         "Un ELF de 315 Ko, PIE, dépouillé. Trois constats, et chacun oriente la suite : "
         "PIE, donc les adresses sont des décalages ; dépouillé, donc "
         "<font face='Courier'>main</font> est à retrouver ; 315 Ko pour un crackme, donc "
         "ce n'est pas de l'assembleur écrit à la main."),
        ("p",
         "Le module <b>Chaînes lisibles</b> répond au troisième point : quarante et une occurrences "
         "de <font face='Courier'>rustc</font>. C'est du Rust, et cela décide de la "
         "méthode — en Rust, une chaîne n'a pas de zéro final, c'est un couple (adresse, "
         "longueur). Un <font face='Courier'>strings</font> ordinaire y lit donc des pâtés "
         "où trois messages se suivent sans séparation. La découpe n'est pas dans les "
         "données : elle est dans le code, où chaque chargement d'adresse est suivi du "
         "chargement de sa longueur. namsdec rend alors les chaînes telles que le "
         "programmeur les a écrites — dont "
         "<font face='Courier'>\"Correct Password\"</font>, et donc l'adresse où le "
         "programme décide qu'on a gagné."),
        ("p",
         "<b>Où lire ?</b> Le code d'amorçage ne saute pas dans "
         "<font face='Courier'>main</font> : il le <i>passe</i> en premier argument à "
         "<font face='Courier'>__libc_start_main</font>. Le "
         "<font face='Courier'>lea rdi, [rip + …]</font> qui précède cet appel donne donc "
         "son adresse, dans un binaire qui n'a plus un seul nom."),
        ("p",
         "Le module <b>Figures du code</b> range ce qu'il reconnaît de la plus rare à la plus "
         "commune, et c'est ce classement qui fait le travail : 1 758 tests de nullité "
         "sont la ponctuation du langage machine, quatre comparaisons par XOR sur 62 382 "
         "instructions sont une piste."),
        ("h2", "Quand le listing ment"),
        ("p",
         "Une figure passe avant toutes les autres, parce qu'elle met en cause ce que "
         "namsdec <b>affiche</b> : <b>le listing est trompé</b>. Un désassemblage lit "
         "<i>linéairement</i> — une instruction, sa longueur, la suivante. Le processeur, "
         "lui, va <b>où les sauts l'envoient</b>. Les deux coïncident dans un programme "
         "ordinaire, parce qu'un compilateur fait commencer ses sauts sur des frontières "
         "d'instruction. Elles divergent dès qu'on les fait diverger exprès :"),
        ("code",
         "    eb 01        jmp $+3       ; saute UN octet plus loin que la suite\n"
         "    e8           db 0xE8       ; l'octet-piege : un opcode de `call`\n"
         "    31 c0        xor eax,eax   ; le vrai code commence ici"),
        ("p",
         "La lecture ligne à ligne tombe sur <font face='Courier'>e8</font> et le prend "
         "pour un <font face='Courier'>call</font> : cet opcode avale quatre octets "
         "d'adresse, donc elle engloutit le vrai code, repart de travers et finit en "
         "<font face='Courier'>(bad)</font>. Le processeur, lui, saute par-dessus le piège "
         "et lit <font face='Courier'>xor eax,eax</font>. <b>Un seul octet</b> a suffi pour "
         "que tout un désassemblage mente."),
        ("p",
         "Le panneau donne les <b>deux lectures côte à côte</b> — ce que le listing lit, "
         "ce que le processeur lit — et l'octet qui les sépare. Il ne corrige pas le "
         "listing : refaire le désassemblage en suivant le flot laisserait des trous "
         "partout où l'on ne sait pas aller, et un listing troué serait pire. Nommer "
         "l'endroit suffit à ne plus se faire avoir."),
        ("note",
         "Ce qu'on apprend au passage vaut plus que la figure : <b>le découpage en "
         "instructions n'est pas une propriété des octets</b>. C'est une propriété du "
         "point où l'on commence à lire. Les mêmes octets, lus un cran plus loin, sont un "
         "autre programme — et c'est là-dessus que repose tout l'anti-désassemblage."),
        ("p",
         "Le relevé ne regarde que les sections <b>exécutables</b>, et que les sauts qui "
         "tombent à moins de seize octets. Sans ces deux bornes, une chaîne de format "
         "comme <font face='Courier'>%d%% - </font>, décodée comme du code, rend des "
         "« sauts » qui tombent au hasard : mesuré sur <font face='Courier'>/bin/ls</font>, "
         "cinq figures relevées et aucune vraie. Un panneau qui crie au loup n'est plus lu "
         "du tout."),
        ("code",
         "cmp  rsi, 7               ; la longueur saisie doit valoir 7\n"
         "jne  incorrect\n"
         "mov  ecx, 0x48555648\n"
         "xor  ecx, [rbx + rax]     ; les 4 premiers octets saisis\n"
         "mov  edx, 0x4E444148\n"
         "xor  edx, [rbx + rax + 3] ; les octets 3 à 6\n"
         "or   edx, ecx             ; les deux doivent être nuls\n"
         "je   correct"),
        ("liste", [
         "<b>Un XOR est une comparaison déguisée.</b> "
         "<font face='Courier'>a xor b</font> ne vaut zéro que si "
         "<font face='Courier'>a == b</font> : il n'y a rien à inverser, la constante "
         "<i>est</i> la valeur attendue.",
         "<b>Les octets sortent en petit-boutiste.</b> "
         "<font face='Courier'>0x48555648</font> ne se prononce pas « 48 55 56 48 » : en "
         "mémoire, c'est <font face='Courier'>H V U H</font>. Un octet de décalage dans "
         "cette lecture, et l'on cherche un mot de passe qui n'existe pas.",
         "<b>Les deux tests se recouvrent.</b> Le premier porte sur les octets 0 à 3, le "
         "second sur les octets 3 à 6. La position 3 est lue deux fois, et les deux "
         "lectures coïncident sur un "
         "<font face='Courier'>H</font> — ce qui confirme le décodage et explique la "
         "longueur attendue : 4 + 4 - 1 = 7.",
        ]),
        ("p",
         "La réponse est <font face='Courier'>HVUHADN</font>. Et la leçon dépasse le "
         "crackme : tout entier de quatre ou huit octets dont chaque octet est un "
         "caractère imprimable mérite d'être lu comme du texte. Le réflexe vaut pour les "
         "magies de format (<font face='Courier'>\\x7fELF</font>, "
         "<font face='Courier'>PK\\x03\\x04</font>), les protocoles et les jetons."),

        ("h2", "Un sérial qui se calcule"),
        ("p",
         "Seize kilo-octets, lié dynamiquement, <b>non dépouillé</b> : "
         "<font face='Courier'>main</font> porte son nom. C'est le cas le plus simple "
         "qu'on rencontre, et le bon premier exercice."),
        ("p",
         "Quatre phrases et trois fonctions suffisent à deviner la forme : "
         "<font face='Courier'>\"Enter your name:\"</font>, "
         "<font face='Courier'>\"Enter your serial:\"</font>, un message de réussite, un "
         "d'échec, puis <font face='Courier'>scanf</font>, "
         "<font face='Courier'>strlen</font> et <font face='Courier'>printf</font>. "
         "<font face='Courier'>strlen</font> annonce une boucle sur le nom, donc un calcul "
         "qui en dépend — et non une constante cachée."),
        ("p",
         "C'est toute la différence entre un crackme et un <i>keygen</i>me : il ne s'agit "
         "pas de trouver une réponse, mais la <b>règle</b> qui les produit toutes. Ici "
         "elle tient en une ligne — la somme des codes des lettres du nom, multipliée par "
         "sept, plus cent vingt-trois."),
        ("note",
         "Un détail d'époque, relevé dans ce binaire : la fonction importée est "
         "<font face='Courier'>__isoc23_scanf</font>, et non "
         "<font face='Courier'>__isoc99_scanf</font>. La bibliothèque C préfixe ses "
         "fonctions à nombre variable d'arguments du millésime de la norme qu'elles "
         "suivent, et elle est passée à C23. Un outil qui ne connaît que l'ancien nom ne "
         "reconnaît plus rien — c'est le genre de détail qui fait qu'une analyse "
         "automatique vieillit."),

        ("h2", "Un programme qui refuse d'être observé"),
        ("p",
         "Quatorze kilo-octets pour un « hard » : la difficulté n'est pas dans le volume. "
         "Le vocabulaire est costumé — un « rituel », un « sigil » — mais la structure ne "
         "l'est pas : un nom, une empreinte de vingt-quatre caractères hexadécimaux, une "
         "phrase de réussite. Vingt-quatre hexadécimaux font douze octets, soit trois mots "
         "de trente-deux bits : un indice, avant d'avoir lu une seule instruction."),
        ("p",
         "Et une fonction importée trahit le reste : "
         "<font face='Courier'>ptrace</font>. Le programme appelle "
         "<font face='Courier'>ptrace(PTRACE_TRACEME)</font> à l'entrée — un processus ne "
         "pouvant être suivi que par un seul autre, l'appel échoue si un débogueur est "
         "déjà là, et le programme s'arrête en disant que « les esprits sentent un "
         "observateur »."),
        ("p",
         "namsdec <b>fournit</b> <font face='Courier'>ptrace</font> et "
         "<font face='Courier'>prctl</font> : ils rendent ce que le programme espère, et "
         "la garde passe. Ce n'est pas un contournement caché, c'est la même chose que "
         "fournir <font face='Courier'>write</font> — un appel système simulé rend une "
         "valeur, et celle-ci est dite dans le journal."),
        ("liste", [
         "Une garde anti-débogage se lit dans la liste des fonctions empruntées avant de "
         "se rencontrer à l'exécution. Le module <b>Fonctions importées</b> la montre en "
         "premier.",
         "<font face='Courier'>ptrace</font> n'est que la plus connue. "
         "<font face='Courier'>prctl</font>, un "
         "<font face='Courier'>fork</font> qui se surveille lui-même, la lecture de "
         "<font face='Courier'>/proc/self/status</font> : toutes posent la même question, "
         "et toutes se répondent de la même façon.",
        ]),

        ("h2", "Un programme qui se déballe — et qui résiste"),
        ("p",
         "Trente et un kilo-octets, et pourtant "
         "<font face='Courier'>strings</font> ne rend presque rien : pas de message de "
         "réussite, pas d'invite. Les seules fonctions importées qui comptent sont "
         "<font face='Courier'>mmap</font>, "
         "<font face='Courier'>mprotect</font>, "
         "<font face='Courier'>memcpy</font> et "
         "<font face='Courier'>malloc</font>."),
        ("p",
         "Cette liste suffit à annoncer la couleur : <b>le programme se déballe "
         "lui-même</b>. Un bloc compressé devient vingt-trois kilo-octets de code, posé "
         "dans une page anonyme rendue exécutable par "
         "<font face='Courier'>mprotect</font>. Le code qu'on lit dans le fichier n'est "
         "pas celui qui décidera — et c'est exactement ce que l'entropie disait déjà, "
         "avant qu'on ouvre quoi que ce soit."),
        ("p",
         "Cette analyse-là n'a pas abouti : la structure est établie, la logique de "
         "validation ne l'est pas. Le compte rendu dit où elle se trouve et pourquoi elle "
         "coûte cher, ce qui vaut mieux qu'une conclusion inventée. <b>Un compte rendu "
         "honnête sur ce qu'il n'a pas trouvé est plus utile qu'un compte rendu qui "
         "conclut à tout prix</b> : le suivant reprend là où celui-ci s'arrête, au lieu "
         "de refaire le chemin en doutant."),

        ("h1", "Patcher un binaire"),
        ("p",
         "On ouvre un crackme, on voit le <font face='Courier'>je</font> qu'il faudrait "
         "inverser, et jusqu'ici l'on ne pouvait rien en faire : r\u00e9assembler le "
         "listing est refus\u00e9, et \u00e0 juste titre — un d\u00e9sassemblage "
         "n'est pas un source. \u00c9crire les octets \u00e0 leur place, si. Outils \u203a "
         "<b>Patcher le binaire\u2026</b>, sur la ligne o\u00f9 est le curseur."),
        ("table", [
            ["Geste", "Ce qu'il fait"],
            ["Inverser ce saut", "<font face='Courier'>je</font> devient "
             "<font face='Courier'>jne</font> : le programme prend l'autre branche"],
            ["Forcer le saut (jmp)", "<font face='Courier'>je</font> devient "
             "<font face='Courier'>jmp</font> : la branche est prise quoi qu’il "
             "arrive"],
            ["Neutraliser (nop)", "l'instruction est remplac\u00e9e par des "
             "<font face='Courier'>nop</font> : un appel de v\u00e9rification dispara\u00eet"],
        ]),
        ("p",
         "Les trois gardent la <b>longueur exacte</b> de ce qu'ils remplacent. C'est la "
         "r\u00e8gle qui rend l'op\u00e9ration s\u00fbre : un octet de plus ou de moins "
         "d\u00e9calerait tout ce qui suit, et chaque adresse du fichier deviendrait "
         "fausse. La fen\u00eatre \u00e9crit les octets d'avant et ceux d'apr\u00e8s "
         "<i>avant</i> qu'on clique — on ne change pas les octets de quelqu'un \u00e0 "
         "l'aveugle — et chaque changement se d\u00e9fait."),
        ("p",
         "Inverser et forcer ne r\u00e9pondent pas \u00e0 la m\u00eame question. "
         "Inverser demande \u00ab et si la condition \u00e9tait l\u2019autre ? \u00bb ; "
         "forcer demande \u00ab et si on y allait de toute fa\u00e7on ? \u00bb — "
         "c\u2019est celui qu\u2019on veut devant un contr\u00f4le qu\u2019on n\u2019a pas "
         "encore compris. Le saut long y gagne un <font face='Courier'>nop</font> : "
         "<font face='Courier'>jmp</font> long fait un octet de moins que "
         "<font face='Courier'>0f 8x</font>, et le d\u00e9placement est corrig\u00e9 "
         "d\u2019autant pour arriver exactement o\u00f9 l\u2019autre arrivait."),
        ("note",
         "L'inversion d'un saut tient en un bit, et c'est le processeur qui l'a voulu "
         "ainsi : les conditions vont par paires, et le bit de poids faible de l'opcode dit "
         "laquelle des deux. <font face='Courier'>je</font> est "
         "<font face='Courier'>0x74</font>, <font face='Courier'>jne</font> est "
         "<font face='Courier'>0x75</font>. Il n'y a donc aucune table \u00e0 tenir \u00e0 "
         "jour, et l'inverse de l'inverse retombe toujours sur l'original."),
        ("p",
         "On \u00e9crit aussi les octets \u00e0 la main, depuis le module <b>Vidage du "
         "fichier</b> de la fen\u00eatre d'analyse : on choisit la tranche, on tape ce "
         "qu'on veut \u00e0 la place, et le changement rejoint la liste des patchs. "
         "Autant d'octets que la tranche en compte, exactement — c'est la m\u00eame "
         "r\u00e8gle, et c'est elle qui garde toutes les autres adresses justes."),
        ("p",
         "Le binaire est recharg\u00e9 apr\u00e8s chaque changement : les octets chang\u00e9s "
         "donnent un autre d\u00e9sassemblage, d'autres fonctions, d'autres r\u00e9f\u00e9rences "
         "crois\u00e9es. Puis <b>Fichier \u203a Exporter le binaire</b> \u00e9crit le fichier "
         "patch\u00e9, qui se lance dans un vrai terminal."),

        ("h1", "Sortir de l'atelier"),
        ("p",
         "Fichier › <b>Exporter le binaire</b> écrit le programme tel que "
         "<font face='Courier'>ld</font> l'a produit : un ELF exécutable, celui que la ligne "
         "de commande aurait donné. Il se lance dans un terminal, se lit avec "
         "<font face='Courier'>objdump</font>, se compare avec ce que namsdec en dit."),
        ("p",
         "L'onglet Diagnostics affiche les deux commandes réellement lancées, prêtes à "
         "copier. Un atelier qui ne laisse rien sortir apprend à se servir de lui, pas de "
         "l'assembleur."),

        ("h1", "Ajouter des plugins"),
        ("p",
         "Tout ce qui s'ajoute à namsdec sans le recompiler se range en deux familles, et la "
         "différence entre elles n'est pas une question de classement : c'est une question de "
         "confiance."),
        ("table", [
            ["Famille", "Ce que c'est", "Ce que ça peut faire"],
            ["Thèmes", "des couleurs, et rien d'autre", "rien : namsdec lit des nombres et peint"],
            ["Fonctionnalités", "la description d'un programme", "ce que ce programme peut faire"],
        ]),
        ("p",
         "Un thème ne s'exécute pas. Une fonctionnalité, si — et c'est pourquoi namsdec ne va "
         "jamais en chercher tout seul, ne les lance que sur demande explicite, et écrit la "
         "commande complète avant de la lancer."),
        ("h2", "Où ils vivent"),
        ("code",
         "~/.local/share/namsdec/\n"
         "|-- modules/         TOUS les outils extérieurs\n"
         "|-- themes/          les thèmes\n"
         "|-- exercises/       les séries d'exercices\n"
         "`-- tools/           ce que namsdec a bâti lui-même"),
        ("p",
         "<b>Un seul dossier pour les modules</b>, et la fiche dit ce qu'elle rend. Il y en "
         "avait deux — <font face='Courier'>disassemblers/</font> et "
         "<font face='Courier'>decompilers/</font> —, et c'était le DOSSIER qui portait le "
         "sens&nbsp;: la même fiche changeait de nature selon l'endroit où on la posait, et "
         "une fiche recopiée dans le mauvais dossier était simplement ignorée, sans message. "
         "Elle porte maintenant un champ <font face='Courier'>\"kind\"</font>, qui vaut "
         "<font face='Courier'>disassembler</font> (par défaut) ou "
         "<font face='Courier'>decompiler</font>. Les deux dossiers d'avant sont relus au "
         "premier lancement et leur contenu recopié&nbsp;; rien n'y est effacé."),
        ("p",
         "Installer, c'est copier un fichier JSON dans le bon dossier. Depuis namsdec : "
         "Préférences › Plugins pour un thème, Préférences › Outils pour un décompilateur. Le "
         "dépôt en contient des exemples prêts à l'emploi, dans son dossier "
         "<font face='Courier'>plugins/</font>, avec le format décrit champ par champ."),
        ("h2", "Installer un décompilateur"),
        ("p",
         "namsdec n'installe pas le programme lui-même : télécharger et exécuter du code au nom "
         "de quelqu'un n'est pas une commodité, c'est une décision. Il dit ce qu'il faut taper, "
         "et laisse taper."),
        ("table", [
            ["Outil", "Ce qu'il rend", "Comment l'obtenir"],
            ["radare2", "du pseudo-code, sans greffon", "sudo dnf install radare2"],
            ["Rizin + rz-ghidra", "le décompilateur de Ghidra",
             "sudo dnf install rizin rizin-plugin-ghidra"],
            ["RetDec", "du C compilable, lentement", "github.com/avast/retdec/releases"],
            ["objdump, ndisasm", "des instructions", "déjà là si namsdec assemble"],
            ["DZF", "un désassembleur maison, en cours",
             "bouton Installer…, ou git clone github.com/fredza/DZF"],
        ]),
        ("h2", "Installé, et proposé comme moteur"),
        ("p",
         "Chaque fiche répond à <b>deux</b> questions, et elles ne pèsent pas le même poids. "
         "Le programme est-il <b>installé sur la machine</b>&nbsp;? S'il l'est, "
         "<b>il sert déjà</b>&nbsp;: l'onglet RE lui donne son propre panneau, où l'on clique "
         "« Lancer ». Il n'y a rien à activer pour cela."),
        ("note",
         "La seconde pastille disait « activé dans namsdec » / « désactivé dans namsdec », et "
         "c'était trompeur&nbsp;: on lisait « désactivé » sur six outils installés dont les "
         "panneaux fonctionnaient parfaitement — « je ne comprends pas, tout est désactivé » "
         "(signalé le 20 octobre 2026). Elle ne parle que de la <b>liste des moteurs</b> de la "
         "vue de désassemblage, celle où l'on choisit qui désassemble le listing. Elle dit "
         "donc maintenant « proposé comme moteur » ou « pas dans la liste des moteurs », et le "
         "bouton « Ajouter aux moteurs »."),
        ("p",
         "L'y ajouter n'écrit qu'un fichier de quelques lignes dans "
         "<font face='Courier'>~/.local/share/namsdec/modules/</font> : rien n'est téléchargé "
         "ni lancé tant qu'on ne le demande pas. Un outil proposé comme moteur mais disparu de la "
         "machine échouera au lancement, ce que la fiche dit en rouge. Un troisième réglage, distinct des deux "
         "premiers, est le <b>module d'installation</b> (Préférences › Modules) : c'est lui qui "
         "sait aller chercher les sources et les construire."),
        ("p",
         "Un outil <b>incomplet</b> — rizin sans son greffon rz-ghidra, par exemple — ne "
         "s'ajoute pas aux moteurs : l'y mettre déclarerait un décompilateur qui échoue au "
         "premier essai (la commande <font face='Courier'>pdg</font> n'existe pas sans le "
         "greffon). Son bouton est donc grisé, et c'est <b>Réparer</b>, en vif, qui mène la "
         "marche — poser la pièce qui manque, puis activer. On ne peut activer qu'un outil "
         "prêt."),
        ("p",
         "La décompilation elle-même est une fonction optionnelle, allumée dans Modules. Tant "
         "qu'elle est éteinte, activer un outil ici ne l'emploie nulle part — l'onglet Outils "
         "le dit en tête, avec de quoi l'allumer sur place, sans aller chercher la case dans un "
         "autre onglet."),
        ("p",
         "La fiche d'un outil absent dit <b>comment l'obtenir sans rien déplier</b> : c'est une "
         "panne, pas un détail. Le repli « Détails » ne garde que la commande exacte qui sera "
         "lancée — ce qu'on va chercher en se demandant « et sans namsdec, comment je fais ? ». "
         "Les fiches s'ouvraient d'office quand l'outil manquait, et sur une machine neuve, où "
         "aucun ne l'est, les deux derniers outils passaient sous le bord de la fenêtre. Le "
         "nombre d'outils connus est écrit à côté du titre : la liste se déroule."),
        ("p",
         "namsdec cherche un outil dans le PATH et dans son propre dossier d'outils. Ce qu'on a "
         "bâti soi-même ailleurs — un binaire compilé dans le dépôt d'un projet, sans l'avoir "
         "installé — n'est ni l'un ni l'autre, et reste « absent » quoi qu'on fasse. La fiche d'un "
         "outil absent propose donc <b>Localiser le binaire…</b> : on désigne le fichier une fois, "
         "namsdec le retient et l'emploie tel quel. C'est la voie pour DZF, et pour tout outil "
         "rangé hors des chemins habituels."),
        ("note",
         "Un outil qui répond ne propose que <b>Fermer</b>. « Recommencer » était là, et c'était "
         "une offre sans motif : vingt minutes de compilation pour un résultat identique, et l'on "
         "finissait par douter que l'installation ait abouti. Un outil <b>incomplet</b>, lui, "
         "propose <b>Réparer</b> — c'est le seul cas où relancer la recette sert à quelque "
         "chose."),
        ("note",
         "<b>DZF est dans la liste bien qu'il soit en cours d'écriture</b> : c'est le "
         "désassembleur x86-64 maison, en C++23 et sans aucune dépendance. Il ne lit pas l'ELF, "
         "et n'a pas à l'apprendre — namsdec lui passe les octets exécutables et leur adresse. "
         "Depuis la 0.2.8-beta.1, <b>namsdec sait l'installer lui-même</b> : comme pour Rizin et "
         "RetDec, allumez le module d'installation, puis <b>Installer…</b> sur sa fiche. namsdec "
         "clone <font face='Courier'>github.com/fredza/DZF</font>, le construit avec "
         "<font face='Courier'>cmake</font>, et pose <font face='Courier'>x64-disassembler</font> "
         "et <font face='Courier'>dzf-plugin-engine</font> dans son dossier d'outils — rien hors "
         "de là, aucun <font face='Courier'>sudo</font>. Il faut seulement "
         "<font face='Courier'>git</font>, <font face='Courier'>cmake</font>, "
         "<font face='Courier'>make</font> et un compilateur C++23. Si vous l'avez bâti vous-même "
         "ailleurs, <b>Localiser le binaire…</b> (voir ci-dessus) reste la voie : on montre une "
         "fois où est <font face='Courier'>x64-disassembler</font>, et il devient prêt."),
        ("note",
         "<b>Rizin seul ne décompile pas</b> : la commande <font face='Courier'>pdg</font> vient "
         "du greffon <font face='Courier'>rz-ghidra</font>. Là où il n'est pas empaqueté, "
         "<font face='Courier'>rz-pm -i rz-ghidra</font> l'installe. namsdec vérifie sa présence "
         "et affiche « incomplet » plutôt que de vous laisser essayer."),
        ("p",
         "Cette vérification <b>lance l’outil</b> — c’est la seule façon de "
         "savoir si <font face='Courier'>pdg</font> répond. Son résultat est donc "
         "gardé : il est refait à la fin d’une installation, à l’ouverture "
         "de la fenêtre, et sur le bouton <b>Revérifier</b>. Ce dernier est là "
         "pour le cas où l’on vient d’installer quelque chose dans un terminal et "
         "où l’on veut que namsdec le voie sans redémarrer."),
        ("p",
         "Le module reste éteint tant qu'on ne l'allume pas (Préférences › Modules), l'entrée de "
         "menu n'apparaît qu'ensuite, et rien ne part sans le bouton « Lancer ». Passé deux "
         "minutes, un outil qui ne rend pas la main est arrêté."),
        ("h2", "Brancher son propre outil"),
        ("p",
         "La fiche dit ce que l'outil veut recevoir, et namsdec le lui donne. Quatre "
         "marqueurs, à poser dans ses arguments :"),
        ("table", [
            ["Marqueur", "Ce que namsdec y met"],
            ["{file}", "le binaire entier, tel qu'il est sur le disque"],
            ["{text}", "les octets exécutables seuls, sans en-tête ni données"],
            ["{base}", "l'adresse où ces octets sont chargés, en hexadécimal"],
            ["{out}", "un fichier où écrire, pour qui ne parle pas sur la sortie standard"],
        ]),
        ("p",
         "Les deux du milieu servent les outils qui <b>ne lisent pas l'ELF</b> — et c'est le "
         "cas du premier désassembleur que quiconque écrit. Sans eux, un tel outil commence "
         "par désassembler l'en-tête du fichier comme s'il s'agissait de code (les vingt "
         "premières lignes ne veulent rien dire) et adresse tout son listing à partir de "
         "zéro, si bien qu'aucun saut ne tombe juste. <b>C'est namsdec qui sait lire "
         "l'ELF</b> ; il n'y a pas de raison de le faire réapprendre à chaque outil."),
        ("code",
         '{\n'
         '  "id": "dzf",\n'
         '  "name": "DZF - desassemblage x86-64 (maison)",\n'
         '  "command": "x64-disassembler",\n'
         '  "args": ["--base", "{base}", "{text}"]\n'
         '}'),
        ("p",
         "C'est la fiche livrée avec DZF, le désassembleur x86-64 maison écrit en C++ sans "
         "aucune dépendance. Il n'a rien eu à changer pour recevoir du code propre : c'est "
         "la fiche qui dit ce qu'il veut."),

        ("h1", "Régler namsdec"),
        ("p",
         "Les préférences couvrent le thème, la langue (français ou anglais), les modules, "
         "l'affichage et les raccourcis — chacun réattribuable. <b>Tout réglage est "
         "enregistré</b> dans <font face='Courier'>~/.config/namsdec/settings.json</font>, "
         "un fichier lisible et modifiable à la main. <i>Tout</i> : les panneaux ouverts "
         "ou fermés — l'explorateur de fichiers compris —, et jusqu'aux filtres des vues, "
         "comme « seulement le chemin courant » du graphe de flot ou « seulement les pas "
         "remarquables » du déroulement. Un contrôle le vérifie en lisant le code qui "
         "bascule : un réglage neuf tombe sous la règle sans qu'on y pense."),
        ("note",
         "Tout ce que namsdec réécrit tout seul — les réglages, les raccourcis, les noms "
         "que vous donnez aux fonctions d'un binaire — est remplacé <b>d'un bloc</b> : le "
         "nouveau contenu est écrit à côté, poussé sur le disque, puis renommé par-dessus "
         "l'ancien. À tout instant le fichier est l'ancien ou le nouveau, jamais un mélange. "
         "Sans cela, une coupure de courant au mauvais moment laissait un fichier tronqué "
         "— que la lecture suivante ne savait pas analyser, et remplaçait par les défauts "
         "sans un mot. L'enregistrement de votre source suit la même règle."),
        ("p",
         "L'onglet <b>Modules</b> sépare deux choses qu'un même mot embrouillait : les "
         "<b>Composants</b> dont namsdec est fait — sains ou non, on ne les allume pas, on "
         "lit leur état — et les <b>Fonctions optionnelles</b> qu'on choisit d'allumer "
         "(l'onglet RE, les exercices, la décompilation par un outil extérieur, et le module "
         "qui va chercher et construit ces outils). Rien d'optionnel n'est obligatoire : "
         "namsdec tourne sans."),
        ("p",
         "Des thèmes et des désassembleurs supplémentaires s'installent comme les séries "
         "d'exercices : un fichier déposé dans "
         "<font face='Courier'>~/.local/share/namsdec/</font>. Un désassembleur externe est "
         "un <i>programme</i> : il ne s'exécute que sur demande, et le panneau annonce la "
         "commande avant de la lancer."),

        ("h2", "Reprendre son travail"),
        ("p",
         "namsdec rouvre ce qu'on avait : les fichiers sources et leurs onglets, le "
         "<b>binaire</b> ouvert, ses points d'arrêt, ses zones surveillées, les arguments "
         "et variables d'environnement du programme, l'onglet latéral qu'on regardait, et "
         "la <b>vue</b> qu'on avait choisie dans la rangée — l'établi se retrouve là où "
         "on l'a laissé."),
        ("p",
         "Et <b>la ligne qu'on lisait</b>. Un listing de binaire fait des dizaines de "
         "milliers de lignes : retrouver le fichier et ses points d'arrêt mais rouvrir à "
         "la ligne 1, c'était laisser à refaire le plus long — se replacer. La vue "
         "revient dessus, centrée, le curseur l'y suit, et le mode ne change pas : on "
         "rouvre un binaire pour le lire."),
        ("p",
         "Les <b>fenêtres d'établi</b> reviennent aussi : la calculatrice, le motif "
         "cyclique, l'aide, les exercices — et les <b>vues qu'on avait détachées</b>, "
         "qui reviennent détachées : le désassemblage, le déroulement, la "
         "décompilation, l'analyse, la chronologie, les points d'arrêt, les registres "
         "étendus et les correctifs. Ce qu'on a disposé autour de son code est un réglage comme un "
         "autre. Les boîtes qui posent une question — Préférences, « À propos », la saisie "
         "du programme — ne se rouvrent pas toutes seules : elles se ferment sur leur "
         "réponse. Les deux fenêtres qui appartiennent à un module ne reviennent que si le "
         "module est encore allumé. Elles se posent en <b>damier</b> et non les unes "
         "sur les autres : chacune a sa case, une ligne sur deux décalée d'un quart "
         "de pas, si bien qu'on voit du premier coup combien il y en a et qu'on "
         "attrape celle qu'on veut par sa barre de titre."),
        ("p",
         "C'était un <b>escalier</b> jusqu'ici — un seul cran en diagonale, le même "
         "pour tous —, et il ne suffisait pas. Son pas s'arrêtait à vingt-quatre "
         "pixels : douze fenêtres tenaient dans deux cent soixante pixels de "
         "diagonale, et sur un grand écran elles se rouvraient en tas au milieu. "
         "Mesuré aux quatre tailles d'écran du banc d'essai, vingt barres de titre "
         "étaient entièrement couvertes ; il en reste cinq, et aucune au-dessus de "
         "2560 x 1440. Une fenêtre dont la poignée est enterrée n'est pas seulement "
         "laide à voir : on la cherche dans le menu « Affichage » en la croyant "
         "fermée. Les fenêtres qui s'ouvrent à la taille de l'écran — le graphe, "
         "quand un programme est chargé — gardent le centre : il n'y a pas de place "
         "pour les étaler, et un cran ne ferait que les pousser dehors."),
        ("p",
         "<b>Aucune ne sort de l'écran.</b> Sur un affichage étroit — namsdec accepte de "
         "descendre jusqu'à 800 x 520 —, c'est le <b>contenu</b> d'une fenêtre qui défile, "
         "jamais son bandeau qui passe au-dessus du bord. La différence n'est pas "
         "cosmétique : une fenêtre dont le bandeau n'est plus visible ne se déplace plus "
         "et ne se ferme plus à la souris. La calculatrice, « À propos » et les registres "
         "étendus en sortaient — de 236, 79 et 68 pixels, mesurés — et c'est le haut qui "
         "manquait, car une fenêtre trop haute déborde des deux côtés à la fois."),
        ("note",
         "Les repères posés sur un binaire ne reviennent que si le <b>fichier n'a pas "
         "changé</b> — taille et date. Ils se comptent en numéros de ligne d'un listing "
         "reconstruit à chaque ouverture ; sur un binaire recompilé entre-temps, ils "
         "désigneraient d'autres instructions. Le binaire se rouvre quand même, les repères "
         "sont oubliés, et namsdec le dit."),

        ("p",
         "Tout ce que namsdec sait faire a un <b>chemin qui se voit</b> : une entrée de "
         "menu ou un bouton, avec son raccourci écrit à côté. Huit actions n'avaient que "
         "le clavier — dupliquer une ligne, surligner les occurrences, aller à la "
         "suivante, les renommer toutes, nommer ou commenter une adresse, revenir au "
         "fichier précédent. La palette de commandes les offre toutes elle aussi, mais "
         "elle est un filet, pas un chemin : on l'ouvre quand on sait déjà ce qu'on "
         "cherche, tandis qu'un menu se parcourt."),

        ("h1", "Les raccourcis"),
        ("p",
         "Deux jeux sont livrés. <b>Fake Vim-GDB</b>, celui d'origine, s'inspire de vim "
         "et de gdb sans les imiter : il en reprend les gestes — les modes, la touche "
         "« leader », les noms des commandes d'exécution — mais ni le langage de commande "
         "de l'un ni celui de l'autre. <b>Éditeur ordinaire</b> emploie les raccourcis "
         "auxquels on s'attend ailleurs. On change de jeu dans Préférences › Raccourcis, "
         "et les deux couvrent les mêmes actions."),
        ("p",
         "Les séquences notées « Espace puis … » sont des touches relâchées l'une après "
         "l'autre, à la façon de la touche « leader » de vim. Elles ne s'arment que lorsque "
         "l'éditeur n'a pas le focus, sinon un espace tapé dans le code déclencherait tout."),
        ("p",
         "Les raccourcis vivent dans <font face='Courier'>~/.config/namsdec/keymap.json</font>, "
         "qui ne garde que ce qui <b>s’écarte</b> du jeu choisi : un jeu qui gagne une "
         "action l’apporte alors sans qu’on ait rien à faire. Le fichier est "
         "<b>surveillé</b> — modifié pendant que namsdec tourne, il est repris dans "
         "la seconde, et la barre d’état le dit. Et Préférences › Raccourcis "
         "propose d’<b>exporter le jeu entier</b>, à plat : les trente actions "
         "écrites une par une, pour partir d’une liste complète plutôt que d’un "
         "écart."),
    ]


# --- Le rendu ------------------------------------------------------------

ENCRE = colors.HexColor("#1f2430")
DOUX = colors.HexColor("#5a6478")
ACCENT = colors.HexColor("#3b6ea5")
FILET = colors.HexColor("#c8cedb")
FOND = colors.HexColor("#f4f6fa")


def styles():
    base = getSampleStyleSheet()
    s = {}
    s["titre"] = ParagraphStyle(
        "titre", parent=base["Title"], fontName="Helvetica-Bold",
        fontSize=30, leading=34, textColor=ENCRE, spaceAfter=6)
    s["sous_titre"] = ParagraphStyle(
        "sous_titre", parent=base["Normal"], fontName="Helvetica",
        fontSize=13, leading=18, textColor=DOUX, alignment=1)
    s["h1"] = ParagraphStyle(
        "h1", parent=base["Heading1"], fontName="Helvetica-Bold",
        fontSize=17, leading=21, textColor=ENCRE, spaceBefore=16, spaceAfter=6)
    s["h2"] = ParagraphStyle(
        "h2", parent=base["Heading2"], fontName="Helvetica-Bold",
        fontSize=12.5, leading=16, textColor=ACCENT, spaceBefore=11, spaceAfter=3)
    s["p"] = ParagraphStyle(
        "p", parent=base["BodyText"], fontName="Helvetica", fontSize=9.7,
        leading=14.2, textColor=ENCRE, alignment=TA_JUSTIFY, spaceAfter=6)
    s["liste"] = ParagraphStyle(
        "liste", parent=s["p"], leftIndent=12, bulletIndent=3, spaceAfter=3.5)
    s["code"] = ParagraphStyle(
        "code", parent=base["Code"], fontName="Courier", fontSize=8.6,
        leading=12, textColor=ENCRE, leftIndent=8, spaceBefore=3, spaceAfter=8)
    s["note"] = ParagraphStyle(
        "note", parent=s["p"], leftIndent=8, rightIndent=8,
        spaceBefore=4, spaceAfter=4)
    s["pied"] = ParagraphStyle(
        "pied", parent=base["Normal"], fontName="Helvetica", fontSize=7.6,
        textColor=DOUX)
    return s


def rendre(chemin, v, blocs):
    """Écrit le PDF. Une page de titre, puis le corps à deux niveaux."""
    aujourdhui = date.today().strftime("%d/%m/%Y")

    def décor(canvas, doc):
        """L'en-tête et le pied : la version d'un côté, la page de l'autre."""
        canvas.saveState()
        largeur, hauteur = A4
        if doc.page > 1:
            canvas.setFont("Helvetica", 7.6)
            canvas.setFillColor(DOUX)
            canvas.drawString(20 * mm, hauteur - 12 * mm,
                              "namsdec %s — guide de l'utilisateur" % v)
            canvas.setStrokeColor(FILET)
            canvas.setLineWidth(0.4)
            canvas.line(20 * mm, hauteur - 14 * mm, largeur - 20 * mm, hauteur - 14 * mm)
            canvas.line(20 * mm, 15 * mm, largeur - 20 * mm, 15 * mm)
            canvas.drawString(20 * mm, 11 * mm, "namsdec.io")
            canvas.drawRightString(largeur - 20 * mm, 11 * mm, "page %d" % doc.page)
        canvas.restoreState()

    doc = BaseDocTemplate(
        str(chemin), pagesize=A4,
        leftMargin=20 * mm, rightMargin=20 * mm,
        topMargin=20 * mm, bottomMargin=20 * mm,
        title="namsdec %s — guide de l'utilisateur" % v,
        author="namsdec", subject="Atelier d'assembleur x86-64 et de rétro-ingénierie",
    )
    cadre = Frame(doc.leftMargin, doc.bottomMargin, doc.width, doc.height, id="corps")
    doc.addPageTemplates([PageTemplate(id="page", frames=[cadre], onPage=décor)])

    st = styles()
    flux = []

    # --- Page de titre ---
    flux.append(Spacer(1, 52 * mm))
    flux.append(Paragraph("namsdec", st["titre"]))
    flux.append(Paragraph("Guide de l'utilisateur", st["sous_titre"]))
    flux.append(Spacer(1, 6 * mm))
    flux.append(Paragraph(
        "version <b>%s</b> &nbsp;·&nbsp; %s" % (v, aujourdhui), st["sous_titre"]))
    flux.append(Spacer(1, 22 * mm))
    flux.append(Paragraph(
        "Apprendre l'assembleur x86-64 sous Linux,<br/>et lire les binaires "
        "qu'on n'a pas écrits.", st["sous_titre"]))
    flux.append(Spacer(1, 40 * mm))
    flux.append(Paragraph(
        "Logiciel libre, licence MIT &nbsp;·&nbsp; namsdec.io", st["sous_titre"]))
    flux.append(PageBreak())

    # --- Sommaire, dressé depuis les titres ---
    flux.append(Paragraph("Sommaire", st["h1"]))
    chapitres = [texte for genre, texte in blocs if genre == "h1"]
    lignes = [[Paragraph("%d." % (rang + 1), st["p"]), Paragraph(titre, st["p"])]
              for rang, titre in enumerate(chapitres)]
    sommaire = Table(lignes, colWidths=[10 * mm, doc.width - 10 * mm])
    sommaire.setStyle(TableStyle([
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 1),
        ("TOPPADDING", (0, 0), (-1, -1), 1),
    ]))
    flux.append(sommaire)
    flux.append(PageBreak())

    # --- Le corps ---
    numéro = 0
    for genre, contenu_bloc in blocs:
        if genre == "h1":
            numéro += 1
            flux.append(Paragraph("%d. %s" % (numéro, contenu_bloc), st["h1"]))
        elif genre == "h2":
            flux.append(Paragraph(contenu_bloc, st["h2"]))
        elif genre == "p":
            flux.append(Paragraph(contenu_bloc, st["p"]))
        elif genre == "liste":
            for item in contenu_bloc:
                flux.append(Paragraph(item, st["liste"], bulletText="•"))
            flux.append(Spacer(1, 3))
        elif genre == "code":
            flux.append(_cadre_code(contenu_bloc, st, doc.width))
        elif genre == "note":
            flux.append(_cadre_note(contenu_bloc, st, doc.width))
        elif genre == "table":
            flux.append(_tableau(contenu_bloc, st, doc.width))
        else:
            # Un genre inconnu passait à la trappe sans un mot : un titre
            # écrit « h3 » — qui n'existe pas ici — a disparu du manuel
            # sans que rien ne le dise, et le PDF s'est généré comme si
            # de rien n'était.
            raise SystemExit(
                f"genre de bloc inconnu : {genre!r} — "
                f"attendus : h1, h2, p, liste, code, note, table"
            )
    doc.build(flux)


def _cadre_code(texte, st, largeur):
    table = Table([[Preformatted(texte, st["code"])]], colWidths=[largeur])
    table.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), FOND),
        ("BOX", (0, 0), (-1, -1), 0.5, FILET),
        ("LEFTPADDING", (0, 0), (-1, -1), 8),
        ("TOPPADDING", (0, 0), (-1, -1), 6),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
    ]))
    return KeepTogether(table)


def _cadre_note(texte, st, largeur):
    table = Table([[Paragraph(texte, st["note"])]], colWidths=[largeur])
    table.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), colors.HexColor("#fdf6e3")),
        ("LINEBEFORE", (0, 0), (0, -1), 2.2, ACCENT),
        ("LEFTPADDING", (0, 0), (-1, -1), 9),
        ("TOPPADDING", (0, 0), (-1, -1), 7),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 7),
    ]))
    return KeepTogether(table)


def _tableau(lignes, st, largeur):
    entête = ParagraphStyle("entête", parent=st["p"], fontName="Helvetica-Bold",
                            textColor=colors.white, spaceAfter=0)
    cellule = ParagraphStyle("cellule", parent=st["p"], spaceAfter=0, fontSize=9)
    données = [[Paragraph(c, entête) for c in lignes[0]]]
    for ligne in lignes[1:]:
        données.append([Paragraph(_monospace(c), cellule) for c in ligne])
    colonnes = [largeur / len(lignes[0])] * len(lignes[0])
    table = Table(données, colWidths=colonnes, repeatRows=1)
    table.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, 0), ACCENT),
        ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, FOND]),
        ("GRID", (0, 0), (-1, -1), 0.4, FILET),
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("LEFTPADDING", (0, 0), (-1, -1), 6),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
    ]))
    return table


def _monospace(texte):
    """Les mots entre accents graves passent en chasse fixe."""
    return re.sub(r"`([^`]+)`", r"<font face='Courier'>\1</font>", texte)


def _verifie_les_glyphes(blocs):
    """Tout caractère du guide existe-t-il dans la police du PDF ?

    Les polices de base d'un PDF — Helvetica, Courier — sont encodées en
    WinAnsi, qui ne connaît ni la flèche pleine « U+25B8 » ni les traits
    d'arborescence. reportlab ne s'en plaint pas : il pose un carré noir
    à la place, et l'on ne s'en aperçoit qu'en relisant le PDF fini.
    Mieux vaut refuser de l'écrire.

    C'est la même leçon que dans le programme, où un test vérifie que
    chaque glyphe de l'interface existe dans sa police : une police qui
    n'a pas le caractère demandé ne le dit pas, elle dessine autre chose.
    """
    fautifs = {}

    def regarde(texte):
        texte = str(texte)
        for c in texte:
            try:
                c.encode("cp1252")
            except UnicodeEncodeError:
                fautifs.setdefault(c, texte[:60])

    for bloc in blocs:
        corps = bloc[1]
        if isinstance(corps, list):
            for élément in corps:
                if isinstance(élément, list):
                    for case in élément:
                        regarde(case)
                else:
                    regarde(élément)
        else:
            regarde(corps)
    if fautifs:
        détails = ", ".join(
            "%r dans « %s… »" % (c, t) for c, t in sorted(fautifs.items())
        )
        raise SystemExit(
            "le guide emploie des caractères absents de la police du PDF : " + détails
        )


def main():
    v = version()
    total, avec_but, paliers = exercices()
    total_atelier, atelier_paliers = atelier()
    blocs = contenu(v, total, avec_but, paliers, fiches(), tests(),
                    fonctions_de_la_calculatrice(), total_atelier, atelier_paliers)
    # Les raccourcis, en dernier chapitre : ils viennent du code.
    table = [["Action", "Raccourci"]]
    for action, touches in raccourcis():
        table.append([_lisible(action), touches])
    blocs.append(("table", table))
    blocs.append(("h1", "Où vivent les fichiers"))
    blocs.append(("table", [
        ["Chemin", "Ce qu'il contient"],
        ["~/.local/bin/namsdec", "le programme"],
        ["~/.config/namsdec/settings.json", "tous vos réglages, dont `donation_url`"],
        ["~/.config/namsdec/keymap.json", "le jeu de raccourcis actif"],
        ["~/.config/namsdec/keymaps/", "les jeux de raccourcis enregistrés"],
        ["~/.local/share/namsdec/exercises/", "le parcours et vos séries"],
        ["~/.local/share/namsdec/themes/", "les thèmes installés"],
        ["~/.local/share/namsdec/modules/", "les outils extérieurs déclarés"],
    ]))
    blocs.append(("h1", "Ce que namsdec ne fait pas"))
    blocs.append(("liste", [
        "Pas de mode 32 bits, ni d'interruption `int 0x80` : seul le x86-64 est simulé.",
        "Pas de bibliothèque C <i>chargée</i> : namsdec n'est pas un éditeur de liens "
        "dynamique, et le code de `printf` n'est nulle part dans la mémoire simulée. "
        "À la place il en <b>fournit cent soixante-dix</b>, écrites en Rust — afficher, "
        "lire, découper et comparer des chaînes, classer des caractères, demander de la "
        "mémoire. Un binaire qui emprunte autre chose s'arrête en <b>nommant</b> la "
        "fonction ; un binaire lié statiquement (`gcc -static`) n'emprunte rien et "
        "s'exécute entier.",
        "Pas de système de fichiers : `fopen` répond « introuvable », ce qui est la "
        "vérité — il n'y a pas de fichier — et laisse le programme suivre sa branche "
        "d'erreur, souvent la moitié intéressante. La console le dit la première fois.",
        "Pas de processus ni de bibliothèque chargée à l'exécution : `fork`, `execvp`, "
        "`dlopen` restent refusés en se nommant, et le resteront.",
        "Les appels système qui touchent au monde — `mmap`, `open`, `fork` — sont nommés "
        "mais pas exécutés. namsdec le dit dès l'assemblage, avec la ligne.",
        "AVX-512 : ses <b>trente-deux registres de 512 bits</b> (`zmm`), ses huit <b>registres de masque</b> (`k0` à `k7`) et les instructions qui les remplissent, et l'<b>application</b> du masque — les voies retenues reçoivent le résultat, les autres gardent leur valeur ou tombent à zéro. Une instruction dont namsdec ne sait pas nommer l'élément s'arrête en le disant, plutôt que de décaler les voies et de rendre un résultat faux sans avertissement. FMA, `in`/`out` et `cmpxchg16b` ne sont pas implémentés ; SSE, SSE2, SSSE3, SSE4.1, SSE4.2, AVX et AVX2 le sont entièrement.",
    ]))
    blocs.append(("h1", "Signaler une anomalie"))
    blocs.append(("p",
        "<b>Aide › Signaler une anomalie…</b> rassemble le contexte d'un défaut — version, "
        "système, outils trouvés, forme du fichier ouvert, et ce que namsdec disait au "
        "moment du clic. C'est la moitié du travail qui coûte du temps à qui signale, et "
        "c'est elle qui fait qu'une anomalie n'est jamais signalée."))
    blocs.append(("liste", [
        "<b>Rien ne part tout seul.</b> Aucune requête n'est faite en ouvrant la fenêtre. "
        "Le texte reste sur la machine tant qu'on n'a pas cliqué.",
        "<b>Ce qui part est montré avant</b>, en entier, dans un champ qu'on lit et qu'on "
        "corrige. Un rapport qu'on n'a pas lu est un rapport qu'on n'a pas consenti à "
        "envoyer.",
        "<b>Ni le fichier, ni son chemin.</b> Un binaire analysé est souvent ce qu'on a de "
        "plus privé — un échantillon, un travail en cours, le code de quelqu'un d'autre —, "
        "et un rapport part sur un tracker public. Le rapport dit sa FORME — « ELF64 "
        "x86-64 · PIE · dyn · dépouillé, 84&nbsp;548 octets » — et jamais son nom ni son "
        "contenu. Une case, décochée, joint le chemin pour qui le veut vraiment.",
        "<b>Deux sorties, les deux au clic</b> : copier le rapport dans le presse-papiers, "
        "ou ouvrir le formulaire du dépôt avec le titre et le corps déjà remplis. namsdec "
        "n'héberge pas de serveur, et ce qu'il met dans l'adresse est exactement ce que la "
        "fenêtre montrait.",
    ]))
    blocs.append(("h1", "Mises à jour, licence et contact"))
    blocs.append(("p",
        "« À propos › Rechercher une mise à jour » interroge les versions publiées sur "
        "GitHub et compare avec la vôtre, préversions comprises. Rien n'est téléchargé ni "
        "installé sans vous : namsdec ouvre la page de la version."))
    blocs.append(("p",
        "namsdec est publié sous licence <b>MIT</b> : ce que vous comprenez ici est fait "
        "pour être transmis. Le site est <b>namsdec.io</b>, le code et les tickets sont sur "
        "GitHub, et la boîte « À propos » mène aux deux ainsi qu'au salon Discord."))

    _verifie_les_glyphes(blocs)
    sortie = RACINE / "docs" / ("guide-namsdec-%s.pdf" % v)
    rendre(sortie, v, blocs)
    print("écrit : %s" % sortie)
    return 0


def _lisible(action):
    """Le nom d'une action, tel qu'on le dit."""
    noms = {
        "ToggleExplorer": "Explorateur de fichiers", "Assemble": "Assembler",
        "Run": "Exécuter", "StepInto": "Pas à pas détaillé",
        "StepOver": "Pas à pas principal", "StepBack": "Pas en arrière",
        "Stop": "Arrêter", "Reset": "Remettre à zéro", "Save": "Enregistrer",
        "OpenFile": "Ouvrir un fichier", "OpenFolder": "Ouvrir un dossier",
        "InstructionHelp": "Aide sur l'instruction", "RunToCursor": "Jusqu'au curseur",
        "RunToCall": "Jusqu'au prochain appel", "RunToReturn": "Jusqu'au prochain retour",
        "RunToSyscall": "Jusqu'au prochain appel système",
        "GoToLabel": "Aller à l'étiquette", "Find": "Chercher", "New": "Nouveau fichier",
        "ShowFlow": "Déroulement du programme", "NextTab": "Fichier suivant",
        "PreviousTab": "Fichier précédent", "CloseTab": "Fermer le fichier",
        "ToggleCalculator": "Calculatrice",
    }
    return noms.get(action, action)


if __name__ == "__main__":
    sys.exit(main())
