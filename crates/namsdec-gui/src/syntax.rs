//! Coloration syntaxique NASM minimale pour l'éditeur : un tokenizer
//! simple par ligne (mots/espaces/ponctuation), pas une grammaire NASM
//! complète — pas de gestion des chaînes entre guillemets par exemple,
//! hors de portée pour ce que l'éditeur a besoin de distinguer visuellement
//! (mnémoniques, nombres, étiquettes, commentaires).

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, TextStyle, Ui};
use namsdec_theme::Theme;

const MNEMONICS: &[&str] = &[
    "mov", "add", "sub", "cmp", "test", "and", "or", "xor", "inc", "dec", "push", "pop", "lea",
    "nop", "jmp", "je", "jne", "jl", "jle", "jg", "jge", "jb", "jbe", "ja", "jae", "jz", "jnz",
    "call", "ret", "hlt",
];

const DIRECTIVES: &[&str] = &[
    "bits", "org", "section", "global", "extern", "db", "dw", "dd", "dq", "resb", "resw", "resd",
    "resq", "equ", "times", "default", "align",
];

/// Les couleurs viennent du thème actif (`namsdec-theme`) et non du
/// `dark_mode` d'egui : un thème installé plus tard apporte sa propre
/// palette de code sans qu'il y ait ici deux branches à rallonger.
pub fn highlight(ui: &Ui, theme: &Theme, text: &str, wrap_width: f32) -> LayoutJob {
    let font_id = TextStyle::Monospace.resolve(ui.style());

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            append(&mut job, "\n", &font_id, theme.text, false);
        }
        highlight_line(&mut job, theme, line, &font_id);
    }

    job
}

fn highlight_line(job: &mut LayoutJob, theme: &Theme, line: &str, font_id: &FontId) {
    let base = theme.text;
    let (code, comment) = match line.find(';') {
        Some(idx) => (&line[..idx], Some(&line[idx..])),
        None => (line, None),
    };

    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    let mut first_word_on_line = true;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            let start = i;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            append(
                job,
                &chars[start..i].iter().collect::<String>(),
                font_id,
                base,
                false,
            );
        } else if c.is_alphanumeric() || c == '_' || c == '.' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let followed_by_colon = i < chars.len() && chars[i] == ':';
            let lower = word.to_ascii_lowercase();
            let color = if followed_by_colon {
                theme.label
            } else if first_word_on_line
                && (MNEMONICS.contains(&lower.as_str()) || DIRECTIVES.contains(&lower.as_str()))
            {
                theme.mnemonic
            } else if word.starts_with(|c: char| c.is_ascii_digit()) {
                theme.number
            } else {
                base
            };
            append(job, &word, font_id, color, false);
            first_word_on_line = false;
        } else {
            append(job, &chars[i].to_string(), font_id, base, false);
            i += 1;
        }
    }

    if let Some(comment) = comment {
        append(job, comment, font_id, theme.comment, true);
    }
}

fn append(job: &mut LayoutJob, text: &str, font_id: &FontId, color: Color32, italics: bool) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color,
            italics,
            ..Default::default()
        },
    );
}
