import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { createRecipesBulk, importPhoto, type BulkImportResult } from "../lib/api";
import { parseRecipesJson } from "../lib/parseRecipesJson";
import styles from "./Importer.module.css";

const sampleJson = `[
  {
    "nom": "Tarte aux pommes normande",
    "ingredients": [
      {"quantite":"6","unite":"—","aliment":"pommes"},
      {"quantite":"200","unite":"g","aliment":"farine"}
    ],
    "etapes": ["Éplucher les pommes...", "..."]
  },
  { "nom": "Gratin dauphinois", "...": "..." }
]`;

export default function Importer() {
  const [tab, setTab] = useState<"photo" | "json">("photo");

  // Import photo
  const [fileName, setFileName] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();

  const importMutation = useMutation({
    mutationFn: (file: File) => importPhoto(file),
    onSuccess: (draft) => navigate("/ajouter", { state: draft }),
  });

  function handleFile(file: File) {
    setFileName(file.name);
    importMutation.mutate(file);
  }

  function handleDrop(e: React.DragEvent) {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files[0];
    if (file) handleFile(file);
  }

  // Import JSON
  const [jsonText, setJsonText] = useState("");
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [expanded, setExpanded] = useState<number | null>(null);
  const jsonFileInputRef = useRef<HTMLInputElement>(null);

  const parsed = useMemo(() => parseRecipesJson(jsonText), [jsonText]);

  useEffect(() => {
    setSelected(new Set(parsed.recipes.map((_, i) => i)));
    setExpanded(null);
  }, [parsed.recipes]);

  function toggleSelected(index: number) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  }

  function loadJsonFile(file: File) {
    const reader = new FileReader();
    reader.onload = () => setJsonText(String(reader.result ?? ""));
    reader.readAsText(file);
  }

  const bulkMutation = useMutation({
    mutationFn: () => createRecipesBulk(parsed.recipes.filter((_, i) => selected.has(i))),
  });

  const results = bulkMutation.data;
  const successCount = results?.filter((r) => r.success).length ?? 0;

  return (
    <>
      <div>
        <h1 className={styles.title}>Importer une recette</h1>
        <p className={styles.subtitle}>
          Depuis une photo unique, ou depuis un JSON généré par Claude à partir d'un livre PDF.
        </p>
      </div>

      <div className={styles.tabs}>
        <button className={tab === "photo" ? styles.tabActive : styles.tab} onClick={() => setTab("photo")}>
          Import photo
        </button>
        <button className={tab === "json" ? styles.tabActive : styles.tab} onClick={() => setTab("json")}>
          Import JSON (livre PDF)
        </button>
      </div>

      {tab === "photo" ? (
        <div className={styles.photoPanel}>
          {!importMutation.isPending && (
            <div
              className={dragOver ? styles.dropzoneActive : styles.dropzone}
              onClick={() => fileInputRef.current?.click()}
              onDragOver={(e) => {
                e.preventDefault();
                setDragOver(true);
              }}
              onDragLeave={() => setDragOver(false)}
              onDrop={handleDrop}
            >
              <input
                ref={fileInputRef}
                type="file"
                accept="image/*"
                capture="environment"
                hidden
                onChange={(e) => e.target.files?.[0] && handleFile(e.target.files[0])}
              />
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#C1592F" strokeWidth={1.8}>
                <rect x="3" y="3" width="18" height="18" rx="3" />
                <circle cx="9" cy="9" r="2" />
                <path d="m21 15-5-5L5 21" />
              </svg>
              <p className={styles.dropzoneTitle}>Glissez une photo, ou cliquez pour choisir un fichier</p>
              <p className={styles.dropzoneHint}>JPG, PNG — recette manuscrite ou imprimée</p>
            </div>
          )}

          {importMutation.isError && (
            <p className={styles.errorText}>
              Échec du traitement de la photo. Réessayez, ou vérifiez que le serveur OCR/IA est joignable.
            </p>
          )}

          {(importMutation.isPending || fileName) && !importMutation.isError && (
            <div className={styles.progress}>
              <p className={styles.progressLabel}>Traitement — {fileName}</p>
              <div className={styles.progressSteps}>
                <div className={styles.progressStep}>
                  <span className={styles.stepDoneCircle}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" strokeWidth={3} strokeLinecap="round" strokeLinejoin="round">
                      <path d="M20 6 9 17l-5-5" />
                    </svg>
                  </span>
                  <span className={styles.stepDoneLabel}>Photo reçue</span>
                </div>
                <div className={styles.progressLineDone} />
                <div className={styles.progressStep}>
                  <span className={importMutation.isPending ? styles.stepActiveCircle : styles.stepDoneCircle}>
                    {importMutation.isPending ? (
                      "2"
                    ) : (
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" strokeWidth={3} strokeLinecap="round" strokeLinejoin="round">
                        <path d="M20 6 9 17l-5-5" />
                      </svg>
                    )}
                  </span>
                  <span className={importMutation.isPending ? styles.stepActiveLabel : styles.stepDoneLabel}>
                    Lecture OCR &amp; structuration IA…
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className={styles.jsonPanel}>
          <div className={styles.jsonColumns}>
            <div className={styles.jsonInputColumn}>
              <label className={styles.label}>Coller le JSON généré par Claude</label>
              <textarea
                className={styles.textarea}
                value={jsonText}
                onChange={(e) => setJsonText(e.target.value)}
                placeholder={sampleJson}
              />
              <input
                ref={jsonFileInputRef}
                type="file"
                accept="application/json,.json"
                hidden
                onChange={(e) => e.target.files?.[0] && loadJsonFile(e.target.files[0])}
              />
              <button className={styles.loadFileButton} onClick={() => jsonFileInputRef.current?.click()}>
                Charger un fichier .json
              </button>
              {parsed.error && <p className={styles.errorText}>{parsed.error}</p>}
            </div>
            <div className={styles.detectedColumn}>
              <label className={styles.label}>
                {parsed.recipes.length} recette{parsed.recipes.length !== 1 && "s"} détectée
                {parsed.recipes.length !== 1 && "s"}
              </label>
              <div className={styles.detectedList}>
                {parsed.recipes.map((recipe, i) => (
                  <div key={i}>
                    <div className={styles.detectedRow}>
                      <input
                        type="checkbox"
                        checked={selected.has(i)}
                        onChange={() => toggleSelected(i)}
                        className={styles.checkbox}
                      />
                      <span className={styles.detectedName}>{recipe.nom}</span>
                      <button
                        type="button"
                        className={styles.detectedView}
                        onClick={() => setExpanded((current) => (current === i ? null : i))}
                      >
                        {expanded === i ? "Masquer" : "Voir"}
                      </button>
                    </div>
                    {expanded === i && (
                      <div className={styles.preview}>
                        <span>
                          {recipe.ingredients.length} ingrédient{recipe.ingredients.length !== 1 && "s"}
                        </span>
                        <ul className={styles.previewList}>
                          {recipe.ingredients.slice(0, 6).map((ing, j) => (
                            <li key={j}>
                              {[ing.quantite, ing.unite, ing.aliment].filter(Boolean).join(" ")}
                            </li>
                          ))}
                          {recipe.ingredients.length > 6 && <li>…</li>}
                        </ul>
                        <span>
                          {recipe.etapes.length} étape{recipe.etapes.length !== 1 && "s"}
                        </span>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </div>
          <div className={styles.jsonFooter}>
            {results ? (
              <div className={styles.resultsSummary}>
                <span>
                  {successCount}/{results.length} recette{results.length !== 1 && "s"} importée
                  {successCount !== 1 && "s"}.
                </span>
                {results
                  .filter((r) => !r.success)
                  .map((r: BulkImportResult) => (
                    <span key={r.nom} className={styles.errorText}>
                      {r.nom} : {r.error}
                    </span>
                  ))}
                <button className={styles.importButton} onClick={() => navigate("/")}>
                  Voir la liste des recettes
                </button>
              </div>
            ) : (
              <button
                className={styles.importButton}
                disabled={selected.size === 0 || bulkMutation.isPending}
                onClick={() => bulkMutation.mutate()}
              >
                {bulkMutation.isPending
                  ? "Import en cours…"
                  : `Importer les recettes sélectionnées (${selected.size})`}
              </button>
            )}
          </div>
        </div>
      )}
    </>
  );
}
