// Pont vers tesseract.js, chargé à la demande depuis /tesseract/ (fichiers
// servis avec l'application, voir vendor/tesseract/README.md). Le worker est
// créé au premier import puis réutilisé.

const BASE = "/tesseract";
let workerPromise = null;

function loadScript(src) {
  return new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = src;
    script.onload = resolve;
    script.onerror = () => reject(new Error(`Impossible de charger ${src}`));
    document.head.appendChild(script);
  });
}

function getWorker(lang) {
  if (!workerPromise) {
    workerPromise = (async () => {
      if (!window.Tesseract) {
        await loadScript(`${BASE}/tesseract.min.js`);
      }
      // OEM 1 : moteur LSTM, celui qu'utilisait Tesseract côté serveur.
      const worker = await window.Tesseract.createWorker(lang, 1, {
        workerPath: `${BASE}/worker.min.js`,
        corePath: `${BASE}/core`,
        langPath: `${BASE}/lang`,
        gzip: false,
      });
      // Segmentation automatique de la page (PSM 3), comme la ligne de
      // commande tesseract ; tesseract.js part sinon d'un bloc unique, ce qui
      // casse la lecture des fiches à deux colonnes.
      await worker.setParameters({ tessedit_pageseg_mode: "3" });
      return worker;
    })().catch((err) => {
      workerPromise = null;
      throw err;
    });
  }
  return workerPromise;
}

export async function recognizeTsv(png, lang) {
  const worker = await getWorker(lang);
  const { data } = await worker.recognize(new Blob([png], { type: "image/png" }), {}, { tsv: true });
  return data.tsv;
}
