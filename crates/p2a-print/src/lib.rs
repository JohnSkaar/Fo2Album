//! Trykkfilen: albumet som PDF, laget lokalt på maskinen (M7, første del).
//!
//! Sidene: forside, en tekstside om året, historiene side for side slik utkastet viser dem,
//! en tom side ved behov (sidetallet må være partall), og bakside. Hver side er 21 × 28 cm
//! med 3 mm utfallende kant (`TrimBox` og `BleedBox` er satt). Bildene leses fra
//! originalfilene, skaleres til 300 ppi for feltet de står i og legges inn som JPEG.
//! Tekst tegnes som streker fra fontene, så ingen fonter må bygges inn.
//!
//! Ingenting sendes noe sted: filen skrives der brukeren velger. Bestilling (å sende den
//! ferdige filen til trykkeriet) kommer senere, og er det eneste som noen gang forlater
//! maskinen (CLAUDE.md, prinsipp 1).
//!
//! Kommer senere: omslag som eget oppslag med rygg (trykkeriets mål), CMYK/ICC etter
//! trykkeriets krav, og oppskarping.

pub mod album;
pub mod layout;
pub mod text;

use std::path::PathBuf;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use p2a_core::layout::PageKind;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};

use layout::{Frame, BLEED, MARGIN, TRIM_H, TRIM_W};
use text::Font;

/// Oppløsningen bildene skaleres til (punkter per tomme på trykket papir).
pub const PRINT_PPI: f32 = 300.0;
/// Under dette blir bildet merket som for lite for størrelsen det har i albumet.
pub const LOW_PPI: f32 = 150.0;
const MM_TO_PT: f32 = 72.0 / 25.4;

/// En originalfil. Bredde og høyde er etter rotering (som bildet skal vises).
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoSource {
    pub path: PathBuf,
    pub format: Option<String>,
    pub orientation: Option<u16>,
    pub width: u32,
    pub height: u32,
}

