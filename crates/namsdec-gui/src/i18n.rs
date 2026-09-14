//! Chaînes d'interface bilingues français/anglais. Un champ par libellé
//! *statique* (menus, onglets, boutons, titres de boîtes de dialogue) —
//! l'exhaustivité est vérifiée par le compilateur, ce qui évite d'oublier
//! une traduction. Le texte dynamique (avec des valeurs interpolées, ex.
//! "N octets assemblés") reste géré au point d'appel plutôt que centralisé
//! ici : le gain de mutualiser un simple `if/else` de quelques mots est
//! nul face à la mécanique qu'il faudrait pour templater des arguments
//! formatés à travers une seule struct.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr,
    En,
}

pub struct Strings {
    // Barre de menus
    pub menu_file: &'static str,
    pub menu_edit: &'static str,
    pub menu_tools: &'static str,
    pub menu_view: &'static str,
    pub menu_help: &'static str,

    pub file_preferences: &'static str,
    pub file_quit: &'static str,
    pub edit_copy_code: &'static str,
    pub edit_paste: &'static str,
    pub help_about: &'static str,

    // Barre d'outils / menu Outils (mêmes libellés, réutilisés)
    pub action_assemble: &'static str,
    pub action_step: &'static str,
    pub action_run: &'static str,
    pub action_reset: &'static str,
    pub nasm_not_found: &'static str,

    // Onglets du panneau latéral / menu Affichage
    pub tab_registers: &'static str,
    pub tab_flags: &'static str,
    pub tab_stack: &'static str,
    pub tab_memory: &'static str,
    pub tab_re: &'static str,
    pub tab_modules: &'static str,

    // Éditeur
    pub breakpoint_on_line: &'static str,
    pub add: &'static str,
    pub memory_address_label: &'static str,

    // Barre de statut (cas statiques uniquement)
    pub status_ready: &'static str,
    pub status_halted: &'static str,
    pub status_step_limit: &'static str,
    pub status_nasm_missing: &'static str,
    pub binary_too_large: &'static str,

    // Phrases "rien à afficher pour l'instant", réutilisées à plusieurs
    // endroits
    pub no_binary_assembled: &'static str,
    pub no_binary_assembled_yet: &'static str,
    pub no_program_loaded: &'static str,
    pub no_re_module_registered: &'static str,
    pub address_out_of_range: &'static str,

    // Onglet Modules
    pub module_core_detail: &'static str,

    // Boîte Préférences
    pub preferences_title: &'static str,
    pub preferences_theme: &'static str,
    pub theme_dark: &'static str,
    pub theme_light: &'static str,
    pub preferences_language: &'static str,

    // Boîte À propos
    pub about_title: &'static str,
    pub about_build_word: &'static str,
    pub about_license: &'static str,
    pub about_description: &'static str,
}

const FR: Strings = Strings {
    menu_file: "Fichier",
    menu_edit: "Édition",
    menu_tools: "Outils",
    menu_view: "Affichage",
    menu_help: "Aide",

    file_preferences: "Préférences…",
    file_quit: "Quitter",
    edit_copy_code: "Copier le code",
    edit_paste: "Coller",
    help_about: "À propos…",

    action_assemble: "Assembler",
    action_step: "Pas à pas",
    action_run: "Exécuter",
    action_reset: "Réinitialiser",
    nasm_not_found: "nasm introuvable",

    tab_registers: "Registres",
    tab_flags: "Flags",
    tab_stack: "Pile",
    tab_memory: "Mémoire",
    tab_re: "RE",
    tab_modules: "Modules",

    breakpoint_on_line: "Point d'arrêt sur la ligne :",
    add: "Ajouter",
    memory_address_label: "Adresse :",

    status_ready: "Prêt.",
    status_halted: "Arrêté (hlt).",
    status_step_limit: "Limite de pas atteinte — boucle infinie possible ?",
    status_nasm_missing: "nasm est introuvable dans le PATH (ex : `sudo dnf install nasm`).",
    binary_too_large: "le binaire assemblé dépasse la fenêtre mémoire simulée",

    no_binary_assembled: "Aucun binaire assemblé.",
    no_binary_assembled_yet: "Aucun binaire assemblé pour l'instant.",
    no_program_loaded: "Aucun programme chargé.",
    no_re_module_registered: "Aucun module RE enregistré.",
    address_out_of_range: "Adresse hors de la fenêtre mémoire simulée.",

    module_core_detail: "Registres, indicateurs (RFLAGS), mémoire simulée, état CPU.",

    preferences_title: "Préférences",
    preferences_theme: "Thème",
    theme_dark: "🌙 Sombre",
    theme_light: "☀ Clair",
    preferences_language: "Langue",

    about_title: "À propos de Namsdec",
    about_build_word: "build",
    about_license: "Licence MIT",
    about_description: "Atelier d'apprentissage NASM et de reverse engineering, modulaire.",
};

const EN: Strings = Strings {
    menu_file: "File",
    menu_edit: "Edit",
    menu_tools: "Tools",
    menu_view: "View",
    menu_help: "Help",

    file_preferences: "Preferences…",
    file_quit: "Quit",
    edit_copy_code: "Copy code",
    edit_paste: "Paste",
    help_about: "About…",

    action_assemble: "Assemble",
    action_step: "Step",
    action_run: "Run",
    action_reset: "Reset",
    nasm_not_found: "nasm not found",

    tab_registers: "Registers",
    tab_flags: "Flags",
    tab_stack: "Stack",
    tab_memory: "Memory",
    tab_re: "RE",
    tab_modules: "Modules",

    breakpoint_on_line: "Breakpoint on line:",
    add: "Add",
    memory_address_label: "Address:",

    status_ready: "Ready.",
    status_halted: "Halted (hlt).",
    status_step_limit: "Step limit reached — possible infinite loop?",
    status_nasm_missing: "nasm was not found in PATH (e.g.: `sudo dnf install nasm`).",
    binary_too_large: "the assembled binary exceeds the simulated memory window",

    no_binary_assembled: "No binary assembled.",
    no_binary_assembled_yet: "No binary assembled yet.",
    no_program_loaded: "No program loaded.",
    no_re_module_registered: "No RE module registered.",
    address_out_of_range: "Address outside the simulated memory window.",

    module_core_detail: "Registers, flags (RFLAGS), simulated memory, CPU state.",

    preferences_title: "Preferences",
    preferences_theme: "Theme",
    theme_dark: "🌙 Dark",
    theme_light: "☀ Light",
    preferences_language: "Language",

    about_title: "About Namsdec",
    about_build_word: "build",
    about_license: "MIT License",
    about_description: "A modular NASM learning and reverse-engineering workbench.",
};

pub fn strings(lang: Lang) -> &'static Strings {
    match lang {
        Lang::Fr => &FR,
        Lang::En => &EN,
    }
}
