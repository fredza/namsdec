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
            });
            self.status = Status::Note(|s| s.patch_applied);
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
                namsdec_re::ReRequest::GoTo { addr } => self.show_address(addr),
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
            }
        }
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
                // LA FENÊTRE D'ANALYSE S'EFFACE. Elle couvre le listing
                // — c'est tout l'objet d'une grande fenêtre —, et l'on
                // venait de demander à voir ce qu'elle cache. Le clic
                // marchait déjà, mais personne ne pouvait le savoir :
                // le listing défilait derrière un panneau opaque. Le
                // panneau choisi est retenu ; « Analyse du binaire… »
                // le rouvre à la même page.
                self.show_analysis = false;
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
    pub(crate) fn undo_patch(&mut self, rang: usize) {
        let Some(patch) = self.patches.get(rang).cloned() else {
            return;
        };
        let Some(state) = &self.assemble_state else {
            return;
        };
        let mut image = state.output.file.clone();
        if namsdec_re::patch::apply(&mut image, patch.offset, &patch.before).is_none() {
            return;
        }
        let nom = state
            .from_binary
            .as_ref()
            .map(|binaire| binaire.name.clone())
            .unwrap_or_default();
        if self.load_elf(&image, &nom) {
            self.patches.remove(rang);
            self.status = Status::Note(|s| s.patch_undone);
        }
    }
}
