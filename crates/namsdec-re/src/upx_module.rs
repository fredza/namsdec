//! UPX : le reconnaître, et rendre l'original.
//!
//! Le panneau « Entropie » nomme déjà l'empaqueteur quand il se nomme —
//! une section `UPX0`, le nombre magique `UPX!` — et dit quoi en faire :
//! « `upx -d` rend l'original ». C'est juste, et cela s'arrêtait là. Il
//! fallait sortir de namsdec, retrouver le chemin du fichier, taper la
//! commande, et revenir. Entre les deux, la moitié des gens tape
//! `upx -d fichier` **sur son échantillon**, qui est écrasé en place.
//!
//! Ce panneau fait les deux gestes, et **aucun ne touche le fichier
//! ouvert** :
//!
//! - `upx -l` : ce qu'UPX voit. Lecture seule, rien n'est écrit.
//! - `upx -d -o <nouveau> <copie>` : l'original déballé, dans un
//!   fichier **qu'on choisit**. L'entrée est une copie temporaire, et
//!   la sortie un fichier neuf — l'échantillon reste tel qu'il est
//!   arrivé, ce qui est la première règle quand on analyse ce qu'on n'a
//!   pas écrit.
//!
//! Les mêmes trois règles que les autres rubriques d'outil, et elles
//! sont le sujet de ce module autant que la sortie qu'il affiche :
//! **rien ne s'exécute tout seul**, **la commande est annoncée avant
//! d'être lancée**, et **namsdec n'installe rien à votre place**.
//!
//! CE QUE LE PANNEAU DIT QUAND IL NE TROUVE RIEN compte autant que le
//! reste. Pas de marque `UPX!` ne veut pas dire « pas d'UPX » : la
//! signature et les noms de section s'effacent en trois octets, et
//! beaucoup de protecteurs réels partent d'UPX puis les effacent
//! justement pour que `upx -d` refuse. Le panneau le dit, plutôt que de
//! laisser conclure à tort.

use namsdec_disasm::known::Availability;

use crate::{ReContext, ReModule, ReRequest, ReResult, ReSection, UpxAction, framed};

/// Combien de lignes de sortie le panneau peint.
///
/// `upx -l` en rend une dizaine et `upx -d` autant : il n'y a pas de
/// pagination à prévoir, seulement une borne pour qu'une sortie
/// inattendue ne fasse pas une page de dix mille lignes.
const LIGNES_MAX: usize = 200;

/// Le panneau d'UPX.
pub struct UpxModule {
    /// Les marques d'UPX dans le fichier ouvert, s'il y en a.
    ///
    /// La même reconnaissance que celle du panneau « Entropie » — c'est
    /// `packer::detect` qui la porte, et la refaire ici en donnerait
    /// une seconde à garder d'accord avec la première.
    marque: Option<crate::packer::Found>,
    /// La taille du fichier ouvert, pour comparer avant/après.
    taille: usize,
    /// Ce que namsdec sait d'`upx` sur cette machine. C'est
    /// l'application qui le dit : la question se pose en parcourant le
    /// PATH, ce qui ne se fait pas soixante fois par seconde.
    disponible: Availability,
    /// Le geste en cours, s'il y en a un.
    en_cours: Option<UpxAction>,
    /// Ce qu'UPX a répondu, découpé en lignes, et lequel des deux
    /// gestes l'a obtenu.
    sortie: Vec<String>,
    fait: Option<UpxAction>,
    /// Son refus, le cas échéant.
    erreur: Option<String>,
    /// Le fichier déballé, quand il y en a un.
    déballé: Option<std::path::PathBuf>,
    demande: Option<ReRequest>,
}

/// Le panneau part de « upx est absent ».
///
/// C'est la seule supposition prudente : l'application dit l'état réel
/// dès la première image, et partir de « prêt » aurait montré pendant un
/// instant deux boutons actifs qui n'auraient rien pu lancer.
impl Default for UpxModule {
    fn default() -> Self {
        Self {
            marque: None,
            taille: 0,
            disponible: Availability::Missing,
            en_cours: None,
            sortie: Vec::new(),
            fait: None,
            erreur: None,
            déballé: None,
            demande: None,
        }
    }
}

