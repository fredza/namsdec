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

def tests():
    """Combien de tests automatiques accompagnent le programme."""
    total = 0
    for source in (RACINE / "crates").rglob("*.rs"):
        total += len(re.findall(r"^\s*#\[test\]", source.read_text(encoding="utf-8"), re.M))
    return total


def fiches():
    texte = (RACINE / "crates/namsdec-doc/src/table.rs").read_text(encoding="utf-8")
    return texte.count("kind: Kind::")

def raccourcis():
    """Les raccourcis par défaut, lus dans le code qui les pose."""
    texte = (RACINE / "crates/namsdec-gui/src/keybindings.rs").read_text(encoding="utf-8")
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


def _touche(nom):
    """Le nom d'une touche, tel qu'on le dit en français."""
    return {
        "CTRL": "Ctrl", "SHIFT": "Maj", "ALT": "Alt", "COMMAND": "Cmd",
        "Space": "Espace", "Escape": "Échap",
    }.get(nom, nom)


# --- Le contenu ----------------------------------------------------------

def contenu(v, total_exos, avec_but, paliers, nb_fiches, nb_tests):
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

        ("h1", "Le premier programme"),
        ("p",
         "Au premier lancement, l'éditeur contient un programme d'accueil. Trois gestes "
         "suffisent pour le voir tourner :"),
        ("liste", [
         "<b>Assembler</b> (Ctrl+Maj+B) : nasm et ld font leur travail. Les erreurs "
         "apparaissent dans l'onglet Diagnostics, chacune cliquable vers sa ligne.",
         "<b>Pas à pas</b> (Espace puis s puis i) : une instruction à la fois. Le bandeau "
         "du panneau droit montre ce qui vient, puis ce qui a changé.",
         "<b>Exécuter</b> (Espace puis r) : jusqu'au bout, ou jusqu'au premier point "
         "d'arrêt.",
        ]),
        ("p",
         "Le programme s'arrête sur <font face='Courier'>hlt</font> ou sur l'appel système "
         "<font face='Courier'>exit</font>. Un programme que rien n'arrête est signalé dès "
         "l'assemblage : il continuerait dans ce qui suit son code, ce qui n'a aucun sens."),

        ("h1", "La fenêtre"),
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
         "Plusieurs fichiers peuvent être ouverts à la fois : une barre d'onglets apparaît "
         "au-dessus de l'éditeur, chacun gardant son texte, son curseur, ses points d'arrêt "
         "et son exécution en cours. Ctrl+Tab passe au suivant, Ctrl+W ferme."),

        ("h2", "Le panneau de droite"),
        ("p", "Il se lit de haut en bas, et se partage en trois zones fixes :"),
        ("liste", [
         "<b>Prochaine instruction</b> — l'adresse, le texte, le pseudo-code "
         "(<font face='Courier'>RAX = RAX + RCX</font> plutôt que "
         "<font face='Courier'>add rax, rcx</font>), les octets, et le détail des opérandes : "
         "l'adresse effective qu'un calcul désigne, et l'octet qui s'y trouve.",
         "<b>État du processeur</b> — seize registres, RIP, les six indicateurs en "
         "pastilles, le nombre d'instructions exécutées. Un clic sur un nom de registre pose "
         "une surveillance ; un clic sur une valeur permet de la remplacer.",
         "<b>La vue choisie</b> — Pile, Mémoire, Tas, Octets, et RE si le module est activé. "
         "Les onglets sont en haut à droite, dans la barre d'outils.",
        ]),
        ("p",
         "La séparation entre le bandeau et la vue se règle à la poignée, et Affichage ▸ "
         "« Panneau latéral partagé tout seul » rend le partage automatique."),

        ("h2", "La console"),
        ("p", "En bas, trois onglets :"),
        ("liste", [
         "<b>Console</b> — ce que le programme écrit lui-même, par "
         "<font face='Courier'>write</font>. Le champ de saisie n'apparaît que si le "
         "programme contient un <font face='Courier'>read</font> : inutile de proposer une "
         "entrée à qui ne lira jamais.",
         "<b>Diagnostics</b> — ce que nasm a eu à dire, les avertissements que namsdec "
         "ajoute, et les deux commandes réellement lancées, prêtes à copier.",
         "<b>Profil</b> — les lignes classées par temps passé dedans : passages, part du "
         "total, et un clic qui y emmène.",
        ]),

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
         "(Outils ▸ Chronologie) permet de cliquer un pas passé pour y revenir."),

        ("h2", "Les points d'arrêt"),
        ("p",
         "Un clic dans la gouttière pose un point d'arrêt. Il peut porter une "
         "<b>condition</b> — <font face='Courier'>rcx == 0</font> — auquel cas l'exécution "
         "ne s'arrête que si elle est vraie. La fenêtre des points d'arrêt les liste tous, "
         "avec leur condition et le nombre de fois qu'ils ont arrêté le programme."),
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
        ("p",
         "L'adresse s'écrit en hexadécimal, en décimal, ou par un nom de registre : taper "
         "<font face='Courier'>rsi</font> et valider suit le pointeur. « Suivre » fixe la "
         "vue sur RSP ou RIP pas à pas."),
        ("h2", "Les outils de mémoire"),
        ("liste", [
         "<b>Carte de la mémoire</b> — les zones et leurs droits, à la façon de `vmmap`.",
         "<b>Chercher en mémoire</b> — un texte, des octets ou une valeur.",
         "<b>Écrire ici</b> — poser une valeur à la main, pour essayer une hypothèse.",
        ]),
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
         "Une garantie que peu d'outils donnent : un test parcourt soixante binaires de "
         "votre système et vérifie qu'il n'y reste <b>pas une seule instruction</b> que le "
         "manuel ne sache expliquer."),
        ("h2", "Le déroulement du programme"),
        ("p",
         "<b>Espace puis g</b> ouvre le déroulement : le graphe des blocs. Un listing se lit "
         "de haut en bas ; un programme, non. Chaque boîte est une suite d'instructions "
         "qu'on ne peut qu'entrer par le haut et quitter par le bas ; les flèches disent le "
         "saut pris (vert), le saut non pris (rouge) et l'appel (bleu) ; une boucle se voit "
         "à ce qu'elle remonte. Le bloc où le programme se trouve est mis en avant, un clic "
         "y emmène dans l'éditeur, et Ctrl+molette recule pour voir la forme d'ensemble."),
        ("p",
         "Un saut dont la cible se calcule — table de sauts, pointeur de fonction — est "
         "signalé comme tel : aucune flèche n'est inventée vers une adresse inconnue."),
        ("h2", "Le profil"),
        ("p",
         "La bande de chaleur dit <i>où</i> ça chauffe ; le profil dit <i>de combien</i>. "
         "Il classe les lignes par nombre de passages, donne la part du total, nomme le bloc "
         "de chaque ligne, et se copie en texte. Quand un exercice impose un budget "
         "d'instructions, le profil le rappelle et désigne la ligne par où reprendre."),

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
         "La plupart portent aussi un <b>budget</b> : le nombre d'instructions exécutées "
         "qu'il faut tenir. Il est <i>mesuré</i>, jamais estimé — un test exécute les %d "
         "exercices et vérifie que chacun atteint son but dans son budget. Un programme "
         "juste peut être deux fois plus court, et c'est en cherchant comment qu'on apprend "
         "<font face='Courier'>lea</font>, les décalages, ou qu'une boucle se remplace par "
         "une formule." % total_exos),
        ("note",
         "Le module est éteint par défaut : Préférences ▸ Modules l'allume. Chaque exercice "
         "ouvert prend son propre onglet, sous son titre — on passe au suivant sans "
         "abandonner ce qu'on a écrit dans le précédent."),
        ("p",
         "Le parcours <b>n'est pas dans le binaire</b> : c'est un fichier de données, "
         "<font face='Courier'>~/.local/share/namsdec/exercises/namsdec.json</font>. "
         "Corriger un énoncé ne demande donc pas de recompiler, et une série que vous "
         "écrivez s'installe exactement de la même façon (Outils ▸ Exercices)."),

        ("h1", "Lire un binaire qu'on n'a pas écrit"),
        ("p",
         "Fichier ▸ Ouvrir accepte un ELF64 : ses sections vont à leur adresse, son point "
         "d'entrée est celui qu'il déclare, et seul le code est désassemblé. Le listing "
         "obtenu remplit l'éditeur, une instruction par ligne — ce qui donne aussi la "
         "correspondance ligne/adresse dont dépendent la gouttière, le suivi de RIP et les "
         "points d'arrêt."),
        ("p",
         "Le module <b>RE</b> (Préférences ▸ Modules) ajoute un onglet et une fenêtre "
         "d'analyse, où tout se lit déplié :"),
        ("liste", [
         "<b>Protections</b> — PIE, NX, RELRO, canari de pile, FORTIFY, IBT et pile "
         "fantôme, segments W+X, symboles, bibliothèques, RPATH — avec, pour chacune, ce "
         "que son état change concrètement.",
         "<b>Fonctions empruntées</b> — chaque entrée de la table de liaison retrouve son "
         "nom, avec le nombre d'appels qui y mènent et une phrase d'explication pour les "
         "plus courantes.",
         "<b>Appels système</b> — le numéro remonté depuis le RAX qui précède, et le nom "
         "qui va avec.",
         "<b>Chaînes</b> — cherchées dans les sections de données, comme "
         "<font face='Courier'>strings -d</font>, avec la possibilité de ne montrer que "
         "celles que le code désigne.",
         "<b>Références croisées</b> — qui appelle quoi, qui saute où, qui désigne quelle "
         "adresse. Une adresse qu'un `call` vise est une fonction, même dépouillée de son "
         "nom.",
         "<b>Gadgets</b> — les fins de séquence utiles à une chaîne ROP.",
        ]),
        ("p",
         "Le <b>motif cyclique</b> (Outils ▸ Motif cyclique) est celui de pwntools, octet "
         "pour octet : on l'envoie au programme, et le décalage relu depuis un registre dit "
         "exactement de combien un débordement a dépassé."),
        ("p",
         "Le rapport d'analyse se copie en Markdown ou s'enregistre, pour être collé dans "
         "un compte rendu."),

        ("h1", "Sortir de l'atelier"),
        ("p",
         "Fichier ▸ <b>Exporter le binaire</b> écrit le programme tel que "
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
         "├── themes/          les thèmes\n"
         "├── decompilers/     les désassembleurs et décompilateurs\n"
         "└── exercises/       les séries d'exercices"),
        ("p",
         "Installer, c'est copier un fichier JSON dans le bon dossier. Depuis namsdec : "
         "Préférences ▸ Plugins pour un thème, Préférences ▸ Outils pour un décompilateur. Le "
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
        ]),
        ("note",
         "<b>Rizin seul ne décompile pas</b> : la commande <font face='Courier'>pdg</font> vient "
         "du greffon <font face='Courier'>rz-ghidra</font>. Là où il n'est pas empaqueté, "
         "<font face='Courier'>rz-pm -i rz-ghidra</font> l'installe. namsdec vérifie sa présence "
         "et affiche « incomplet » plutôt que de vous laisser essayer."),
        ("p",
         "Le module reste éteint tant qu'on ne l'allume pas (Préférences ▸ Modules), l'entrée de "
         "menu n'apparaît qu'ensuite, et rien ne part sans le bouton « Lancer ». Passé deux "
         "minutes, un outil qui ne rend pas la main est arrêté."),

        ("h1", "Régler namsdec"),
        ("p",
         "Les préférences couvrent le thème, la langue (français ou anglais), les modules, "
         "l'affichage et les raccourcis — chacun réattribuable. <b>Tout réglage est "
         "enregistré</b> dans <font face='Courier'>~/.config/namsdec/settings.json</font>, "
         "un fichier lisible et modifiable à la main."),
        ("p",
         "Des thèmes et des désassembleurs supplémentaires s'installent comme les séries "
         "d'exercices : un fichier déposé dans "
         "<font face='Courier'>~/.local/share/namsdec/</font>. Un désassembleur externe est "
         "un <i>programme</i> : il ne s'exécute que sur demande, et le panneau annonce la "
         "commande avant de la lancer."),

        ("h1", "Les raccourcis"),
        ("p",
         "Les séquences notées « Espace puis … » sont des touches relâchées l'une après "
         "l'autre, à la façon de la touche « leader » de vim. Elles ne s'arment que lorsque "
         "l'éditeur n'a pas le focus, sinon un espace tapé dans le code déclencherait tout."),
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


