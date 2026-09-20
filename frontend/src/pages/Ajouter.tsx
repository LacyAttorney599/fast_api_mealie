import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { createRecipe, type RecipeDraft } from "../lib/api";
import styles from "./Ajouter.module.css";

interface Ingredient {
  qty: string;
  unit: string;
  food: string;
}

function fromDraft(draft: RecipeDraft | undefined) {
  if (!draft) {
    return { nom: "", ingredients: [{ qty: "", unit: "", food: "" }], steps: [""] };
  }
  return {
    nom: draft.nom,
    ingredients:
      draft.ingredients.length > 0
        ? draft.ingredients.map((ing) => ({
            qty: ing.quantite !== null ? String(ing.quantite) : "",
            unit: ing.unite ?? "",
            food: ing.aliment,
          }))
        : [{ qty: "", unit: "", food: "" }],
    steps: draft.etapes.length > 0 ? draft.etapes : [""],
  };
}

export default function Ajouter() {
  const navigate = useNavigate();
  const location = useLocation();
  const initial = fromDraft(location.state as RecipeDraft | undefined);

  const [nom, setNom] = useState(initial.nom);
  const [ingredients, setIngredients] = useState<Ingredient[]>(initial.ingredients);
  const [steps, setSteps] = useState<string[]>(initial.steps);

  const saveMutation = useMutation({
    mutationFn: (draft: RecipeDraft) => createRecipe(draft),
    onSuccess: ({ slug }) => navigate(`/recette/${slug}`),
  });

  function updateIngredient(index: number, patch: Partial<Ingredient>) {
    setIngredients((current) => current.map((ing, i) => (i === index ? { ...ing, ...patch } : ing)));
  }

  function updateStep(index: number, text: string) {
    setSteps((current) => current.map((step, i) => (i === index ? text : step)));
  }

  function handleSave() {
    const draft: RecipeDraft = {
      nom,
      ingredients: ingredients
        .filter((ing) => ing.food.trim())
        .map((ing) => ({
          quantite: ing.qty.trim() ? Number(ing.qty.replace(",", ".")) : null,
          unite: ing.unit.trim() || null,
          aliment: ing.food.trim(),
        })),
      etapes: steps.map((step) => step.trim()).filter(Boolean),
    };
    saveMutation.mutate(draft);
  }

  return (
    <>
      <div>
        <h1 className={styles.title}>Nouvelle recette</h1>
        <p className={styles.subtitle}>Saisissez les informations, ou importez-les depuis une photo ou un livre PDF.</p>
      </div>

      <div className={styles.tabs}>
        <span className={styles.tabActive}>Saisie manuelle</span>
        <Link to="/importer" className={styles.tab}>
          Import photo
        </Link>
        <Link to="/importer" className={styles.tab}>
          Import JSON (livre PDF)
        </Link>
      </div>

      <div className={styles.form}>
        <div className={styles.topRow}>
          <div className={styles.field}>
            <label className={styles.label}>Nom de la recette</label>
            <input className={styles.input} value={nom} onChange={(e) => setNom(e.target.value)} />
          </div>
          <div className={styles.field}>
            <label className={styles.label}>Importer depuis une URL</label>
            <div className={styles.urlRow}>
              <input className={styles.input} placeholder="https://…" />
              <button className={styles.extractButton}>Extraire</button>
            </div>
          </div>
        </div>

        <div className={styles.columns}>
          <div className={styles.ingredientsColumn}>
            <div className={styles.columnHeader}>
              <label className={styles.label}>Ingrédients</label>
              <button
                className={styles.addLink}
                onClick={() => setIngredients((current) => [...current, { qty: "", unit: "", food: "" }])}
              >
                + Ajouter un ingrédient
              </button>
            </div>
            <div className={styles.ingredientsHead}>
              <span>QTÉ</span>
              <span>UNITÉ</span>
              <span>ALIMENT</span>
            </div>
            {ingredients.map((ing, i) => (
              <div key={i} className={styles.ingredientRow}>
                <input
                  className={styles.smallInput}
                  value={ing.qty}
                  onChange={(e) => updateIngredient(i, { qty: e.target.value })}
                />
                <input
                  className={styles.smallInput}
                  value={ing.unit}
                  onChange={(e) => updateIngredient(i, { unit: e.target.value })}
                />
                <input
                  className={styles.smallInput}
                  value={ing.food}
                  onChange={(e) => updateIngredient(i, { food: e.target.value })}
                />
              </div>
            ))}
          </div>

          <div className={styles.stepsColumn}>
            <div className={styles.columnHeader}>
              <label className={styles.label}>Étapes</label>
              <button className={styles.addLink} onClick={() => setSteps((current) => [...current, ""])}>
                + Ajouter une étape
              </button>
            </div>
            {steps.map((step, i) => (
              <div key={i} className={styles.stepRow}>
                <span className={styles.stepNumber}>{i + 1}</span>
                <textarea
                  className={styles.textarea}
                  value={step}
                  onChange={(e) => updateStep(i, e.target.value)}
                />
              </div>
            ))}
          </div>
        </div>
      </div>

      {saveMutation.isError && <p className={styles.error}>Échec de l'enregistrement vers Mealie.</p>}

      <div className={styles.footer}>
        <button className={styles.cancelButton} onClick={() => navigate("/")}>
          Annuler
        </button>
        <button className={styles.saveButton} disabled={saveMutation.isPending} onClick={handleSave}>
          {saveMutation.isPending ? "Enregistrement…" : "Enregistrer la recette"}
        </button>
      </div>
    </>
  );
}
