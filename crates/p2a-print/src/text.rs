//! Tekst i trykkfilen, tegnet som streker fra fontfilene (Fraunces og Nunito Sans, SIL OFL
//! 1.1). Slik ser teksten lik ut hos alle trykkerier, uten innebygde fonter.

use pdf_writer::Content;
use ttf_parser::{Face, OutlineBuilder};

static FRAUNCES: &[u8] = include_bytes!("../fonts/Fraunces-SemiBold.ttf");
static FRAUNCES_ITALIC: &[u8] = include_bytes!("../fonts/Fraunces-Italic.ttf");
static NUNITO: &[u8] = include_bytes!("../fonts/NunitoSans-Regular.ttf");

/// Skriftene som brukes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// Overskrifter og tittel.
    Display,
    /// Kursiv til små hilsener.
    Italic,
    /// Brødtekst.
    Body,
}

impl Font {
    fn face(self) -> Face<'static> {
        let data = match self {
            Font::Display => FRAUNCES,
            Font::Italic => FRAUNCES_ITALIC,
            Font::Body => NUNITO,
        };
        Face::parse(data, 0).expect("fontfilen følger med programmet")
    }
}

/// Skriver én glyf som stier i PDF-koordinater (punkter, y opp).
struct Pen<'a> {
    c: &'a mut Content,
    x: f32,
    y: f32,
    s: f32,
    last: (f32, f32),
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x * self.s, self.y + y * self.s)
    }
}

impl OutlineBuilder for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        self.c.move_to(a, b);
        self.last = (a, b);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        self.c.line_to(a, b);
        self.last = (a, b);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        // Kvadratisk til kubisk kurve.
        let (q1, q2) = self.p(x1, y1);
        let (e1, e2) = self.p(x, y);
        let (s1, s2) = self.last;
        self.c.cubic_to(
            s1 + 2.0 / 3.0 * (q1 - s1),
            s2 + 2.0 / 3.0 * (q2 - s2),
            e1 + 2.0 / 3.0 * (q1 - e1),
            e2 + 2.0 / 3.0 * (q2 - e2),
            e1,
            e2,
        );
        self.last = (e1, e2);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a1, b1) = self.p(x1, y1);
        let (a2, b2) = self.p(x2, y2);
        let (e1, e2) = self.p(x, y);
        self.c.cubic_to(a1, b1, a2, b2, e1, e2);
        self.last = (e1, e2);
    }
    fn close(&mut self) {
        self.c.close_path();
    }
}

/// Bredden av teksten i punkter.
pub fn width(font: Font, size: f32, text: &str) -> f32 {
    let face = font.face();
    let s = size / face.units_per_em() as f32;
    text.chars()
        .map(|ch| {
            face.glyph_index(ch)
                .and_then(|g| face.glyph_hor_advance(g))
                .unwrap_or(0) as f32
                * s
        })
        .sum()
}

/// Tegner én linje med grunnlinjen i (x, y), i punkter. Fargen settes av den som kaller.
pub fn draw(c: &mut Content, font: Font, size: f32, x: f32, y: f32, text: &str) {
    let face = font.face();
    let s = size / face.units_per_em() as f32;
    let mut pen_x = x;
    for ch in text.chars() {
        let Some(g) = face.glyph_index(ch) else {
            continue;
        };
        let mut pen = Pen {
            c,
            x: pen_x,
            y,
            s,
            last: (pen_x, y),
        };
        if face.outline_glyph(g, &mut pen).is_some() {
            c.fill_nonzero();
        }
        pen_x += face.glyph_hor_advance(g).unwrap_or(0) as f32 * s;
    }
}

/// Bryter teksten i linjer som er høyst `max` punkter brede. Tomme linjer (avsnitt) beholdes.
pub fn wrap(font: Font, size: f32, text: &str, max: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && width(font, size, &candidate) > max {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norske_bokstaver_finnes_og_har_bredde() {
        for f in [Font::Display, Font::Italic, Font::Body] {
            let face = f.face();
            for ch in "ÆØÅæøå«»–".chars() {
                assert!(face.glyph_index(ch).is_some(), "{ch} mangler i {f:?}");
            }
            assert!(width(f, 12.0, "Øyeblikk fra 2011") > 50.0);
        }
    }

    #[test]
    fn linjer_brytes_innenfor_bredden() {
        let text = "Vi var på hytta hele sommeren, og barna lærte å svømme.\nNytt avsnitt.";
        let lines = wrap(Font::Body, 11.0, text, 120.0);
        assert!(lines.len() >= 3);
        assert!(lines
            .iter()
            .all(|l| width(Font::Body, 11.0, l) <= 120.0 || !l.contains(' ')));
        assert_eq!(lines.last().map(String::as_str), Some("Nytt avsnitt."));
    }
}
