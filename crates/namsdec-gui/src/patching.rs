//! Modifier les octets du binaire ouvert : prévoir un correctif, l'écrire
//! (octets ou instruction assemblée), et le défaire.
//!
//! Des méthodes de `NamsdecApp` regroupées hors de `main.rs` : elles
//! forment un sujet à part — le seul endroit où l'application change ce
//! qu'elle a chargé, toujours de façon réversible.

use super::*;

impl NamsdecApp {
    /// Ce que le patch de la ligne du curseur ferait, s'il peut faire
    /// quelque chose.
    ///
    /// Rend l'adresse, les octets d'origine et ceux qu'on écrirait.
    /// Séparé de l'action pour que le bouton puisse dire *avant* de
    /// cliquer ce qu'il s'apprête à faire : on ne change pas les octets
    /// de quelqu'un à l'aveugle.
    pub(crate) fn patch_preview(&self, geste: PatchGeste) -> Option<(u64, Vec<u8>, Vec<u8>)> {
        let state = self.assemble_state.as_ref()?;
        // Un binaire ouvert, non un programme qu'on vient d'assembler :
        // celui-ci se change en changeant son source, ce qui est plus
        // simple et plus sûr.
        state.from_binary.as_ref()?;
        let ligne = self.cursor_pos.0 as u32;
        // Par la carte ligne → adresse, et non en parcourant la carte
        // inverse : l'aperçu est demandé trois fois par image, fenêtre
        // des correctifs ouverte, et un listing d'un million de lignes
        // porte un million d'entrées. Les deux cartes existent
        // justement pour n'avoir jamais à parcourir l'une des deux.
        let adresse = *state.output.line_to_addr.get(&ligne)?;
        // Les instructions sont triées par adresse : dichotomie.
        let rang = state
            .disasm
            .binary_search_by_key(&adresse, |instruction| instruction.address)
            .ok()?;
        let instruction = &state.disasm[rang];
        let nouveaux = match geste {
            PatchGeste::Inverser => namsdec_re::patch::inverted_jump(&instruction.bytes)?,
            PatchGeste::Neutraliser => namsdec_re::patch::nops(instruction.bytes.len()),
            PatchGeste::Forcer => namsdec_re::patch::forced_jump(&instruction.bytes)?,
        };
        if nouveaux == instruction.bytes {
            return None;
        }
        Some((adresse, instruction.bytes.clone(), nouveaux))
    }

