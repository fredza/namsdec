#!/usr/bin/env python3
"""Remet le téléchargement du site à jour : le lien, et la somme.

POURQUOI UN SCRIPT. Le bouton « Télécharger » mène au BINAIRE, pas à une
page de versions où il faudrait encore chercher — c'est ce qu'on veut
pour quelqu'un qui découvre. Mais un lien vers un binaire porte le
numéro de version, et la somme SHA-256 change à chaque publication.
Deux nombres écrits dans deux pages, qu'il faudrait penser à changer :
c'est précisément ce qu'on oublie.

Ils se remettent donc d'une commande, après chaque publication :

    python3 docs/maj-telechargement.py

Le numéro vient de `Cargo.toml`, la somme de la version publiée sur
GitHub. Rien n'est inventé ici. Un contrôle (`le_site_ne_montre_rien_
qui_manque`) refuse que le lien désigne une autre version que celle
qu'on compile ; il ne peut pas vérifier la somme, qui n'existe qu'en
ligne — d'où ce script, qui va la chercher.

    --hors-ligne   ne met à jour que le lien, et laisse la somme
"""

import re
import sys
import urllib.request
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
DÉPÔT = "https://github.com/fredza/namsdec"
PAGES = ["docs/index.html", "docs/en.html"]


def version():
    """Le numéro de l'espace de travail, seule source qui fasse foi."""
    texte = (RACINE / "Cargo.toml").read_text(encoding="utf-8")
    trouvé = re.search(r'^version\s*=\s*"([^"]+)"', texte, re.M)
    if not trouvé:
        raise SystemExit("version introuvable dans Cargo.toml")
    return trouvé.group(1)


def somme_publiée(v, binaire):
    """La somme que la publication porte, lue sur GitHub."""
    url = f"{DÉPÔT}/releases/download/v{v}/{binaire}.sha256"
    try:
        with urllib.request.urlopen(url, timeout=30) as réponse:
            ligne = réponse.read().decode("utf-8", "replace").strip()
    except Exception as souci:
        raise SystemExit(
            f"la somme de v{v} ne se lit pas ({souci}).\n"
            "La version est-elle publiée ? Sinon, `--hors-ligne` met le lien "
            "à jour et laisse la somme telle quelle."
        )
    premier = ligne.split()[0] if ligne.split() else ""
    if len(premier) != 64 or not all(c in "0123456789abcdef" for c in premier):
        raise SystemExit(f"ce que GitHub rend n'est pas une somme : {ligne!r}")
    return premier


def main():
    v = version()
    binaire = f"namsdec-{v}-x86_64-linux"
    lien = f"{DÉPÔT}/releases/download/v{v}/{binaire}"
    somme = None if "--hors-ligne" in sys.argv else somme_publiée(v, binaire)

    for chemin in PAGES:
        p = RACINE / chemin
        texte = p.read_text(encoding="utf-8")
        avant = texte
        # Le lien, où qu'il soit : la barre et le haut de page.
        texte = re.sub(
            re.escape(DÉPÔT) + r"/releases/download/v[^/\"]+/namsdec-[^\"]+",
            lien,
            texte,
        )
        # Le nom du fichier, dans la phrase qui dit de quoi est la somme.
        texte = re.sub(r"namsdec-[0-9][^<\" ]*-x86_64-linux", binaire, texte)
        if somme:
            texte = re.sub(
                r'(<code id="sha256">)[0-9a-f]{64}(</code>)',
                r"\g<1>" + somme + r"\g<2>",
                texte,
            )
        p.write_text(texte, encoding="utf-8")
        print(("inchangé : " if texte == avant else "mis à jour : ") + chemin)

    print(f"version {v}")
    print(f"lien     {lien}")
    print("somme    " + (somme if somme else "laissée telle quelle (--hors-ligne)"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
