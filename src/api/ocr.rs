//! OCR d'une photo de recette, entièrement dans le navigateur : décodage par
//! le navigateur, prétraitement identique à l'ancien serveur (Pillow),
//! Tesseract (tesseract.js) puis reconstruction de l'ordre de lecture des
//! fiches à deux colonnes.

use std::io::Cursor;

use image::imageops::FilterType;
use image::{GrayImage, ImageFormat};
use indexmap::IndexMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::api::error::ApiError;
use crate::py;

/// Langue Tesseract : `fra.traineddata`, servi avec l'application.
const TESSERACT_LANG: &str = "fra";

#[wasm_bindgen(module = "/src/js/ocr.js")]
extern "C" {
    #[wasm_bindgen(js_name = recognizeTsv, catch)]
    async fn recognize_tsv(png: js_sys::Uint8Array, lang: &str) -> Result<JsValue, JsValue>;
}

pub async fn extract_text(file: &web_sys::File) -> Result<String, ApiError> {
    let (rgba, width, height) = decode(file).await?;
    let image = preprocess(&rgba, width, height);
    let png = encode_png(&image)?;
    let tsv = recognize_tsv(js_sys::Uint8Array::from(png.as_slice()), TESSERACT_LANG)
        .await
        .map_err(js_error)?
        .as_string()
        .ok_or_else(|| ApiError::Internal("Tesseract n'a pas renvoyé de TSV".to_owned()))?;
    let lines = group_lines(&parse_tsv(&tsv)?);
    let lines = split_intra_line_gaps(lines, image.width());
    Ok(reading_order_text(&lines, image.width()))
}

fn js_error(err: JsValue) -> ApiError {
    let message = err
        .dyn_ref::<js_sys::Error>()
        .map(|err| String::from(err.message()))
        .or_else(|| err.as_string())
        .unwrap_or_else(|| format!("{err:?}"));
    ApiError::Internal(message)
}

/// Pixels RGBA de la photo, décodée par le navigateur (tous les formats qu'il
/// sait afficher), orientation EXIF appliquée comme `ImageOps.exif_transpose`.
async fn decode(file: &web_sys::File) -> Result<(Vec<u8>, u32, u32), ApiError> {
    let unavailable = || ApiError::Internal("navigateur indisponible".to_owned());
    let window = web_sys::window().ok_or_else(unavailable)?;
    let options = web_sys::ImageBitmapOptions::new();
    options.set_image_orientation(web_sys::ImageOrientation::FromImage);
    let promise = window
        .create_image_bitmap_with_blob_and_image_bitmap_options(file, &options)
        .map_err(js_error)?;
    let bitmap: web_sys::ImageBitmap = JsFuture::from(promise)
        .await
        .map_err(|_| ApiError::Internal("cannot identify image file".to_owned()))?
        .unchecked_into();
    let (width, height) = (bitmap.width(), bitmap.height());

    let canvas: web_sys::HtmlCanvasElement = window
        .document()
        .ok_or_else(unavailable)?
        .create_element("canvas")
        .map_err(js_error)?
        .unchecked_into();
    canvas.set_width(width);
    canvas.set_height(height);
    let context: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(js_error)?
        .ok_or_else(unavailable)?
        .unchecked_into();
    context
        .draw_image_with_image_bitmap(&bitmap, 0.0, 0.0)
        .map_err(js_error)?;
    bitmap.close();
    let pixels = context
        .get_image_data(0.0, 0.0, f64::from(width), f64::from(height))
        .map_err(js_error)?;
    Ok((pixels.data().0, width, height))
}

/// Même chaîne que l'ancien serveur : `convert("L")`, `autocontrast`, puis
/// agrandissement ×2 (Lanczos) des images de moins de 2000 px de côté.
fn preprocess(rgba: &[u8], width: u32, height: u32) -> GrayImage {
    let mut image = to_luma(rgba, width, height);
    autocontrast(&mut image);
    if width.max(height) < 2000 {
        image = image::imageops::resize(&image, width * 2, height * 2, FilterType::Lanczos3);
    }
    image
}

