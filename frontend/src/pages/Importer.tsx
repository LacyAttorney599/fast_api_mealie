import { useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { importPhoto } from "../lib/api";
import styles from "./Importer.module.css";

const detected = [
  "Tarte aux pommes normande",
  "Gratin dauphinois",
  "Poulet rôti aux herbes",
  "Soupe de potiron",
  "Clafoutis aux cerises",
  "Bœuf bourguignon",
  "Ratatouille",
];

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
              <textarea className={styles.textarea} defaultValue={sampleJson} />
              <button className={styles.loadFileButton}>Charger un fichier .json</button>
            </div>
            <div className={styles.detectedColumn}>
              <label className={styles.label}>{detected.length} recettes détectées</label>
              <div className={styles.detectedList}>
                {detected.map((name) => (
                  <div key={name} className={styles.detectedRow}>
                    <input type="checkbox" defaultChecked className={styles.checkbox} />
                    <span className={styles.detectedName}>{name}</span>
                    <span className={styles.detectedView}>Voir</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
          <div className={styles.jsonFooter}>
            <button className={styles.importButton}>Importer les recettes sélectionnées ({detected.length})</button>
          </div>
        </div>
      )}
    </>
  );
}
