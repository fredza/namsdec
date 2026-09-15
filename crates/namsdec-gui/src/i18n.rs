//! Chaînes d'interface bilingues français/anglais. Un champ par libellé
//! *statique* (menus, onglets, boutons, titres de boîtes de dialogue) —
//! l'exhaustivité est vérifiée par le compilateur, ce qui évite d'oublier
//! une traduction. Le texte dynamique (avec des valeurs interpolées, ex.
//! "N octets assemblés") reste géré au point d'appel plutôt que centralisé
//! ici : le gain de mutualiser un simple `if/else` de quelques mots est
//! nul face à la mécanique qu'il faudrait pour templater des arguments
//! formatés à travers une seule struct.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    pub file_open_folder: &'static str,
    pub file_open_file: &'static str,
    // Filtres du sélecteur de fichiers natif
    pub filter_asm: &'static str,
    pub filter_binary: &'static str,
    pub filter_all: &'static str,
    pub file_save_as: &'static str,
    pub file_preferences: &'static str,
    pub file_quit: &'static str,
    pub edit_copy_code: &'static str,
    pub edit_paste: &'static str,
    pub help_about: &'static str,

    // Barre d'outils / menu Outils (mêmes libellés, réutilisés)
    pub action_assemble: &'static str,
    pub action_step_into: &'static str,
    pub action_step_over: &'static str,
    pub action_run: &'static str,
    pub action_stop: &'static str,
    pub action_reset: &'static str,
    pub action_toggle_explorer: &'static str,
    pub action_save: &'static str,
    pub nasm_not_found: &'static str,

    // Boutons communs aux boîtes de dialogue
    pub dialog_ok: &'static str,
    pub dialog_quit: &'static str,

    // Onglets du panneau latéral / menu Affichage
    pub tab_registers: &'static str,
    pub tab_flags: &'static str,
    pub tab_stack: &'static str,
    pub tab_memory: &'static str,
    pub tab_output: &'static str,
    pub tab_re: &'static str,

    // Menu Affichage — visibilité des panneaux
    pub view_toolbar: &'static str,
    pub view_sidebar: &'static str,
    pub view_statusbar: &'static str,

    // Explorateur de fichiers
    pub explorer_title: &'static str,
    pub explorer_empty: &'static str,
    pub explorer_filter_hint: &'static str,
    pub explorer_path_hint: &'static str,
    pub explorer_go_parent: &'static str,
    pub explorer_go_root: &'static str,
    pub explorer_go_home: &'static str,
    pub explorer_go_current_file: &'static str,
    pub explorer_rename_hint: &'static str,
    /// Les trois messages d'échec de l'explorateur portent un `{}`, que la
    /// crate remplace par la raison ou le nom fautif.
    pub explorer_rename_failed: &'static str,
    pub explorer_rename_exists: &'static str,
    pub explorer_path_not_a_directory: &'static str,

    // Palette de commandes
    pub palette_open: &'static str,
    pub palette_placeholder: &'static str,
    pub palette_no_match: &'static str,
    pub palette_toggle_toolbar: &'static str,
    pub palette_toggle_sidebar: &'static str,
    pub palette_toggle_statusbar: &'static str,

    // Éditeur
    pub memory_address_label: &'static str,

    // Barre de statut (cas statiques uniquement)
    pub status_ready: &'static str,
    pub status_assembled: &'static str,
    pub status_assemble_failed: &'static str,
    pub status_running: &'static str,
    pub status_halted: &'static str,
    pub status_step_limit: &'static str,
    pub status_nasm_missing: &'static str,
    pub binary_too_large: &'static str,
    pub binary_not_text: &'static str,

    // Phrases "rien à afficher pour l'instant", réutilisées à plusieurs
    // endroits
    pub no_binary_assembled: &'static str,
    pub no_binary_assembled_yet: &'static str,
    pub no_program_loaded: &'static str,
    pub no_re_module_registered: &'static str,
    pub output_empty: &'static str,
    pub console_input_hint: &'static str,
    pub console_send: &'static str,
    pub console_eof: &'static str,
    pub console_eof_hint: &'static str,
    pub console_waiting: &'static str,
    pub status_waiting_for_input: &'static str,
    pub address_out_of_range: &'static str,

    // Vue d'ensemble des modules (dans Préférences)
    pub module_core_detail: &'static str,
    pub modules_legend: &'static str,
    pub show_re_tab_checkbox: &'static str,

    // Boîte Préférences
    pub preferences_title: &'static str,
    pub preferences_tab_general: &'static str,
    pub preferences_tab_shortcuts: &'static str,
    pub preferences_tab_modules: &'static str,
    pub preferences_tab_plugins: &'static str,
    pub preferences_theme: &'static str,
    pub preferences_theme_in_plugins: &'static str,
    pub preferences_language: &'static str,

    // Onglet Plugins (point d'extension : les thèmes en sont le premier
    // type, d'autres viendront s'y installer)
    pub plugins_themes: &'static str,
    pub plugins_themes_hint: &'static str,
    pub plugins_install_soon: &'static str,
    pub plugins_active: &'static str,
    pub rebind_simple: &'static str,
    pub rebind_leader: &'static str,
    pub rebind_waiting: &'static str,

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

    file_open_folder: "Ouvrir un dossier…",
    file_open_file: "Ouvrir un fichier…",
    filter_asm: "Source assembleur",
    filter_binary: "Binaire",
    filter_all: "Tous les fichiers",
    file_save_as: "Enregistrer sous…",
    file_preferences: "Préférences…",
    file_quit: "Quitter",
    edit_copy_code: "Copier le code",
    edit_paste: "Coller",
    help_about: "À propos…",

    action_assemble: "Assembler",
    action_step_into: "Step Into",
    action_step_over: "Step Over",
    action_run: "Exécuter",
    action_stop: "Stop",
    action_reset: "Réinitialiser",
    action_toggle_explorer: "Explorateur de fichiers",
    action_save: "Enregistrer",
    nasm_not_found: "nasm introuvable",

    dialog_ok: "OK",
    dialog_quit: "Quitter",

    tab_registers: "Registres",
    tab_flags: "Flags",
    tab_stack: "Pile",
    tab_memory: "Mémoire",
    tab_output: "Console",
    tab_re: "RE",

    view_toolbar: "Barre d'outils",
    view_sidebar: "Panneau latéral",
    view_statusbar: "Barre de statut",

    explorer_title: "Explorateur",
    explorer_empty: "Aucun dossier ouvert.",
    explorer_filter_hint: "Filtrer…",
    explorer_path_hint: "Chemin du dossier (~ accepté)",
    explorer_go_parent: "Dossier parent (Retour arrière)",
    explorer_go_root: "Racine du disque",
    explorer_go_home: "Dossier personnel",
    explorer_go_current_file: "Dossier du fichier ouvert",
    explorer_rename_hint: "Nouveau nom (Entrée pour valider)",
    explorer_rename_failed: "Renommage impossible : {}",
    explorer_rename_exists: "« {} » existe déjà.",
    explorer_path_not_a_directory: "« {} » n'est pas un dossier.",

    palette_open: "Palette de commandes…",
    palette_placeholder: "Tapez une commande…",
    palette_no_match: "Aucune commande ne correspond.",
    palette_toggle_toolbar: "Afficher/masquer la barre d'outils",
    palette_toggle_sidebar: "Afficher/masquer le panneau latéral",
    palette_toggle_statusbar: "Afficher/masquer la barre de statut",

    memory_address_label: "Adresse :",

    status_ready: "Prêt.",
    status_assembled: "Assemblage réussi",
    status_assemble_failed: "Erreur d'assemblage",
    status_running: "Exécution en cours…",
    status_halted: "Arrêté (hlt).",
    status_step_limit: "Limite de pas atteinte — boucle infinie possible ?",
    status_nasm_missing: "nasm est introuvable dans le PATH (ex : `sudo dnf install nasm`).",
    binary_too_large: "le binaire assemblé dépasse la fenêtre mémoire simulée",
    binary_not_text: "Fichier binaire chargé et désassemblé (le code affiché est reconstruit).",

    no_binary_assembled: "Aucun binaire assemblé.",
    no_binary_assembled_yet: "Aucun binaire assemblé pour l'instant.",
    no_program_loaded: "Aucun programme chargé.",
    no_re_module_registered: "Aucun module RE enregistré.",
    output_empty: "Le programme n'a rien écrit.",
    console_input_hint: "Entrée du programme…",
    console_send: "Envoyer",
    console_eof: "Fin d'entrée",
    console_eof_hint: "Signale qu'il n'y a plus rien à lire (comme Ctrl+D). Les `read` suivants rendront 0.",
    console_waiting: "Le programme attend une entrée.",
    status_waiting_for_input: "En attente d'une entrée…",
    address_out_of_range: "Adresse hors de la fenêtre mémoire simulée.",

    module_core_detail: "Registres, indicateurs (RFLAGS), mémoire simulée, état CPU.",
    modules_legend: "Vert = sain/actif · Rouge = problème · Bleu = actif, des données à afficher · Gris = inactif pour l'instant.",
    show_re_tab_checkbox: "Afficher l'onglet RE dans la barre latérale",

    preferences_title: "Préférences",
    preferences_tab_general: "Général",
    preferences_tab_shortcuts: "Raccourcis",
    preferences_tab_modules: "Modules",
    preferences_tab_plugins: "Plugins",
    preferences_theme: "Thème",
    preferences_theme_in_plugins: "Le thème se choisit dans l'onglet Plugins.",
    preferences_language: "Langue",

    plugins_themes: "Thèmes",
    plugins_themes_hint: "Cliquez sur un thème pour l'appliquer aussitôt.",
    plugins_install_soon: "D'autres thèmes pourront être installés gratuitement ici dans une prochaine version.",
    plugins_active: "actif",
    rebind_simple: "Touche simple…",
    rebind_leader: "Séquence…",
    rebind_waiting: "En attente d'une touche…",

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

    file_open_folder: "Open Folder…",
    file_open_file: "Open File…",
    filter_asm: "Assembly source",
    filter_binary: "Binary",
    filter_all: "All files",
    file_save_as: "Save As…",
    file_preferences: "Preferences…",
    file_quit: "Quit",
    edit_copy_code: "Copy code",
    edit_paste: "Paste",
    help_about: "About…",

    action_assemble: "Assemble",
    action_step_into: "Step Into",
    action_step_over: "Step Over",
    action_run: "Run",
    action_stop: "Stop",
    action_reset: "Reset",
    action_toggle_explorer: "File Explorer",
    action_save: "Save",
    nasm_not_found: "nasm not found",

    dialog_ok: "OK",
    dialog_quit: "Quit",

    tab_registers: "Registers",
    tab_flags: "Flags",
    tab_stack: "Stack",
    tab_memory: "Memory",
    tab_output: "Console",
    tab_re: "RE",

    view_toolbar: "Toolbar",
    view_sidebar: "Side Panel",
    view_statusbar: "Status Bar",

    explorer_title: "Explorer",
    explorer_empty: "No folder open.",
    explorer_filter_hint: "Filter…",
    explorer_path_hint: "Folder path (~ accepted)",
    explorer_go_parent: "Parent folder (Backspace)",
    explorer_go_root: "Filesystem root",
    explorer_go_home: "Home folder",
    explorer_go_current_file: "Folder of the open file",
    explorer_rename_hint: "New name (Enter to confirm)",
    explorer_rename_failed: "Rename failed: {}",
    explorer_rename_exists: "“{}” already exists.",
    explorer_path_not_a_directory: "“{}” is not a folder.",

    palette_open: "Command Palette…",
    palette_placeholder: "Type a command…",
    palette_no_match: "No matching command.",
    palette_toggle_toolbar: "Toggle Toolbar",
    palette_toggle_sidebar: "Toggle Side Panel",
    palette_toggle_statusbar: "Toggle Status Bar",

    memory_address_label: "Address:",

    status_ready: "Ready.",
    status_assembled: "Assembled",
    status_assemble_failed: "Assembly error",
    status_running: "Running…",
    status_halted: "Halted (hlt).",
    status_step_limit: "Step limit reached — possible infinite loop?",
    status_nasm_missing: "nasm was not found in PATH (e.g.: `sudo dnf install nasm`).",
    binary_too_large: "the assembled binary exceeds the simulated memory window",
    binary_not_text: "Binary file loaded and disassembled (the code shown is reconstructed).",

    no_binary_assembled: "No binary assembled.",
    no_binary_assembled_yet: "No binary assembled yet.",
    no_program_loaded: "No program loaded.",
    no_re_module_registered: "No RE module registered.",
    output_empty: "The program has not written anything.",
    console_input_hint: "Program input…",
    console_send: "Send",
    console_eof: "End input",
    console_eof_hint: "Signals there is nothing left to read (like Ctrl+D). Further reads return 0.",
    console_waiting: "The program is waiting for input.",
    status_waiting_for_input: "Waiting for input…",
    address_out_of_range: "Address outside the simulated memory window.",

    module_core_detail: "Registers, flags (RFLAGS), simulated memory, CPU state.",
    modules_legend: "Green = healthy/active · Red = problem · Blue = active, has data to show · Gray = inactive for now.",
    show_re_tab_checkbox: "Show the RE tab in the sidebar",

    preferences_title: "Preferences",
    preferences_tab_general: "General",
    preferences_tab_shortcuts: "Shortcuts",
    preferences_tab_modules: "Modules",
    preferences_tab_plugins: "Plugins",
    preferences_theme: "Theme",
    preferences_theme_in_plugins: "Pick a theme in the Plugins tab.",
    preferences_language: "Language",

    plugins_themes: "Themes",
    plugins_themes_hint: "Click a theme to apply it right away.",
    plugins_install_soon: "More themes will be installable here for free in a future release.",
    plugins_active: "active",
    rebind_simple: "Single key…",
    rebind_leader: "Sequence…",
    rebind_waiting: "Waiting for a key…",

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