    /// Écrit des octets à une position du fichier, puis recharge.
    ///
    /// Le fond commun des deux gestes et de l'écriture depuis le vidage :
    /// tous finissent ici. `adresse` ne sert qu'à le dire à l'écran —
    /// zéro quand on ne l'a pas, comme depuis le vidage, où l'on désigne
    /// une position et non une instruction.
    pub(crate) fn write_patch(&mut self, offset: usize, après: Vec<u8>, adresse: u64) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            return;
        };
        let mut image = state.output.file.clone();
        let Some(avant) = namsdec_re::patch::apply(&mut image, offset, &après) else {
            self.status = Status::Error(s.patch_no_offset.to_owned());
            return;
        };
        if avant == après {
            // Rien à faire, et surtout rien à enregistrer : un patch qui
            // ne change rien encombrerait la liste de ce qu'on peut
            // défaire.
            return;
        }
        let nom = state
            .from_binary
            .as_ref()
            .map(|binaire| binaire.name.clone())
            .unwrap_or_default();
        if self.load_elf(&image, &nom) {
            self.patches.push(namsdec_re::patch::Patch {
                offset,
                before: avant,
                after: après,
                addr: adresse,
                // Un correctif posé seul ne dépend de rien d'autre.
                group: None,
            });
            self.status = Status::Note(|s| s.patch_applied);
        } else {
            // LE REFUS SE DIT. Il ne se disait pas, et c'est la
            // réponse au grief « dans l'éditeur binaire on ne sait rien
            // modifier » du 21 octobre 2026.
            //
            // Écrire sur l'en-tête — l'octet zéro porte le `7f` du
            // nombre magique — rend le fichier illisible, `load_elf`
            // refuse de le relire, et l'on sortait d'ici SANS RIEN
            // CHANGER ET SANS RIEN DIRE. L'octet retrouvait sa valeur
            // d'avant à l'image suivante : on tapait, et il ne se
            // passait rien.
            //
            // Le refus est juste — namsdec ne peut pas désassembler ce
            // qu'il ne sait plus lire —, et il manquait seulement de le
            // dire. Reproduit par `the_binary_editor_refuses_loudly`.
            self.status = Status::Error(s.patch_breaks_file.to_owned());
        }
    }

    /// Pose un détour : ses deux changements, sur la même image.
    ///
    /// Jamais deux appels à [`Self::write_patch`] à la file. Entre les
    /// deux, le fichier porterait un `jmp` vers un corps pas encore
    /// écrit — c'est-à-dire vers du bourrage —, et comme chaque écriture
    /// recharge l'ELF, cet état intermédiaire serait réellement celui
    /// que namsdec montre et que l'émulateur exécuterait. On applique
    /// les deux, puis on recharge une fois.
    pub(crate) fn write_detour(&mut self, detour: &namsdec_re::patch::Detour) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            return;
        };
        let mut image = state.output.file.clone();

        // Le corps d'abord, le saut ensuite. L'ordre ne change rien ici
        // — les deux sont appliqués avant tout rechargement —, mais il
        // dit l'intention : la destination existe avant qu'on y envoie
        // quiconque.
        let Some(avant_corps) =
            namsdec_re::patch::apply(&mut image, detour.corps.offset, &detour.corps.after)
        else {
            self.status = Status::Error(s.patch_no_offset.to_owned());
            return;
        };
        let Some(avant_aller) =
            namsdec_re::patch::apply(&mut image, detour.aller.offset, &detour.aller.after)
        else {
            self.status = Status::Error(s.patch_no_offset.to_owned());
            return;
        };

        let nom = state
            .from_binary
            .as_ref()
            .map(|binaire| binaire.name.clone())
            .unwrap_or_default();
        if self.load_elf(&image, &nom) {
            // Les octets d'avant relevés à l'application, et non ceux du
            // plan : le plan a été calculé sur l'image d'alors, et un
            // correctif posé entre-temps l'aurait démenti. C'est ce
            // qu'on a vraiment recouvert qu'il faut garder pour défaire.
            let mut corps = detour.corps.clone();
            corps.before = avant_corps;
            let mut aller = detour.aller.clone();
            aller.before = avant_aller;
            self.patches.push(corps);
            self.patches.push(aller);
            self.status = Status::Note(|s| s.patch_applied);
        } else {
            // Même refus, même silence, même correction : un détour qui
            // rendrait le fichier illisible ne se pose pas, et le dit.
            self.status = Status::Error(s.patch_breaks_file.to_owned());
        }
    }

    /// Relève ce que les modules d'analyse demandent, et l'exécute.
    ///
    /// Un module ne connaît ni le disque ni l'historique des
    /// changements : il décrit ce qu'il voudrait, et c'est ici qu'on
    /// décide. Relevé **après** que la fenêtre s'est dessinée, parce que
    /// pendant, `self` est déjà emprunté.
    pub(crate) fn apply_re_requests(&mut self) {
        let mut demandes = Vec::new();
        for module in self.re_registry.iter_mut() {
            if let Some(demande) = module.take_request() {
                demandes.push(demande);
            }
        }
        for demande in demandes {
            match demande {
                namsdec_re::ReRequest::PatchBytes { offset, bytes } => {
                    self.write_patch(offset, bytes, 0);
                }
                // Un détour : les deux changements ensemble, sous un même
                // groupe. Voir `write_detour` pour la raison.
                namsdec_re::ReRequest::PatchDetour { detour } => {
                    self.write_detour(&detour);
                }
                namsdec_re::ReRequest::GoTo { addr, target } => match target {
                    namsdec_re::GoTarget::Listing => self.show_address(addr),
                    namsdec_re::GoTarget::Bytes => self.show_bytes_at(addr),
                },
                // Renommer : on écrit dans les « notes » du binaire — la
                // même table que le listing et le déroulement —, et on la
                // garde sur le disque. Un nom donné dans l'arbre d'appels
                // se lit donc partout, et survit à la fermeture. Un nom
                // vide efface : `set_name` s'en charge.
                namsdec_re::ReRequest::Rename { addr, name } => {
                    self.notes.set_name(addr, &name);
                    self.save_notes();
                    // Et le listing porte le nom lui aussi : c'est là
                    // qu'on lit le programme, et un nom qui ne s'y voit
                    // pas n'a servi qu'aux panneaux.
                    self.refresh_listing_names();
                }
                // Décompiler : on lance le décompilateur actif. Sa sortie,
                // si c'est du JSON DZF, remontera au panneau par
                // `ingest_dzf_json` quand le fil aura fini.
                namsdec_re::ReRequest::Decompile => {
                    self.dzf_wanted = true;
                    self.start_decompile();
                }
                // Un outil des autres, demandé par sa rubrique. C'est
                // ici qu'on sort du bac à sable, jamais dans le module.
                namsdec_re::ReRequest::RunTool { tool } => self.start_tool_run(&tool),
                // UPX. Interroger ne demande rien de plus ; déballer
                // demande D'ABORD où écrire — un déballage qui finit
                // sans destination n'aurait que deux issues, écrire à
                // côté de l'échantillon sans l'avoir demandé, ou jeter
                // le résultat.
                namsdec_re::ReRequest::Upx { action } => match action {
                    namsdec_re::UpxAction::List => self.start_upx_list(),
                    namsdec_re::UpxAction::Unpack => self.ask_where_to_unpack(),
                },
            }
        }
    }

    /// Montre les OCTETS de cette adresse, dans l'éditeur binaire.
    ///
    /// L'autre façon de regarder le même endroit. Le listing dit ce que
    /// la machine y fera ; les octets disent ce qui y est écrit — et
    /// c'est la bonne vue pour une chaîne, une table, un bourrage, tout
    /// ce qu'un désassembleur lit comme des instructions parce qu'il ne
    /// sait rien faire d'autre.
    ///
    /// La conversion est faite ICI et non dans le panneau : lire les
    /// en-têtes du fichier chargé est l'affaire de l'application, et un
    /// module ne connaît que les octets qu'on lui donne.
    pub(crate) fn show_bytes_at(&mut self, addr: u64) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            return;
        };
        let Some(offset) = namsdec_re::imports::file_offset(&state.output.file, addr) else {
            // Une adresse qu'aucune section ni aucun segment ne porte :
            // elle n'est nulle part dans le fichier. Le dire, plutôt
            // que d'ouvrir l'éditeur sur une position inventée.
            self.status = Status::Error(s.analysis_addr_absent.to_owned());
            return;
        };
        // L'ANALYSE RESTE SOUS LES YEUX : la destination est dedans.
        // C'est l'inverse exact du saut au listing, qui l'efface parce
        // que ce qu'on va voir est derrière elle. Dans la rangée, elle
        // redevient l'onglet actif ; dans sa fenêtre, elle y est déjà.
        self.montrer_vue(VueCentrale::Analysis);
        self.analysis_module = "hex_editor".to_owned();
        self.re_registry
            .deliver(&namsdec_re::ReResult::ShowBytesAt {
                offset: offset as usize,
                addr: Some(addr),
            });
        self.status = Status::WentTo(addr);
    }

    /// Montre cette adresse dans le listing.
    ///
    /// Un panneau d'analyse désigne des endroits ; la seule chose qu'on
    /// veuille en faire est d'aller voir. Jusqu'ici il fallait relever
    /// l'adresse à la main et la retaper dans « aller à ».
    ///
    /// L'adresse exacte quand le listing la porte ; sinon **la ligne
    /// juste avant**. C'est le cas d'une adresse qui tombe au milieu
    /// d'une instruction — ce que fait justement un désalignement
    /// volontaire — ou d'une zone de bourrage que le listing ne montre
    /// pas ligne à ligne. Mieux vaut poser l'œil deux lignes trop haut
    /// que ne rien faire : un clic qui ne fait rien passe pour un
    /// défaut, et l'on cesse d'essayer.
    pub(crate) fn show_address(&mut self, addr: u64) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            return;
        };
        let ligne = state
            .addr_to_line
            .get(&addr)
            .copied()
            // La plus proche en dessous : `BTreeMap` les tient triées,
            // et c'est exactement la question qu'on lui pose.
            .or_else(|| {
                state
                    .addr_to_line
                    .range(..=addr)
                    .next_back()
                    .map(|(_, ligne)| *ligne)
            });
        match ligne {
            Some(ligne) => {
                self.goto_line = Some(ligne);
                // On ne suit plus RIP : sans cela, le pas suivant
                // ramènerait le listing là où l'exécution en est, et le
                // clic paraîtrait n'avoir rien fait.
                self.follow_rip = false;
                // L'ANALYSE S'EFFACE. Elle couvre le listing — c'est
                // tout l'objet d'une grande vue —, et l'on venait de
                // demander à voir ce qu'elle cache. Le clic marchait
                // déjà, mais personne ne pouvait le savoir : le listing
                // défilait derrière un panneau opaque. Détachée, sa
                // fenêtre se referme ; dans la rangée, c'est la source
                // qui revient — le listing est dans l'éditeur. La
                // rubrique choisie est retenue, et l'onglet « Analyse »
                // la rouvre à la même page.
                self.masquer_vue(VueCentrale::Analysis);
                self.status = Status::WentTo(addr);
            }
            // Une adresse qui n'est dans aucune partie désassemblée —
            // une section de données, une adresse hors du fichier. Le
            // dire, plutôt que de ne rien faire.
            None => self.status = Status::Error(s.analysis_addr_absent.to_owned()),
        }
    }

    /// Écrit le patch dans l'image, puis recharge le binaire.
    ///
    /// Recharger plutôt que rapiécer l'état en place : les octets
    /// changés donnent un autre désassemblage, d'autres fonctions,
    /// d'autres références croisées. Reprendre le fichier depuis le
    /// début est ce qui garantit que tout ce qu'on lit ensuite parle du
    /// binaire qu'on vient d'écrire — et l'exécution repart de zéro,
    /// ce qui est de toute façon ce qu'on veut après avoir changé le
    /// programme.
    pub(crate) fn apply_patch(&mut self, geste: PatchGeste) {
        let s = strings(self.lang);
        let Some((adresse, _, après)) = self.patch_preview(geste) else {
            self.status = Status::Error(s.patch_impossible.to_owned());
            return;
        };
        let Some(state) = &self.assemble_state else {
            return;
        };
        // L'adresse virtuelle de l'instruction, ramenée à sa position
        // dans le fichier : c'est là qu'on écrit.
        let Some(offset) = namsdec_re::imports::file_offset(&state.output.file, adresse) else {
            self.status = Status::Error(s.patch_no_offset.to_owned());
            return;
        };
        self.write_patch(offset as usize, après, adresse);
    }

    /// Récrit les octets d'une instruction, depuis l'onglet Octets.
    ///
    /// Le même chemin que les trois gestes du patcheur — `write_patch`,
    /// donc la fenêtre Correctifs les liste et sait les défaire. Écrire
    /// à la main depuis l'onglet n'est pas un autre mécanisme, c'est la
    /// même chose dite autrement : on voit les octets là où on les lit.
    ///
    /// LA LONGUEUR NE CHANGE PAS, et c'est refusé plutôt que rattrapé.
    /// Récrire une instruction plus courte ou plus longue déplacerait
    /// tout ce qui suit dans le fichier, donc toutes les adresses, donc
    /// tous les sauts. Neutraliser par des `nop` est le geste qui répond
    /// à ce besoin, et le patcheur le propose déjà.
    pub(crate) fn write_bytes_at(&mut self, adresse: u64, saisie: &str) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            return;
        };
        let Some(instruction) = state
            .disasm
            .iter()
            .find(|instruction| instruction.address == adresse)
        else {
            return;
        };
        let attendu = instruction.bytes.len();
        let octets_d_origine = instruction.bytes.clone();
        // DEUX façons d'écrire la même chose, et un seul champ.
        //
        // Des octets — `48 31 c0` — ou l'instruction elle-même —
        // `xor rax, rax`. Un second champ, ou une bascule, ajouterait un
        // geste et un mot à retenir pour une distinction que la saisie
        // porte déjà.
        //
        // L'hexadécimal PASSE EN PREMIER, et la règle est dite dans
        // l'infobulle : ce qui peut se lire en octets se lit en octets.
        // Sans cet ordre, `fadd` — quatre lettres qui sont aussi quatre
        // chiffres hexadécimaux — changerait de sens selon l'humeur du
        // programme, ce qui est pire que les deux réponses possibles.
        let nouveaux = match lire_octets(saisie) {
            Some(octets) => octets,
            None => match namsdec_asm::assemble_one(saisie, adresse) {
                Ok(octets) => octets,
                Err(erreur) => {
                    // Le message de nasm, non le nôtre : c'est le même
                    // assembleur que pour le reste du programme, et sa
                    // façon de dire « je ne comprends pas » est déjà
                    // celle qu'on apprend dans l'éditeur.
                    self.status = Status::Error(match &erreur {
                        namsdec_asm::AsmError::Failed { diagnostics } => diagnostics
                            .first()
                            .map(|d| d.message.clone())
                            .unwrap_or_else(|| s.bytes_edit_bad.to_owned()),
                        _ => s.bytes_edit_bad.to_owned(),
                    });
                    return;
                }
            },
        };
        if nouveaux.len() != attendu {
            // La longueur ne change pas, et c'est refusé plutôt que
            // rattrapé : décaler ce qui suit déplacerait toutes les
            // adresses du programme, et chaque saut deviendrait faux.
            // Le message dit les deux longueurs — sans cela, on récrit
            // trois fois sans comprendre ce qui cloche.
            self.status = Status::Error(format!(
                "{} ({} {} {})",
                s.bytes_edit_length,
                nouveaux.len(),
                s.bytes_edit_instead_of,
                attendu
            ));
            return;
        }
        if nouveaux == octets_d_origine {
            return;
        }
        let Some(offset) = namsdec_re::imports::file_offset(&state.output.file, adresse) else {
            self.status = Status::Error(s.patch_no_offset.to_owned());
            return;
        };
        self.write_patch(offset as usize, nouveaux, adresse);
        self.status = Status::Note(|s| s.bytes_changed);
    }

    /// Défait un patch : on remet les octets d'avant, et l'on recharge.
    ///
    /// Quand le correctif appartient à un groupe, c'est tout le groupe
    /// qui se défait. Un détour en fait deux, et défaire le corps seul
    /// laisserait le `jmp` sauter dans du bourrage : le programme
    /// partirait dans des zéros. On ne propose donc pas la demi-mesure.
    pub(crate) fn undo_patch(&mut self, rang: usize) {
        let Some(patch) = self.patches.get(rang).cloned() else {
            return;
        };
        let Some(state) = &self.assemble_state else {
            return;
        };
        let mut image = state.output.file.clone();

        // Le groupe à défaire, du dernier posé au premier : remettre les
        // octets dans l'ordre inverse de la pose est la seule façon
        // sûre quand deux correctifs se chevauchent.
        let rangs: Vec<usize> = match patch.group {
            Some(groupe) => self
                .patches
                .iter()
                .enumerate()
                .filter(|(_, p)| p.group == Some(groupe))
                .map(|(r, _)| r)
                .rev()
                .collect(),
            None => vec![rang],
        };
        for r in &rangs {
            let Some(p) = self.patches.get(*r) else {
                return;
            };
            if namsdec_re::patch::apply(&mut image, p.offset, &p.before).is_none() {
                return;
            }
        }
        let nom = state
            .from_binary
            .as_ref()
            .map(|binaire| binaire.name.clone())
            .unwrap_or_default();
        if self.load_elf(&image, &nom) {
            // Déjà du plus grand rang au plus petit : retirer dans cet
            // ordre ne décale pas ceux qui restent à retirer.
            for r in &rangs {
                self.patches.remove(*r);
            }
            self.status = Status::Note(|s| s.patch_undone);
        }
    }
}