/// `Image.convert("L")` de Pillow : luminance ITU-R 601-2, en virgule fixe
/// (la transparence est ignorée).
fn to_luma(rgba: &[u8], width: u32, height: u32) -> GrayImage {
    let luma = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, _]| {
            let [r, g, b] = [r, g, b].map(u32::from);
            ((r * 19_595 + g * 38_470 + b * 7_471 + 0x8000) >> 16) as u8
        })
        .collect();
    GrayImage::from_raw(width, height, luma).expect("un pixel RGBA par pixel")
}

/// `ImageOps.autocontrast` (sans écrêtage) : étire les niveaux présents sur 0–255.
fn autocontrast(image: &mut GrayImage) {
    let mut histogram = [0u64; 256];
    for pixel in image.pixels() {
        histogram[usize::from(pixel[0])] += 1;
    }
    let (Some(lo), Some(hi)) = (
        histogram.iter().position(|&n| n > 0),
        histogram.iter().rposition(|&n| n > 0),
    ) else {
        return;
    };
    if hi <= lo {
        return;
    }
    let scale = 255.0 / (hi - lo) as f64;
    let offset = -(lo as f64) * scale;
    let lut: [u8; 256] =
        std::array::from_fn(|ix| ((ix as f64 * scale + offset) as i64).clamp(0, 255) as u8);
    for pixel in image.pixels_mut() {
        pixel[0] = lut[usize::from(pixel[0])];
    }
}

fn encode_png(image: &GrayImage) -> Result<Vec<u8>, ApiError> {
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|err| ApiError::Internal(err.to_string()))?;
    Ok(png)
}

/// Ligne du TSV de Tesseract (colonnes utiles seulement).
struct TsvRow {
    block_num: i64,
    par_num: i64,
    line_num: i64,
    word_num: i64,
    left: i64,
    top: i64,
    width: i64,
    text: String,
}

/// Colonnes du TSV de Tesseract. La ligne de commande les écrit en en-tête ;
/// tesseract.js (`GetTSVText`) renvoie les lignes sans en-tête.
const TSV_HEADER: [&str; 12] = [
    "level",
    "page_num",
    "block_num",
    "par_num",
    "line_num",
    "word_num",
    "left",
    "top",
    "width",
    "height",
    "conf",
    "text",
];