impl UpxModule {
    /// La sortie telle qu'elle est, pour qui veut la lire sans
    /// l'interface.
    pub fn lines(&self) -> &[String] {
        &self.sortie
    }

    fn poser(&mut self, action: UpxAction, texte: &str) {
        self.sortie = texte
            .lines()
            // La bannière de copyright d'UPX : trois lignes sur chaque
            // sortie, qui n'apprennent rien et repoussent le tableau
            // sous le bord. Ce qu'on veut lire est dessous.
            .filter(|ligne| !bannière(ligne))
            .map(str::to_owned)
            .take(LIGNES_MAX)
            .collect();
        self.fait = Some(action);
        self.erreur = None;
    }
}

/// CE QU'UN REFUS D'UPX VEUT DIRE, en français.
///
/// upx refuse en jetant le nom d'une exception C++ :
/// `CantUnpackException: e_phnum must be >= 2`. C'est exact, et cela
/// n'apprend rien — on le lit comme une panne de namsdec. Signalé sur
/// capture le 21 octobre 2026.
///
/// Les refus sont traduits, et la ligne d'origine reste SOUS la
/// traduction : c'est elle, la preuve, et c'est elle qu'on recopie dans
/// une recherche. On explique, on ne remplace pas.
///
/// Rend `None` sur ce qu'on ne sait pas traduire : inventer une
/// explication serait pire que la ligne brute.
pub fn refus_expliqué(sortie: &str, fr: bool) -> Option<&'static str> {
    let bas = sortie.to_ascii_lowercase();
    if !bas.contains("cantunpack") && !bas.contains("cantpack") && !bas.contains("exception") {
        return None;
    }
    // L'ordre compte : les motifs les plus précis d'abord.
    if bas.contains("not packed by upx") {
        return Some(if fr {
            "Ce fichier n'est pas empaqueté par UPX. Il n'y a donc rien à déballer — ce qui              est une réponse, pas une panne : l'entropie et les sections disent déjà d'où              vient le fichier."
        } else {
            "This file is not packed by UPX. There is nothing to unpack — which is an              answer, not a failure: entropy and sections already say where the file comes              from."
        });
    }
    if bas.contains("e_phnum") || bas.contains("bad e_phoff") || bas.contains("not a valid") {
        return Some(if fr {
            "upx n'a pas reconnu les en-têtes de ce fichier comme ceux d'un ELF qu'il aurait              empaqueté. C'est le cas ordinaire d'un fichier qui n'est tout simplement pas              empaqueté — regardez « Ce que porte ce fichier », juste au-dessus."
        } else {
            "upx did not recognise this file's headers as those of an ELF it would have              packed. That is the ordinary case of a file that simply is not packed — see              “What this file carries”, just above."
        });
    }
    if bas.contains("hacked") || bas.contains("modified") || bas.contains("tampered") {
        return Some(if fr {
            "upx voit un fichier empaqueté PUIS MODIFIÉ, et refuse de le déballer. C'est le              cas intéressant : beaucoup de protecteurs partent d'UPX puis effacent ses              marques, précisément pour que `upx -d` dise non. Le programme est bien              compressé — il se décompresse tout seul à l'exécution —, mais il faudra le              déballer autrement : en l'exécutant pas à pas jusqu'au saut vers le code              rendu, puis en vidant la mémoire."
        } else {
            "upx sees a file that was packed and THEN MODIFIED, and refuses to unpack it.              This is the interesting case: many protectors start from UPX then erase its              markers, precisely so that `upx -d` says no. The program is compressed — it              decompresses itself at run time — but it will have to be unpacked another              way: by stepping until the jump into the restored code, then dumping memory."
        });
    }
    if bas.contains("not executable") {
        return Some(if fr {
            "upx refuse un fichier qui n'a pas le droit d'exécution. namsdec lui passe une              copie temporaire : si ce message paraît, c'est la copie qui a perdu ses              droits, et c'est un défaut de namsdec, non du fichier."
        } else {
            "upx refuses a file without the execute bit. namsdec hands it a temporary copy:              if this message appears, the copy lost its permissions, and that is a defect              of namsdec, not of the file."
        });
    }
    Some(if fr {
        "upx a refusé. La ligne ci-dessous est la sienne, telle quelle : elle nomme          l'exception qu'il a levée, et c'est elle qu'on recopie dans une recherche."
    } else {
        "upx refused. The line below is its own, as-is: it names the exception it raised,          and that is what one copies into a search."
    })
}

