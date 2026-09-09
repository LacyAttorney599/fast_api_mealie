import io
from dataclasses import dataclass

import pytesseract
from PIL import Image, ImageOps
from pytesseract import Output

from app.config import settings


def extract_text(image_bytes: bytes) -> str:
    image = Image.open(io.BytesIO(image_bytes))
    image = _preprocess(image)
    data = pytesseract.image_to_data(image, lang=settings.tesseract_lang, output_type=Output.DICT)
    lines = _group_lines(data)
    lines = _split_intra_line_gaps(lines, image.width)
    return _reading_order_text(lines, image.width)


def _preprocess(image: Image.Image) -> Image.Image:
    image = ImageOps.exif_transpose(image)
    image = image.convert("L")
    image = ImageOps.autocontrast(image)
    if max(image.size) < 2000:
        image = image.resize((image.width * 2, image.height * 2), Image.LANCZOS)
    return image


@dataclass
class _Line:
    text: str
    left: int
    right: int
    top: int


@dataclass
class _Word:
    text: str
    left: int
    right: int
    top: int


def _group_lines(data: dict) -> list[list[_Word]]:
    """Regroupe les mots OCR par ligne (bloc/paragraphe/ligne Tesseract)."""
    groups: dict[tuple[int, int, int], list[int]] = {}
    for i, word in enumerate(data["text"]):
        if not word.strip():
            continue
        key = (data["block_num"][i], data["par_num"][i], data["line_num"][i])
        groups.setdefault(key, []).append(i)

    lines = []
    for indices in groups.values():
        indices.sort(key=lambda i: data["word_num"][i])
        words = [
            _Word(
                text=data["text"][i],
                left=data["left"][i],
                right=data["left"][i] + data["width"][i],
                top=data["top"][i],
            )
            for i in indices
        ]
        lines.append(words)
    return sorted(lines, key=lambda words: min(w.top for w in words))


def _split_intra_line_gaps(lines: list[list[_Word]], image_width: int) -> list[_Line]:
    """Scinde une ligne Tesseract si un grand espace horizontal trahit deux colonnes fusionnées."""
    gap_threshold = max(50, 0.10 * image_width)

    result = []
    for words in lines:
        words = sorted(words, key=lambda w: w.left)
        segment = [words[0]]
        for previous, word in zip(words, words[1:]):
            if word.left - previous.right > gap_threshold:
                result.append(_words_to_line(segment))
                segment = []
            segment.append(word)
        result.append(_words_to_line(segment))
    return result


def _words_to_line(words: list[_Word]) -> _Line:
    return _Line(
        text=" ".join(w.text for w in words),
        left=min(w.left for w in words),
        right=max(w.right for w in words),
        top=min(w.top for w in words),
    )


def _reading_order_text(lines: list[_Line], image_width: int) -> str:
    """Reconstruit un ordre de lecture correct pour les mises en page à deux colonnes.

    Les fiches recette ont souvent un bandeau pleine largeur (titre, intro) suivi
    d'une zone à deux colonnes (étapes / ingrédients) que Tesseract lit sinon en
    entrelaçant les lignes des deux colonnes.
    """
    wide_threshold = 0.65 * image_width
    gap_threshold = 0.15 * image_width

    segments: list[str] = []
    column_buffer: list[_Line] = []

    for line in lines:
        if (line.right - line.left) >= wide_threshold:
            if column_buffer:
                segments.append(_split_columns(column_buffer, gap_threshold))
                column_buffer = []
            segments.append(line.text)
        else:
            column_buffer.append(line)

    if column_buffer:
        segments.append(_split_columns(column_buffer, gap_threshold))

    return "\n".join(segments)


def _split_columns(buffer: list[_Line], gap_threshold: float) -> str:
    if len(buffer) == 1:
        return buffer[0].text

    lefts = sorted(set(line.left for line in buffer))
    split_at, best_gap = None, 0
    for a, b in zip(lefts, lefts[1:]):
        if (gap := b - a) > best_gap:
            best_gap, split_at = gap, (a + b) / 2

    if split_at is None or best_gap < gap_threshold:
        return "\n".join(line.text for line in sorted(buffer, key=lambda line: line.top))

    left_col = sorted((line for line in buffer if line.left < split_at), key=lambda line: line.top)
    right_col = sorted((line for line in buffer if line.left >= split_at), key=lambda line: line.top)
    return "\n".join(line.text for line in left_col) + "\n" + "\n".join(line.text for line in right_col)