def main():
    v = version()
    total, avec_but, paliers = exercices()
    blocs = contenu(v, total, avec_but, paliers, fiches(), tests())
    # Les raccourcis, en dernier chapitre : ils viennent du code.
    table = [["Action", "Raccourci"]]
    for action, touches in raccourcis():
        table.append([_lisible(action), touches])
    blocs.append(("table", table))
    blocs.append(("h1", "Où vivent les fichiers"))
    blocs.append(("table", [
        ["Chemin", "Ce qu'il contient"],
        ["~/.local/bin/namsdec", "le programme"],
        ["~/.config/namsdec/settings.json", "tous vos réglages"],
        ["~/.local/share/namsdec/exercises/", "le parcours et vos séries"],
        ["~/.local/share/namsdec/themes/", "les thèmes installés"],
        ["~/.local/share/namsdec/disassemblers/", "les désassembleurs externes"],
    ]))
    blocs.append(("h1", "Ce que namsdec ne fait pas"))
    blocs.append(("liste", [
        "Pas de mode 32 bits, ni d'interruption `int 0x80` : seul le x86-64 est simulé.",
        "Pas de bibliothèque C : un binaire qui appelle `printf` s'arrête sur l'appel, "
        "faute de code à exécuter derrière.",
        "Les appels système qui touchent au monde — `mmap`, `open`, `fork` — sont nommés "
        "mais pas exécutés. namsdec le dit dès l'assemblage, avec la ligne.",
        "AVX-512, FMA, `in`/`out` et `cmpxchg16b` ne sont pas implémentés.",
    ]))
    blocs.append(("h1", "Mises à jour, licence et contact"))
    blocs.append(("p",
        "« À propos ▸ Rechercher une mise à jour » interroge les versions publiées sur "
        "GitHub et compare avec la vôtre, préversions comprises. Rien n'est téléchargé ni "
        "installé sans vous : namsdec ouvre la page de la version."))
    blocs.append(("p",
        "namsdec est publié sous licence <b>MIT</b> : ce que vous comprenez ici est fait "
        "pour être transmis. Le site est <b>namsdec.io</b>, le code et les tickets sont sur "
        "GitHub, et la boîte « À propos » mène aux deux ainsi qu'au salon Discord."))

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
    }
    return noms.get(action, action)


if __name__ == "__main__":
    sys.exit(main())