impl PhotoSource {
    fn aspect(&self) -> f32 {
        if self.width == 0 || self.height == 0 {
            4.0 / 3.0
        } else {
            self.width as f32 / self.height as f32
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumPage {
    pub kind: PageKind,
    pub photos: Vec<PhotoSource>,
}

/// Tekstene brukeren har skrevet (eller appens forslag).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AlbumText {
    /// Forsiden, f.eks. «Øyeblikk fra 2011».
    pub title: String,
    /// Under tittelen, f.eks. navnene i familien.
    pub subtitle: String,
    /// Første side inne i albumet: tekst om året. Tom gir bare overskriften.
    pub intro: String,
    /// Baksiden.
    pub back: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Album {
    pub year: i32,
    pub text: AlbumText,
    pub cover: Option<PhotoSource>,
    pub back: Option<PhotoSource>,
    pub pages: Vec<AlbumPage>,
}

/// Hva som ble laget, og hva brukeren bør se på før bestilling.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// Sider i filen, med forside og bakside.
    pub pages: usize,
    pub photos: usize,
    /// Bilder med for lav oppløsning for størrelsen i albumet (under 150 ppi).
    pub low_resolution: Vec<PathBuf>,
    /// Bilder som ikke kunne leses (flyttet, slettet, bare i skyen). Vises som grå felt.
    pub missing: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum PrintError {
    #[error("albumet har ingen sider")]
    Empty,
}

/// Farger fra design/tokens.css (sRGB).
mod color {
    pub const PAPER_TINT: [f32; 3] = [
        0xF2 as f32 / 255.0,
        0xEB as f32 / 255.0,
        0xE1 as f32 / 255.0,
    ]; // sand-100
    pub const TEXT: [f32; 3] = [
        0x24 as f32 / 255.0,
        0x1E as f32 / 255.0,
        0x18 as f32 / 255.0,
    ]; // sand-900
    pub const TEXT_2: [f32; 3] = [
        0x5C as f32 / 255.0,
        0x50 as f32 / 255.0,
        0x43 as f32 / 255.0,
    ]; // sand-700
    pub const TEXT_3: [f32; 3] = [
        0x7A as f32 / 255.0,
        0x6C as f32 / 255.0,
        0x5B as f32 / 255.0,
    ]; // sand-600
    pub const BRAND: [f32; 3] = [
        0xB8 as f32 / 255.0,
        0x53 as f32 / 255.0,
        0x2F as f32 / 255.0,
    ]; // terracotta-600
    pub const MISSING: [f32; 3] = [
        0xE4 as f32 / 255.0,
        0xD9 as f32 / 255.0,
        0xCA as f32 / 255.0,
    ]; // sand-200
}

/// Fra millimeter på den ferdige siden (øverst til venstre) til PDF-punkter (nederst til venstre
/// av siden med utfallende kant).
fn pt_x(x: f32) -> f32 {
    (x + BLEED) * MM_TO_PT
}
fn pt_y(y: f32) -> f32 {
    (TRIM_H + BLEED - y) * MM_TO_PT
}

/// Et bilde plassert i et felt.
struct Placed {
    photo: PhotoSource,
    frame: Frame,
}

impl Placed {
    /// Størrelsen bildet tegnes i (mm): dekker feltet når det fylles, ellers lik feltet.
    fn drawn(&self) -> (f32, f32) {
        let a = self.photo.aspect();
        let f = &self.frame;
        if (f.w / f.h > a) == f.fill {
            (f.w, f.w / a)
        } else {
            (f.h * a, f.h)
        }
    }
}

enum Block {
    Fill([f32; 3]),
    Photo(usize),
    Text {
        font: Font,
        size: f32,
        color: [f32; 3],
        x: f32,
        y: f32,
        text: String,
    },
}

struct PageSpec {
    blocks: Vec<Block>,
}

/// Et ferdig kodet bilde, klart til å legges inn.
struct Encoded {
    jpeg: Vec<u8>,
    width: u32,
    height: u32,
}

fn encode(p: &Placed) -> Option<(Encoded, f32)> {
    let (dw, dh) = p.drawn();
    let long_mm = dw.max(dh);
    let need = (long_mm / 25.4 * PRINT_PPI).ceil() as u32;
    let d = p2a_ingest::decode::decode_file_at(
        &p.photo.path,
        p.photo.format.as_deref(),
        p.photo.orientation,
        need,
    )
    .ok()?;
    let mut img = d.image.to_rgb8();
    let long_px = img.width().max(img.height());
    if long_px > need {
        let s = need as f32 / long_px as f32;
        let (w, h) = (
            ((img.width() as f32 * s).round() as u32).max(1),
            ((img.height() as f32 * s).round() as u32).max(1),
        );
        img = image::imageops::resize(&img, w, h, FilterType::Lanczos3);
    }
    let ppi = img.width().max(img.height()) as f32 / (long_mm / 25.4);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 90)
        .encode_image(&img)
        .ok()?;
    Some((
        Encoded {
            jpeg,
            width: img.width(),
            height: img.height(),
        },
        ppi,
    ))
}

fn text_block(font: Font, size: f32, color: [f32; 3], x: f32, y: f32, t: &str) -> Block {
    Block::Text {
        font,
        size,
        color,
        x,
        y,
        text: t.to_string(),
    }
}

/// Midtstilt linje: x er midten (mm).
fn centered(font: Font, size: f32, color: [f32; 3], cx: f32, y: f32, t: &str) -> Block {
    let w = text::width(font, size, t) / MM_TO_PT;
    text_block(font, size, color, cx - w / 2.0, y, t)
}

/// Lager trykkfilen. `progress(ferdig, totalt)` kalles mens bildene behandles.
pub fn render(
    album: &Album,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<(Vec<u8>, Report), PrintError> {
    if album.pages.is_empty() {
        return Err(PrintError::Empty);
    }
    let mut placed: Vec<Placed> = Vec::new();
    let mut pages: Vec<PageSpec> = Vec::new();
    let pt = MM_TO_PT;
    let text_w = TRIM_W - 2.0 * (MARGIN + 10.0);

    // 1. Forsiden: bildet øverst ut i kantene, tittel og undertittel under.
    let mut cover = vec![Block::Fill(color::PAPER_TINT)];
    let photo_h = TRIM_H * 0.70;
    if let Some(c) = &album.cover {
        placed.push(Placed {
            photo: c.clone(),
            frame: Frame {
                x: -BLEED,
                y: -BLEED,
                w: TRIM_W + 2.0 * BLEED,
                h: photo_h + BLEED,
                fill: true,
            },
        });
        cover.push(Block::Photo(placed.len() - 1));
    }
    let mut y = photo_h + 22.0;
    for line in text::wrap(Font::Display, 34.0, &album.text.title, text_w * pt) {
        cover.push(text_block(
            Font::Display,
            34.0,
            color::TEXT,
            MARGIN + 6.0,
            y,
            &line,
        ));
        y += 13.5;
    }
    if !album.text.subtitle.is_empty() {
        cover.push(text_block(
            Font::Body,
            13.0,
            color::TEXT_2,
            MARGIN + 6.0,
            y - 2.0,
            &album.text.subtitle,
        ));
    }
    pages.push(PageSpec { blocks: cover });

    // 2. Første side inne: året og teksten om året.
    let mut intro = vec![text_block(
        Font::Display,
        40.0,
        color::BRAND,
        MARGIN + 10.0,
        70.0,
        &album.year.to_string(),
    )];
    let mut y = 90.0;
    for line in text::wrap(Font::Body, 11.5, &album.text.intro, text_w * pt) {
        if !line.is_empty() {
            intro.push(text_block(
                Font::Body,
                11.5,
                color::TEXT_2,
                MARGIN + 10.0,
                y,
                &line,
            ));
        }
        y += 6.2;
    }
    pages.push(PageSpec { blocks: intro });

    // 3. Historiene.
    for page in &album.pages {
        let aspects: Vec<f32> = page.photos.iter().map(PhotoSource::aspect).collect();
        let frames = layout::frames(page.kind, &aspects);
        let mut blocks = Vec::new();
        for (photo, frame) in page.photos.iter().zip(frames) {
            placed.push(Placed {
                photo: photo.clone(),
                frame,
            });
            blocks.push(Block::Photo(placed.len() - 1));
        }
        pages.push(PageSpec { blocks });
    }

    // 4. Partall sider: en tom side før baksiden ved behov.
    if (pages.len() + 1) % 2 == 1 {
        pages.push(PageSpec { blocks: Vec::new() });
    }

    // 5. Baksiden.
    let mut back = vec![Block::Fill(color::PAPER_TINT)];
    let mut y = TRIM_H * 0.45;
    if let Some(b) = &album.back {
        let h = TRIM_H * 0.6;
        placed.push(Placed {
            photo: b.clone(),
            frame: Frame {
                x: -BLEED,
                y: -BLEED,
                w: TRIM_W + 2.0 * BLEED,
                h: h + BLEED,
                fill: true,
            },
        });
        back.push(Block::Photo(placed.len() - 1));
        y = h + 22.0;
    }
    for line in text::wrap(Font::Italic, 14.0, &album.text.back, text_w * pt) {
        back.push(centered(
            Font::Italic,
            14.0,
            color::TEXT,
            TRIM_W / 2.0,
            y,
            &line,
        ));
        y += 7.0;
    }
    back.push(centered(
        Font::Body,
        8.0,
        color::TEXT_3,
        TRIM_W / 2.0,
        TRIM_H - 14.0,
        "Laget med Fo2Album.no",
    ));
    pages.push(PageSpec { blocks: back });

    // Bildene: lest, skalert og kodet i parallell, i grupper så fremdriften kan vises.
    use rayon::prelude::*;
    let total = placed.len();
    let mut encoded: Vec<Option<(Encoded, f32)>> = Vec::with_capacity(total);
    for chunk in placed.chunks(16) {
        encoded.extend(chunk.par_iter().map(encode).collect::<Vec<_>>());
        progress(encoded.len(), total);
    }
    let mut report = Report {
        pages: pages.len(),
        photos: total,
        ..Report::default()
    };
    for (p, e) in placed.iter().zip(&encoded) {
        match e {
            None => report.missing.push(p.photo.path.clone()),
            Some((_, ppi)) if *ppi < LOW_PPI => report.low_resolution.push(p.photo.path.clone()),
            _ => {}
        }
    }

    // PDF-en.
    let mut pdf = Pdf::new();
    let mut next = 1;
    let mut new_ref = || {
        let r = Ref::new(next);
        next += 1;
        r
    };
    let catalog = new_ref();
    let tree = new_ref();
    let image_refs: Vec<Ref> = (0..total).map(|_| new_ref()).collect();
    let page_refs: Vec<(Ref, Ref)> = (0..pages.len()).map(|_| (new_ref(), new_ref())).collect();
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids(page_refs.iter().map(|(p, _)| *p))
        .count(pages.len() as i32);
    for (i, e) in encoded.iter().enumerate() {
        if let Some((img, _)) = e {
            let mut x = pdf.image_xobject(image_refs[i], &img.jpeg);
            x.filter(Filter::DctDecode);
            x.width(img.width as i32);
            x.height(img.height as i32);
            x.color_space().device_rgb();
            x.bits_per_component(8);
            x.finish();
        }
    }
    let media = Rect::new(
        0.0,
        0.0,
        (TRIM_W + 2.0 * BLEED) * pt,
        (TRIM_H + 2.0 * BLEED) * pt,
    );
    let trim = Rect::new(
        BLEED * pt,
        BLEED * pt,
        (TRIM_W + BLEED) * pt,
        (TRIM_H + BLEED) * pt,
    );
    for (spec, (page_ref, content_ref)) in pages.iter().zip(&page_refs) {
        let mut c = Content::new();
        let mut used: Vec<usize> = Vec::new();
        for block in &spec.blocks {
            match block {
                Block::Fill(rgb) => {
                    c.set_fill_rgb(rgb[0], rgb[1], rgb[2]);
                    c.rect(media.x1, media.y1, media.x2, media.y2);
                    c.fill_nonzero();
                }
                Block::Photo(i) => {
                    let p = &placed[*i];
                    let f = p.frame;
                    if encoded[*i].is_none() {
                        let [r, g, b] = color::MISSING;
                        c.set_fill_rgb(r, g, b);
                        c.rect(pt_x(f.x), pt_y(f.y + f.h), f.w * pt, f.h * pt);
                        c.fill_nonzero();
                        continue;
                    }
                    let (dw, dh) = p.drawn();
                    let (dx, dy) = (f.x + (f.w - dw) / 2.0, f.y + (f.h - dh) / 2.0);
                    c.save_state();
                    // Bare det som er innenfor feltet, vises.
                    c.rect(pt_x(f.x), pt_y(f.y + f.h), f.w * pt, f.h * pt);
                    c.clip_nonzero();
                    c.end_path();
                    c.transform([dw * pt, 0.0, 0.0, dh * pt, pt_x(dx), pt_y(dy + dh)]);
                    let name = format!("B{i}");
                    c.x_object(Name(name.as_bytes()));
                    c.restore_state();
                    used.push(*i);
                }
                Block::Text {
                    font,
                    size,
                    color,
                    x,
                    y,
                    text,
                } => {
                    c.set_fill_rgb(color[0], color[1], color[2]);
                    text::draw(&mut c, *font, *size, pt_x(*x), pt_y(*y), text);
                }
            }
        }
        let names: Vec<String> = used.iter().map(|i| format!("B{i}")).collect();
        let mut page = pdf.page(*page_ref);
        page.media_box(media);
        page.bleed_box(media);
        page.trim_box(trim);
        page.parent(tree);
        page.contents(*content_ref);
        {
            let mut res = page.resources();
            let mut xo = res.x_objects();
            for (i, n) in used.iter().zip(&names) {
                xo.pair(Name(n.as_bytes()), image_refs[*i]);
            }
        }
        page.finish();
        pdf.stream(*content_ref, &c.finish());
    }
    Ok((pdf.finish(), report))
}
