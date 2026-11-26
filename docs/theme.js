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
