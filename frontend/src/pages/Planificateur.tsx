import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import {
  createMealPlanEntry,
  deleteMealPlanEntry,
  fetchCurrentSeason,
  fetchMealPlan,
  fetchRecipes,
  fetchSeasonalRecipes,
  generateAiMealPlan,
  type MealPlanEntry,
  type MealType,
} from "../lib/api";
import { addDays, formatWeekRange, getMonday, getWeekDays, toISODate } from "../lib/dates";
import styles from "./Planificateur.module.css";

interface Slot {
  date: string;
  mealType: MealType;
}

function findEntry(entries: MealPlanEntry[] | undefined, date: string, mealType: MealType) {
  return entries?.find((entry) => entry.date === date && entry.entry_type === mealType);
}

interface RecipeOption {
  slug: string;
  name: string;
  seasonal: boolean;
}

function RecipeSearchPicker({
  recipes,
  onSelect,
  onCancel,
}: {
  recipes: RecipeOption[];
  onSelect: (slug: string) => void;
  onCancel: () => void;
}) {
  const [query, setQuery] = useState("");
  const filtered = recipes.filter((r) => r.name.toLowerCase().includes(query.toLowerCase())).slice(0, 8);

  return (
    <div
      className={styles.slotPicker}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node)) {
          onCancel();
        }
      }}
    >
      <input
        autoFocus
        className={styles.slotPickerInput}
        placeholder="Rechercher une recette…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => e.key === "Escape" && onCancel()}
      />
      <div className={styles.slotPickerList}>
        {filtered.map((recipe) => (
          <button key={recipe.slug} className={styles.slotPickerOption} onClick={() => onSelect(recipe.slug)}>
            {recipe.seasonal ? `🌱 ${recipe.name}` : recipe.name}
          </button>
        ))}
        {filtered.length === 0 && <span className={styles.slotPickerEmpty}>Aucun résultat</span>}
      </div>
    </div>
  );
}