/// Équivalent de `image_to_data(..., output_type=Output.DICT)` : colonnes
/// numériques converties par `int(float(…))`, texte en dernière colonne.
fn parse_tsv(tsv: &str) -> Result<Vec<TsvRow>, ApiError> {
    let tsv = py::strip(tsv);
    if tsv.is_empty() {
        // Dictionnaire vide côté pytesseract : `data["text"]` lève KeyError.
        return Err(ApiError::Data("'text'".to_owned()));
    }
    let mut rows: Vec<Vec<&str>> = tsv
        .split('\n')
        .map(|row| row.split('\t').collect())
        .collect();
    let header = if rows[0].first() == Some(&"level") {
        rows.remove(0)
    } else {
        TSV_HEADER.to_vec()
    };
    if rows.is_empty() {
        return Err(ApiError::Data("'text'".to_owned()));
    }
    let text_column = header.len() - 1;
    let column = |name: &str| {
        header
            .iter()
            .position(|head| *head == name)
            .ok_or_else(|| ApiError::Data(format!("'{name}'")))
    };
    let [block, par, line, word, left, top, width] = [
        "block_num",
        "par_num",
        "line_num",
        "word_num",
        "left",
        "top",
        "width",
    ]
    .map(column);
    let (block, par, line, word, left, top, width) =
        (block?, par?, line?, word?, left?, top?, width?);

    rows.iter()
        .map(|row| {
            let number = |index: usize| -> Result<i64, ApiError> {
                let cell = row.get(index).copied().unwrap_or_default();
                py::strip(cell)
                    .parse::<f64>()
                    .map(|value| value as i64)
                    .map_err(|_| {
                        ApiError::Data(format!("invalid literal for int(): {}", py::repr_str(cell)))
                    })
            };
            Ok(TsvRow {
                block_num: number(block)?,
                par_num: number(par)?,
                line_num: number(line)?,
                word_num: number(word)?,
                left: number(left)?,
                top: number(top)?,
                width: number(width)?,
                text: row.get(text_column).copied().unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
struct Word {
    text: String,
    left: i64,
    right: i64,
    top: i64,
}

#[derive(Debug, Clone)]
struct Line {
    text: String,
    left: i64,
    right: i64,
    top: i64,
}

/// Regroupe les mots OCR par ligne (bloc/paragraphe/ligne Tesseract).
fn group_lines(rows: &[TsvRow]) -> Vec<Vec<Word>> {
    let mut groups: IndexMap<(i64, i64, i64), Vec<&TsvRow>> = IndexMap::new();
    for row in rows {
        if py::strip(&row.text).is_empty() {
            continue;
        }
        groups
            .entry((row.block_num, row.par_num, row.line_num))
            .or_default()
            .push(row);
    }

    let mut lines: Vec<Vec<Word>> = groups
        .into_values()
        .map(|mut members| {
            members.sort_by_key(|row| row.word_num);
            members
                .into_iter()
                .map(|row| Word {
                    text: row.text.clone(),
                    left: row.left,
                    right: row.left + row.width,
                    top: row.top,
                })
                .collect()
        })
        .collect();
    lines.sort_by_key(|words| words.iter().map(|word| word.top).min());
    lines
}

/// Scinde une ligne Tesseract si un grand espace horizontal trahit deux colonnes fusionnées.
fn split_intra_line_gaps(lines: Vec<Vec<Word>>, image_width: u32) -> Vec<Line> {
    let gap_threshold = f64::max(50.0, 0.10 * f64::from(image_width));

    let mut result = Vec::new();
    for mut words in lines {
        words.sort_by_key(|word| word.left);
        let mut segment: Vec<Word> = Vec::new();
        for word in words {
            if let Some(previous) = segment.last()
                && (word.left - previous.right) as f64 > gap_threshold
            {
                result.push(words_to_line(&segment));
                segment.clear();
            }
            segment.push(word);
        }
        result.push(words_to_line(&segment));
    }
    result
}

fn words_to_line(words: &[Word]) -> Line {
    Line {
        text: words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        left: words.iter().map(|word| word.left).min().unwrap_or_default(),
        right: words
            .iter()
            .map(|word| word.right)
            .max()
            .unwrap_or_default(),
        top: words.iter().map(|word| word.top).min().unwrap_or_default(),
    }
}

/// Reconstruit un ordre de lecture correct pour les mises en page à deux colonnes.
///
/// Les fiches recette ont souvent un bandeau pleine largeur (titre, intro) suivi
/// d'une zone à deux colonnes (étapes / ingrédients) que Tesseract lit sinon en
/// entrelaçant les lignes des deux colonnes.
fn reading_order_text(lines: &[Line], image_width: u32) -> String {
    let wide_threshold = 0.65 * f64::from(image_width);
    let gap_threshold = 0.15 * f64::from(image_width);

    let mut segments: Vec<String> = Vec::new();
    let mut column_buffer: Vec<&Line> = Vec::new();

    for line in lines {
        if (line.right - line.left) as f64 >= wide_threshold {
            if !column_buffer.is_empty() {
                segments.push(split_columns(&column_buffer, gap_threshold));
                column_buffer.clear();
            }
            segments.push(line.text.clone());
        } else {
            column_buffer.push(line);
        }
    }

    if !column_buffer.is_empty() {
        segments.push(split_columns(&column_buffer, gap_threshold));
    }

    segments.join("\n")
}

fn split_columns(buffer: &[&Line], gap_threshold: f64) -> String {
    if buffer.len() == 1 {
        return buffer[0].text.clone();
    }

    let mut lefts: Vec<i64> = buffer.iter().map(|line| line.left).collect();
    lefts.sort_unstable();
    lefts.dedup();
    let (mut split_at, mut best_gap) = (None, 0);
    for pair in lefts.windows(2) {
        let gap = pair[1] - pair[0];
        if gap > best_gap {
            best_gap = gap;
            split_at = Some((pair[0] + pair[1]) as f64 / 2.0);
        }
    }

    let by_top = |mut lines: Vec<&Line>| {
        lines.sort_by_key(|line| line.top);
        lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    match split_at {
        Some(split_at) if best_gap as f64 >= gap_threshold => {
            let (left_col, right_col): (Vec<&Line>, Vec<&Line>) = buffer
                .iter()
                .partition(|line| (line.left as f64) < split_at);
            format!("{}\n{}", by_top(left_col), by_top(right_col))
        }
        _ => by_top(buffer.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Valeurs attendues calculées avec Pillow (`ImageOps.autocontrast`).
    #[test]
    fn autocontrast_matches_pillow() {
        for (levels, expected) in [
            (vec![50, 100, 200], vec![0, 85, 255]),
            (vec![3, 17, 90, 91, 250], vec![0, 14, 89, 90, 255]),
            (vec![7, 7], vec![7, 7]),
        ] {
            let mut image = GrayImage::from_raw(levels.len() as u32, 1, levels).unwrap();
            autocontrast(&mut image);
            assert_eq!(image.into_raw(), expected);
        }
    }

    /// Valeurs attendues calculées avec Pillow (`Image.convert("L")`).
    #[test]
    fn luma_matches_pillow() {
        let mut rgba = Vec::new();
        for r in [0, 1, 127, 128, 254, 255] {
            for g in [0, 13, 255] {
                for b in [0, 99, 255] {
                    rgba.extend([r, g, b, 255]);
                }
            }
        }
        let expected = [
            0, 11, 29, 8, 19, 37, 150, 161, 179, 0, 12, 29, 8, 19, 37, 150, 161, 179, 38, 49, 67,
            46, 57, 75, 188, 199, 217, 38, 50, 67, 46, 57, 75, 188, 199, 217, 76, 87, 105, 84, 95,
            113, 226, 237, 255, 76, 88, 105, 84, 95, 113, 226, 237, 255,
        ];
        assert_eq!(to_luma(&rgba, 54, 1).into_raw(), expected);
    }

    #[test]
    fn small_images_are_upscaled_twice() {
        let rgba = [50, 50, 50, 255, 200, 200, 200, 255].repeat(50);
        let image = preprocess(&rgba, 10, 10);
        assert_eq!(image.dimensions(), (20, 20));
        assert_eq!(
            preprocess(&vec![0; 2000 * 4], 2000, 1).dimensions(),
            (2000, 1)
        );
    }

    /// TSV Tesseract et texte attendu produits par les fonctions de mise en
    /// page de l'ancienne implémentation Python.
    #[test]
    fn reading_order_matches_python_fixtures() {
        let fixtures = [
            (
                include_str!("../../tests/fixtures/ocr/fiche_deux_colonnes.tsv"),
                include_str!("../../tests/fixtures/ocr/fiche_deux_colonnes.txt"),
            ),
            (
                include_str!("../../tests/fixtures/ocr/aleatoire_7.tsv"),
                include_str!("../../tests/fixtures/ocr/aleatoire_7.txt"),
            ),
            (
                include_str!("../../tests/fixtures/ocr/aleatoire_42.tsv"),
                include_str!("../../tests/fixtures/ocr/aleatoire_42.txt"),
            ),
        ];
        for (tsv, expected) in fixtures {
            // Les fichiers peuvent avoir été extraits en CRLF par git sous Windows.
            let (tsv, expected) = (tsv.replace("\r\n", "\n"), expected.replace("\r\n", "\n"));
            let (width, expected) = expected.split_once('\n').unwrap();
            let width: u32 = width.parse().unwrap();
            let lines = split_intra_line_gaps(group_lines(&parse_tsv(&tsv).unwrap()), width);
            assert_eq!(reading_order_text(&lines, width), expected);

            // tesseract.js renvoie le même TSV, sans la ligne d'en-tête.
            let (_, headerless) = tsv.split_once('\n').unwrap();
            let lines = split_intra_line_gaps(group_lines(&parse_tsv(headerless).unwrap()), width);
            assert_eq!(reading_order_text(&lines, width), expected);
        }
    }
}