/// Cette ligne est-elle la bannière d'UPX plutôt que sa réponse ?
///
/// Reconnue sur ce qu'elle contient et non sur son rang : `-q` la
/// supprimerait, mais il supprime aussi le tableau des tailles, qui est
/// précisément ce qu'on vient lire.
fn bannière(ligne: &str) -> bool {
    let nu = ligne.trim();
    nu.is_empty()
        || nu.starts_with("Ultimate Packer for eXecutables")
        || nu.starts_with("Copyright (C)")
        || nu.starts_with("UPX ") && nu.contains("Oberhumer")
}

impl ReModule for UpxModule {
    fn id(&self) -> &'static str {
        "upx"
    }

    /// Avec « Le fichier » : c'est une question qu'on se pose sur le
    /// fichier entier — « y a-t-il seulement le vrai programme ici ? »
    /// —, et elle se pose avant toute lecture de code. La rubrique suit
    /// « Entropie », qui l'a posée.
    fn section(&self) -> ReSection {
        ReSection::File
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "UPX — déballer l'original"
        } else {
            "UPX — unpack the original"
        }
    }

    fn nav_title(&self, _lang: &str) -> &'static str {
        "UPX"
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.taille = ctx.binary.len();
        self.marque = crate::packer::detect(ctx.binary)
            .into_iter()
            .find(|trouvé| trouvé.name == "UPX");
        // Un autre binaire : la sortie d'avant ne le concerne plus. La
        // laisser à l'écran ferait lire le rapport d'un programme en
        // croyant lire celui d'un autre.
        self.sortie.clear();
        self.fait = None;
        self.erreur = None;
        self.déballé = None;
        self.en_cours = None;
    }

    fn deliver(&mut self, result: &ReResult) {
        // Ce que namsdec sait d'`upx` sur cette machine arrive par le
        // même canal que pour les autres outils : l'application
        // parcourt le PATH une fois et le dit, plutôt que le panneau
        // soixante fois par seconde.
        if let ReResult::ToolState { tool, availability } = result {
            if tool == "upx" {
                self.disponible = *availability;
            }
            return;
        }
        let ReResult::UpxOutput {
            action,
            text,
            unpacked,
        } = result
        else {
            return;
        };
        self.en_cours = None;
        if let Some(chemin) = unpacked {
            self.déballé = Some(chemin.clone());
        }
        match text {
            Ok(texte) => self.poser(*action, texte),
            Err(message) => {
                self.erreur = Some(message.clone());
                self.sortie.clear();
                self.fait = None;
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.weak(ctx.t(
            "Deux gestes, et aucun ne touche le fichier ouvert : demander à upx ce \
             qu'il voit, et écrire l'original déballé dans un fichier neuf.",
            "Two actions, and neither touches the open file: ask upx what it sees, \
             and write the unpacked original into a new file.",
        ));

        ui.add_space(8.0);
        self.ce_que_porte_le_fichier(ui, ctx);
        ui.add_space(8.0);
        self.où_en_est_upx(ui, ctx);
        ui.add_space(8.0);
        self.les_deux_gestes(ui, ctx);
        ui.add_space(8.0);
        self.ce_qu_upx_a_dit(ui, ctx);
    }
}

impl UpxModule {
    /// Ce que le fichier porte : la marque, ou son absence — dite comme
    /// une absence et non comme une preuve.
    fn ce_que_porte_le_fichier(&self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.strong(ctx.t("Ce que porte ce fichier", "What this file carries"));
        ui.add_space(2.0);
        match &self.marque {
            Some(trouvé) => {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(
                        egui::RichText::new("UPX").color(egui::Color32::from_rgb(0xf4, 0x87, 0x71)),
                    );
                    // La preuve avec le nom, toujours : « c'est de
                    // l'UPX » sans dire pourquoi demande de faire
                    // confiance, et un atelier d'apprentissage n'a pas
                    // à demander ça. Elle se vérifie à la main, dans le
                    // vidage.
                    ui.monospace(trouvé.clue.label(ctx.lang));
                });
                ui.indent("upx_marque", |ui| {
                    ui.weak(trouvé.text(ctx.lang));
                });
            }
            None => {
                ui.weak(ctx.t(
                    "Aucune marque d'UPX : ni section « UPX0 » ou « UPX1 », ni nombre \
                     magique « UPX! ».",
                    "No UPX mark: neither a “UPX0” or “UPX1” section, nor the “UPX!” \
                     magic number.",
                ));
                // CE QUI COMPTE AUTANT QUE LA TROUVAILLE. Trois octets
                // effacés, et la signature disparaît : beaucoup de
                // protecteurs réels partent d'UPX puis la retirent
                // précisément pour qu'`upx -d` refuse. Conclure
                // « pas d'UPX » d'une absence de marque ferait chercher
                // ailleurs pendant une soirée.
                ui.weak(ctx.t(
                    "Ce qui ne prouve pas l'absence d'UPX : la signature et les noms de \
                     section s'effacent en trois octets, et c'est la première chose qu'un \
                     protecteur fait pour qu'upx refuse de déballer. L'entropie, elle, ne \
                     s'efface pas — voyez « Entropie ». upx -l ci-dessous tranche : il lit \
                     les en-têtes de bloc, non la signature.",
                    "Which does not prove UPX is absent: the signature and the section \
                     names are erased in three bytes, and that is the first thing a \
                     protector does so that upx refuses to unpack. Entropy does not erase \
                     — see “Entropy”. upx -l below settles it: it reads the block headers, \
                     not the signature.",
                ));
            }
        }
    }

    /// upx est-il sur cette machine ? Et sinon, ce qu'il faut taper —
    /// namsdec ne l'installe pas à votre place.
    fn où_en_est_upx(&self, ui: &mut egui::Ui, ctx: &ReContext) {
        match self.disponible {
            Availability::Ready | Availability::Incomplete => {
                ui.horizontal_wrapped(|ui| {
                    ui.weak(ctx.t("upx est sur cette machine.", "upx is on this machine."));
                });
            }
            Availability::Missing => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    ctx.t(
                        "upx n'est pas sur cette machine.",
                        "upx is not on this machine.",
                    ),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.weak(ctx.t("Pour l'obtenir :", "To get it:"));
                    ui.add(
                        egui::Label::new(egui::RichText::new("sudo dnf install upx").monospace())
                            .selectable(true),
                    );
                });
                ui.weak(ctx.t(
                    "namsdec ne l'installe pas à votre place : exécuter du code au nom de \
                     quelqu'un n'est pas une commodité, c'est une décision.",
                    "namsdec does not install it for you: running code on someone's behalf \
                     is not a convenience, it is a decision.",
                ));
            }
        }
    }

    /// Les deux commandes, écrites avant d'être lançables.
    fn les_deux_gestes(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let lançable = self.disponible != Availability::Missing && self.en_cours.is_none();

        ui.strong(ctx.t("Les deux gestes", "The two actions"));
        ui.add_space(4.0);

        // 1. IDENTIFIER. Lecture seule, et c'est ce qui en fait le
        // premier : il répond « est-ce packé, et par quoi » sans rien
        // écrire nulle part.
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(lançable, framed(ui, ctx.t("Identifier", "Identify")))
                .on_hover_text(ctx.t(
                    "Demander à upx ce qu'il voit. Rien n'est écrit.",
                    "Ask upx what it sees. Nothing is written.",
                ))
                .clicked()
            {
                self.en_cours = Some(UpxAction::List);
                self.erreur = None;
                self.demande = Some(ReRequest::Upx {
                    action: UpxAction::List,
                });
            }
            ui.add(
                egui::Label::new(egui::RichText::new("upx -l <copie>").monospace())
                    .selectable(true),
            );
            ui.weak(ctx.t("— lecture seule", "— read only"));
        });

        ui.add_space(4.0);

        // 2. DÉBALLER, vers un fichier neuf. `-o` est ce qui distingue
        // ce bouton de la commande qu'on tape de mémoire : sans lui,
        // `upx -d` écrase l'échantillon en place, et l'on a perdu ce
        // qu'on analysait.
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(lançable, framed(ui, ctx.t("Déballer vers…", "Unpack to…")))
                .on_hover_text(ctx.t(
                    "Écrire l'original déballé dans un fichier neuf, et l'ouvrir dans un \
                     onglet.",
                    "Write the unpacked original into a new file, and open it in a tab.",
                ))
                .clicked()
            {
                self.en_cours = Some(UpxAction::Unpack);
                self.erreur = None;
                self.demande = Some(ReRequest::Upx {
                    action: UpxAction::Unpack,
                });
            }
            ui.add(
                egui::Label::new(egui::RichText::new("upx -d -o <choisi> <copie>").monospace())
                    .selectable(true),
            );
        });

        ui.add_space(4.0);
        ui.weak(ctx.t(
            "`<copie>` est une copie temporaire du binaire ouvert, et `<choisi>` le \
             fichier que vous nommez : votre échantillon n'est ni lu en écriture, ni \
             remplacé, ni déplacé. C'est tout l'écart avec `upx -d fichier`, qui écrase \
             en place. Aucun interpréteur de commandes n'est en jeu.",
            "`<copy>` is a temporary copy of the open binary, and `<chosen>` the file you \
             name: your sample is neither opened for writing, nor replaced, nor moved. \
             That is the whole difference from `upx -d file`, which overwrites in place. \
             No shell is involved.",
        ));

        if let Some(action) = self.en_cours {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spinner();
                ui.weak(match action {
                    UpxAction::List => ctx.t("upx regarde le fichier…", "upx is looking…"),
                    UpxAction::Unpack => ctx.t("upx déballe…", "upx is unpacking…"),
                });
            });
        }
    }

    /// Ce qu'UPX a répondu — y compris son refus, qui est une réponse.
    fn ce_qu_upx_a_dit(&self, ui: &mut egui::Ui, ctx: &ReContext) {
        if let Some(message) = &self.erreur {
            ui.colored_label(ui.visuals().error_fg_color, message);
            return;
        }
        let Some(action) = self.fait else {
            return;
        };

        ui.strong(match action {
            UpxAction::List => ctx.t("Ce qu'upx voit", "What upx sees"),
            UpxAction::Unpack => ctx.t("Ce qu'upx a fait", "What upx did"),
        });
        ui.add_space(2.0);
        // LE REFUS, EXPLIQUÉ AVANT D'ÊTRE MONTRÉ.
        //
        // upx refuse en jetant le nom d'une exception C++ —
        // « CantUnpackException: e_phnum must be >= 2 ». C'est exact et
        // cela n'apprend rien : on le lit comme une panne de namsdec.
        // La traduction passe donc DEVANT, et la ligne d'origine reste
        // dessous — c'est elle, la preuve, et c'est elle qu'on recopie
        // dans une recherche.
        let brut = self.sortie.join("\n");
        if let Some(explication) = refus_expliqué(&brut, ctx.lang.starts_with("fr")) {
            ui.colored_label(egui::Color32::from_rgb(0xd7, 0xba, 0x7d), explication);
            ui.add_space(4.0);
        }
        for ligne in &self.sortie {
            ui.add(
                egui::Label::new(egui::RichText::new(ligne).monospace())
                    .selectable(true)
                    .wrap_mode(egui::TextWrapMode::Extend),
            );
        }

        // LE REFUS EXPLIQUÉ. `NotPackedException` est la réponse la plus
        // fréquente sur un vrai échantillon, et lue telle quelle elle
        // fait conclure « ce n'est pas de l'UPX » — ce qu'elle ne dit
        // pas. Elle dit que les en-têtes de bloc ne sont pas là où upx
        // les attend, ce qui arrive aussi quand on les a abîmés exprès.
        if self
            .sortie
            .iter()
            .any(|ligne| ligne.contains("NotPackedException"))
        {
            ui.add_space(6.0);
            ui.weak(ctx.t(
                "« NotPackedException » ne veut pas dire « pas d'UPX » : elle veut dire \
                 que les en-têtes de bloc ne sont pas là où upx les attend. Sur un \
                 binaire ordinaire, c'est la bonne réponse. Sur un binaire à forte \
                 entropie, c'est le signe qu'on a effacé ou modifié ces en-têtes — et \
                 la suite passe alors par le vidage mémoire, après que le programme \
                 s'est déballé lui-même.",
                "“NotPackedException” does not mean “no UPX”: it means the block headers \
                 are not where upx expects them. On an ordinary binary that is the right \
                 answer. On a high-entropy binary it is the sign that those headers were \
                 erased or altered — and what follows goes through a memory dump, once \
                 the program has unpacked itself.",
            ));
        }

        // Le fichier écrit, et ce qu'il pèse par rapport à l'échantillon.
        // Le rapport est la vérification : un déballage qui rend un
        // fichier de la même taille n'a rien déballé.
        if let Some(chemin) = &self.déballé
            && action == UpxAction::Unpack
        {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.weak(ctx.t("Écrit dans", "Written to"));
                ui.add(
                    egui::Label::new(egui::RichText::new(chemin.display().to_string()).monospace())
                        .selectable(true),
                );
            });
            ui.weak(ctx.t(
                "Ouvert dans un onglet à côté : l'échantillon empaqueté reste dans le \
                 sien, et l'on compare les deux.",
                "Opened in a tab beside this one: the packed sample stays in its own, and \
                 one compares the two.",
            ));
        }
    }

    /// Ce que namsdec sait d'`upx`, posé par l'application.
    ///
    /// Public pour que l'interface puisse le dire sans passer par un
    /// `ReResult` quand elle le connaît déjà — et pour qu'un contrôle
    /// puisse peindre le panneau dans les trois états **sans lire la
    /// machine**. Un contrôle qui lit le PATH passe sur le poste de qui
    /// l'écrit et échoue sur la CI.
    pub fn set_availability(&mut self, availability: Availability) {
        self.disponible = availability;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La bannière de copyright s'en va, le tableau reste.
    ///
    /// Trois lignes sur chaque sortie d'upx, qui n'apprennent rien et
    /// repoussent sous le bord ce qu'on vient lire.
    #[test]
    fn la_banniere_sen_va_et_la_reponse_reste() {
        assert!(bannière(
            "                       Ultimate Packer for eXecutables"
        ));
        assert!(bannière(
            "                          Copyright (C) 1996 - 2026"
        ));
        assert!(bannière(
            "UPX 5.2.1       Markus Oberhumer, Laszlo Molnar & John Reiser   Aug 27th 2026"
        ));
        assert!(bannière("   "));
        // Ce qu'on vient lire, et qui doit rester.
        assert!(!bannière(
            "     40752 ->     20248   49.69%   linux/amd64   programme"
        ));
        assert!(!bannière(
            "        File size         Ratio      Format      Name"
        ));
        assert!(!bannière("Unpacked 1 file."));
        assert!(!bannière(
            "upx: /tmp/program: NotPackedException: not packed by UPX"
        ));
        // Une ligne qui commence par « UPX » sans être la bannière : le
        // nom de l'outil seul ne suffit pas à la reconnaître.
        assert!(!bannière("UPX0 est vide, comme toujours"));
    }

    /// La marque d'UPX est relevée à l'ouverture du binaire, et l'absence
    /// de marque n'est pas une marque.
    #[test]
    fn la_marque_est_relevee_a_louverture() {
        let cpu = namsdec_core::Cpu::new(namsdec_core::Memory::new(0x400000, 0x1000));
        let contexte = |octets: &[u8]| -> Vec<u8> { octets.to_vec() };

        let mut packé = vec![0u8; 256];
        packé.extend_from_slice(b"UPX!");
        for (octets, attendu) in [
            (contexte(b"un programme ordinaire"), false),
            (packé.clone(), true),
        ] {
            let mut module = UpxModule::default();
            let ctx = ReContext {
                cpu: &cpu,
                disasm: &[],
                source: "",
                binary: &octets,
                lang: "fr",
                names: None,
            };
            module.on_binary_loaded(&ctx);
            assert_eq!(module.marque.is_some(), attendu, "{} octets", octets.len());
        }
    }

    /// Un autre binaire efface la sortie d'avant.
    ///
    /// La laisser à l'écran ferait lire le rapport d'un programme en
    /// croyant lire celui d'un autre — et sur un déballage, le chemin
    /// montré serait celui du fichier précédent.
    #[test]
    fn un_autre_binaire_efface_la_sortie_davant() {
        let cpu = namsdec_core::Cpu::new(namsdec_core::Memory::new(0x400000, 0x1000));
        let mut module = UpxModule::default();
        module.deliver(&ReResult::UpxOutput {
            action: UpxAction::Unpack,
            text: Ok("Unpacked 1 file.".to_owned()),
            unpacked: Some(std::path::PathBuf::from("/tmp/déballé")),
        });
        assert!(!module.lines().is_empty());
        assert!(module.déballé.is_some());

        let octets = b"un autre programme".to_vec();
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &octets,
            lang: "fr",
            names: None,
        };
        module.on_binary_loaded(&ctx);
        assert!(module.lines().is_empty(), "{:?}", module.lines());
        assert!(module.déballé.is_none(), "et le chemin d'avant aussi");
        assert!(module.fait.is_none());
    }

    /// Un refus d'upx se montre comme un refus, non comme une sortie.
    #[test]
    fn un_refus_se_montre_comme_un_refus() {
        let mut module = UpxModule::default();
        module.deliver(&ReResult::UpxOutput {
            action: UpxAction::List,
            text: Err("upx n'est pas sur cette machine".to_owned()),
            unpacked: None,
        });
        assert!(module.erreur.is_some());
        assert!(module.lines().is_empty());
        assert!(module.en_cours.is_none(), "et le sablier s'arrête");
    }
}

