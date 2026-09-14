//! Coloration syntaxique NASM minimale pour l'éditeur : un tokenizer
//! simple par ligne (mots/espaces/ponctuation), pas une grammaire NASM
//! complète — pas de gestion des chaînes entre guillemets par exemple,
//! hors de portée pour ce que l'éditeur a besoin de distinguer visuellement
//! (mnémoniques, nombres, étiquettes, commentaires).

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, TextStyle, Ui};

const MNEMONICS: &[&str] = &[
    "mov", "add", "sub", "cmp", "test", "and", "or", "xor", "inc", "dec", "push", "pop", "lea",
    "nop", "jmp", "je", "jne", "jl", "jle", "jg", "jge", "jb", "jbe", "ja", "jae", "jz", "jnz",
    "call", "ret", "hlt",
];

const DIRECTIVES: &[&str] = &[
    "bits", "org", "section", "global", "extern", "db", "dw", "dd", "dq", "resb", "resw", "resd",
    "resq", "equ", "times", "default", "align",
];

pub fn highlight(ui: &Ui, text: &str, wrap_width: f32) -> LayoutJob {
    let font_id = TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let base = visuals.text_color();

    // Palette inspirée de VS Code Dark+/Light+, comme le reste de l'app.
    let (mnemonic, number, label, comment) = if visuals.dark_mode {
        (
            Color32::from_rgb(0x56, 0x9c, 0xd6),
            Color32::from_rgb(0xb5, 0xce, 0xa8),
            Color32::from_rgb(0x4e, 0xc9, 0xb0),
            Color32::from_rgb(0x6a, 0x99, 0x55),
        )
    } else {
        (
            Color32::from_rgb(0x00, 0x00, 0xff),
            Color32::from_rgb(0x09, 0x86, 0x58),
            Color32::from_rgb(0x26, 0x7f, 0x99),
            Color32::from_rgb(0x00, 0x80, 0x00),
        )
    };

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            append(&mut job, "\n", &font_id, base, false);
        }
        highlight_line(&mut job, line, &font_id, base, mnemonic, number, label, comment);
    }

    job
}

#[allow(clippy::too_many_arguments)]
fn highlight_line(
    job: &mut LayoutJob,
    line: &str,
    font_id: &FontId,
    base: Color32,
    mnemonic_color: Color32,
    number_color: Color32,
    label_color: Color32,
    comment_color: Color32,
) {
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
            append(job, &chars[start..i].iter().collect::<String>(), font_id, base, false);
        } else if c.is_alphanumeric() || c == '_' || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let followed_by_colon = i < chars.len() && chars[i] == ':';
            let lower = word.to_ascii_lowercase();
            let color = if followed_by_colon {
                label_color
            } else if first_word_on_line
                && (MNEMONICS.contains(&lower.as_str()) || DIRECTIVES.contains(&lower.as_str()))
            {
                mnemonic_color
            } else if word.starts_with(|c: char| c.is_ascii_digit()) {
                number_color
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
        append(job, comment, font_id, comment_color, true);
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
