// Le choix du thème : sombre ou clair, et il se retient.
//
// Vingt lignes, sans bibliothèque. Au premier chargement, on suit ce que
// le système demande (`prefers-color-scheme`) ; dès qu'on clique, c'est
// le choix qui gagne, et il est gardé dans le navigateur.
//
// L'attribut est posé sur <html> AVANT que la page ne se peigne — le
// script est chargé dans <head> sans `defer` exprès. Posé après, on
// verrait le site clignoter du sombre vers le clair à chaque
// chargement.

(function () {
  const CLE = "namsdec-theme";
  const racine = document.documentElement;

  const systeme = () =>
    window.matchMedia("(prefers-color-scheme: light)").matches ? "clair" : "sombre";

  const poser = (theme) => {
    racine.setAttribute("data-theme", theme);
    const bouton = document.getElementById("bascule-theme");
    if (!bouton) return;
    // Le bouton annonce ce qu'un clic FERA, et non l'état courant :
    // c'est ce qu'on lit sur un interrupteur.
    const versClair = theme === "sombre";
    bouton.textContent = versClair ? "☀" : "☾";
    bouton.setAttribute("aria-label", bouton.dataset[versClair ? "clair" : "sombre"] || "");
    bouton.setAttribute("title", bouton.getAttribute("aria-label"));
  };

  poser(localStorage.getItem(CLE) || systeme());

  document.addEventListener("DOMContentLoaded", () => {
    poser(racine.getAttribute("data-theme"));
    const bouton = document.getElementById("bascule-theme");
    if (!bouton) return;
    bouton.addEventListener("click", () => {
      const neuf = racine.getAttribute("data-theme") === "sombre" ? "clair" : "sombre";
      localStorage.setItem(CLE, neuf);
      poser(neuf);
    });
  });
})();

// Copier la somme de contrôle.
//
// Une somme qu'on ne peut pas copier ne sert à personne : soixante-quatre
// caractères hexadécimaux ne se retapent pas, et une sélection à la
// souris en oublie toujours un bout. Le bouton dit ce qu'il a fait, et
// le redit en clair pour les lecteurs d'écran.
document.addEventListener("DOMContentLoaded", () => {
  for (const bouton of document.querySelectorAll(".copier")) {
    bouton.addEventListener("click", async () => {
      const cible = document.getElementById(bouton.dataset.pour);
      if (!cible) return;
      const texte = cible.textContent.trim();
      try {
        await navigator.clipboard.writeText(texte);
      } catch {
        // Le presse-papiers est refusé — hors HTTPS, par exemple. On
        // sélectionne alors le texte : il reste un Ctrl+C à faire, mais
        // la ligne est prise en entier et rien n'est perdu.
        const plage = document.createRange();
        plage.selectNodeContents(cible);
        const choix = window.getSelection();
        choix.removeAllRanges();
        choix.addRange(plage);
        bouton.textContent = bouton.dataset.selection || "Selected";
        return;
      }
      const avant = bouton.textContent;
      bouton.textContent = bouton.dataset.fini || "Copied";
      bouton.setAttribute("data-fait", "");
      setTimeout(() => {
        bouton.textContent = avant;
        bouton.removeAttribute("data-fait");
      }, 1600);
    });
  }
});

// Télécharger : la somme part dans le presse-papiers, et on le dit.
//
// Vérifier une somme suppose de l'avoir sous la main AU MOMENT où l'on
// a le fichier. La chercher après coup, c'est remonter sur le site,
// sélectionner soixante-quatre caractères et espérer n'en pas perdre un
// — autant dire ne pas vérifier. Elle est donc copiée au clic, et une
// infobulle dit ce qui vient de se passer, parce qu'une copie
// silencieuse ne se distingue pas d'une absence de copie.
//
// Le téléchargement n'est pas retenu : le navigateur le lance, la page
// reste, et l'infobulle avec elle.
document.addEventListener("DOMContentLoaded", () => {
  const somme = document.getElementById("sha256");
  const boutons = document.querySelectorAll("a[data-somme]");
  if (!somme || !boutons.length) return;
  const valeur = somme.textContent.trim();

  const montrer = (ancre, texte) => {
    document.querySelectorAll(".bulle").forEach((b) => b.remove());
    const bulle = document.createElement("div");
    bulle.className = "bulle";
    bulle.setAttribute("role", "status");
    bulle.innerHTML = `${texte}<code>${valeur}</code>`;
    document.body.appendChild(bulle);
    const cadre = ancre.getBoundingClientRect();
    // Sous le bouton, et ramenée dans la fenêtre si elle en sortait.
    const gauche = Math.min(
      cadre.left + window.scrollX,
      window.scrollX + document.documentElement.clientWidth - bulle.offsetWidth - 12,
    );
    bulle.style.top = `${cadre.bottom + window.scrollY + 8}px`;
    bulle.style.left = `${Math.max(window.scrollX + 12, gauche)}px`;
    requestAnimationFrame(() => bulle.setAttribute("data-vue", ""));
    setTimeout(() => {
      bulle.removeAttribute("data-vue");
      setTimeout(() => bulle.remove(), 250);
    }, 6000);
  };

  for (const ancre of boutons) {
    // L'infobulle du navigateur aussi : elle se lit au survol, avant
    // même d'avoir cliqué, et elle survit à tout refus du presse-papiers.
    ancre.setAttribute("title", `${ancre.dataset.somme}\n${valeur}`);
    ancre.addEventListener("click", () => {
      if (!navigator.clipboard) {
        montrer(ancre, ancre.dataset.sansCopie || ancre.dataset.somme);
        return;
      }
      navigator.clipboard.writeText(valeur).then(
        () => montrer(ancre, ancre.dataset.copie || ancre.dataset.somme),
        () => montrer(ancre, ancre.dataset.sansCopie || ancre.dataset.somme),
      );
    });
  }
});