export default function Planificateur() {
  const [weekStart, setWeekStart] = useState(() => getMonday(new Date()));
  const [addingSlot, setAddingSlot] = useState<Slot | null>(null);
  const queryClient = useQueryClient();

  const days = getWeekDays(weekStart);
  const isoStart = toISODate(weekStart);
  const isoEnd = days[6].iso;

  const { data: entries } = useQuery({
    queryKey: ["mealplan", isoStart, isoEnd],
    queryFn: () => fetchMealPlan(isoStart, isoEnd),
  });

  const { data: recipes } = useQuery({
    queryKey: ["recipes", ""],
    queryFn: () => fetchRecipes(),
  });

  // Calculé à la demande (pas sur l'écran Liste) : il faut le détail de
  // chaque recette pour savoir si elle est de saison, ce qui coûte un appel
  // par recette côté BFF — acceptable pour une visite ponctuelle du
  // planificateur, pas pour un chargement de liste répété.
  const { data: seasonalRecipes } = useQuery({
    queryKey: ["recipes-seasonal"],
    queryFn: fetchSeasonalRecipes,
  });

  const { data: currentSeason } = useQuery({
    queryKey: ["season-current"],
    queryFn: fetchCurrentSeason,
  });

  const seasonalSlugs = new Set(seasonalRecipes?.filter((r) => r.in_season).map((r) => r.slug));
  const sortedRecipes = recipes
    ? [...recipes].sort((a, b) => Number(seasonalSlugs.has(b.slug)) - Number(seasonalSlugs.has(a.slug)))
    : recipes;

  const mealplanKey = ["mealplan", isoStart, isoEnd];

  const createMutation = useMutation({
    mutationFn: ({ date, mealType, recipeSlug }: { date: string; mealType: MealType; recipeSlug: string }) =>
      createMealPlanEntry(date, mealType, recipeSlug),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mealplanKey });
      setAddingSlot(null);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => deleteMealPlanEntry(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mealplanKey }),
  });

  const generateAiMutation = useMutation({
    mutationFn: () => generateAiMealPlan(isoStart, isoEnd),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mealplanKey }),
  });

  function renderSlot(date: string, mealType: MealType, dotColor: string) {
    const entry = findEntry(entries, date, mealType);
    const isAdding = addingSlot?.date === date && addingSlot?.mealType === mealType;

    if (entry) {
      return (
        <div className={styles.filledSlot}>
          <div className={styles.filledSlotHeader}>
            <span className={styles.dot} style={{ background: dotColor }} />
            <button
              aria-label="Retirer du planning"
              className={styles.removeButton}
              onClick={() => deleteMutation.mutate(entry.id)}
            >
              ×
            </button>
          </div>
          {entry.recipe_slug ? (
            <Link to={`/recette/${entry.recipe_slug}`} className={styles.slotName}>
              {entry.name}
            </Link>
          ) : (
            <span className={styles.slotName}>{entry.name}</span>
          )}
        </div>
      );
    }

    if (isAdding) {
      return (
        <RecipeSearchPicker
          recipes={(sortedRecipes ?? []).map((recipe) => ({
            slug: recipe.slug,
            name: recipe.name,
            seasonal: seasonalSlugs.has(recipe.slug),
          }))}
          onSelect={(recipeSlug) => createMutation.mutate({ date, mealType, recipeSlug })}
          onCancel={() => setAddingSlot(null)}
        />
      );
    }

    return (
      <button className={styles.emptySlot} onClick={() => setAddingSlot({ date, mealType })}>
        + Ajouter
      </button>
    );
  }

  return (
    <>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>Planificateur</h1>
          <p className={styles.subtitle}>Organisez déjeuners et dîners de la semaine.</p>
          {currentSeason && (
            <p className={styles.seasonHint}>
              🌱 De saison ce mois-ci : {currentSeason.produce.join(", ")}
            </p>
          )}
        </div>
        <div className={styles.weekNav}>
          <button
            aria-label="Semaine précédente"
            className={styles.navButton}
            onClick={() => setWeekStart((current) => addDays(current, -7))}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#4A4238" strokeWidth={2.2}>
              <path d="m15 18-6-6 6-6" />
            </svg>
          </button>
          <span className={styles.weekLabel}>{formatWeekRange(weekStart)}</span>
          <button
            aria-label="Semaine suivante"
            className={styles.navButton}
            onClick={() => setWeekStart((current) => addDays(current, 7))}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#4A4238" strokeWidth={2.2}>
              <path d="m9 18 6-6-6-6" />
            </svg>
          </button>
        </div>
      </div>

      <div className={styles.grid}>
        <div />
        {days.map((d) => (
          <div key={d.iso} className={styles.dayLabel}>
            {d.label}
          </div>
        ))}

        <div className={styles.rowLabel}>Déjeuner</div>
        {days.map((d) => <div key={`lunch-${d.iso}`}>{renderSlot(d.iso, "lunch", "#C1592F")}</div>)}

        <div className={styles.rowLabel}>Dîner</div>
        {days.map((d) => <div key={`dinner-${d.iso}`}>{renderSlot(d.iso, "dinner", "#6B7A5E")}</div>)}
      </div>

      {generateAiMutation.isError && (
        <p className={styles.errorText}>
          Échec de la génération IA. Vérifiez que le serveur Ollama est joignable.
        </p>
      )}

      <div className={styles.footer}>
        <button
          className={styles.generateAiButton}
          disabled={generateAiMutation.isPending}
          onClick={() => generateAiMutation.mutate()}
        >
          {generateAiMutation.isPending ? "Génération en cours…" : "✨ Générer avec l'IA"}
        </button>
        <Link to={`/courses?start=${isoStart}&end=${isoEnd}`} className={styles.generateButton}>
          Générer la liste de courses
        </Link>
      </div>
    </>
  );
}