#[cfg(test)]
mod tests_refus {
    use super::*;

    /// Un refus d'upx est TRADUIT, et la ligne d'origine reste.
    ///
    /// `CantUnpackException: e_phnum must be >= 2` est exact et
    /// n'apprend rien : on le lit comme une panne de namsdec. Signalé
    /// sur capture le 21 octobre 2026.
    #[test]
    fn upx_refusals_are_explained() {
        let cas = [
            (
                "upx: /tmp/x: CantUnpackException: e_phnum must be >= 2",
                "en-têtes",
            ),
            (
                "upx: /tmp/x: CantUnpackException: not packed by UPX",
                "n'est pas empaqueté",
            ),
            (
                "upx: /tmp/x: CantUnpackException: file is modified/hacked/protected",
                "PUIS MODIFIÉ",
            ),
            (
                "upx: /tmp/x: CantPackException: file not executable; try 'chmod +x'",
                "droit d'exécution",
            ),
        ];
        for (sortie, attendu) in cas {
            let dit = refus_expliqué(sortie, true)
                .unwrap_or_else(|| panic!("« {sortie} » doit être expliqué"));
            assert!(
                dit.contains(attendu),
                "« {sortie} » : attendu « {attendu} », obtenu « {dit} »"
            );
        }
    }

    /// Et ce qui n'est PAS un refus n'est pas affublé d'une
    /// explication.
    ///
    /// Un déballage réussi rend un tableau de tailles : y coller « upx
    /// a refusé » serait pire que de ne rien dire.
    #[test]
    fn a_success_is_not_dressed_up_as_a_refusal() {
        let réussi = "        File size         Ratio      Format      Name\n\
                      153400 <-     84548   55.12%   linux/amd64   sortie\n\
                      Unpacked 1 file.";
        assert_eq!(
            refus_expliqué(réussi, true),
            None,
            "un succès n'est pas un refus"
        );
    }
}
