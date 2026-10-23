#!/bin/sh
# Télécharge tesseract.js et le modèle français dans vendor/tesseract/, servi
# sous /tesseract/ par l'application (OCR de l'import photo, dans le
# navigateur). Versions et empreintes figées ; relancer ce script est sans effet
# si les fichiers sont déjà là.
#
# Le modèle est celui du paquet Debian tesseract-ocr-fra qu'utilisait l'ancien
# serveur : tessdata_fast 4.1.0.
set -eu

dest="${1:-vendor/tesseract}"
tesseract_js="https://cdn.jsdelivr.net/npm/tesseract.js@7.0.0/dist"
tesseract_core="https://cdn.jsdelivr.net/npm/tesseract.js-core@7.0.0"
tessdata="https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/4.1.0"

fetch() {
    url=$1
    target="$dest/$2"
    sha256=$3
    if [ -f "$target" ] && echo "$sha256  $target" | sha256sum -c - >/dev/null 2>&1; then
        return
    fi
    mkdir -p "$(dirname "$target")"
    curl -fsSL --retry 3 -o "$target.part" "$url"
    if ! echo "$sha256  $target.part" | sha256sum -c - >/dev/null 2>&1; then
        echo "Empreinte inattendue pour $url" >&2
        rm -f "$target.part"
        exit 1
    fi
    mv "$target.part" "$target"
    echo "$2"
}

fetch "$tesseract_js/tesseract.min.js" tesseract.min.js \
    000c27d9cd0def655f77b36c72a389c0ab13793aa31cb4d7aab56d09c0afbc7e
fetch "$tesseract_js/worker.min.js" worker.min.js \
    576b7df7e3393e137e51849357c9adb53fe7ac1bb69bfa06cf3d61520f182c6d
fetch "$tesseract_core/tesseract-core-lstm.wasm.js" core/tesseract-core-lstm.wasm.js \
    eef5f8b2f8e20e150680b20adaec4a60babafee3adbe8a94583c81fee46e8680
fetch "$tesseract_core/tesseract-core-simd-lstm.wasm.js" core/tesseract-core-simd-lstm.wasm.js \
    c58b46a4c796c0b8afccf77591d5b875b6896b45d402bbce8caa6f5362447b38
fetch "$tesseract_core/tesseract-core-relaxedsimd-lstm.wasm.js" core/tesseract-core-relaxedsimd-lstm.wasm.js \
    861a536cf9ef8e63cb644d57bab39c388f37f7d6b6f60024b741c5f6b39a59b3
fetch "$tessdata/fra.traineddata" lang/fra.traineddata \
    ced037562e8c80c13122dece28dd477d399af80911a28791a66a63ac1e3445ca
