import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import {
  createMealPlanEntry,
  deleteMealPlanEntry,
  fetchMealPlan,
  fetchRecipes,
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
        <select
          autoFocus
          className={styles.slotSelect}
          defaultValue=""
          onBlur={() => setAddingSlot(null)}
          onChange={(e) => {
            if (e.target.value) {
              createMutation.mutate({ date, mealType, recipeSlug: e.target.value });
            }
          }}
        >
          <option value="" disabled>
            Choisir une recette…
          </option>
          {recipes?.map((recipe) => (
            <option key={recipe.slug} value={recipe.slug}>
              {recipe.name}
            </option>
          ))}
        </select>
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

      <div className={styles.footer}>
        <Link to={`/courses?start=${isoStart}&end=${isoEnd}`} className={styles.generateButton}>
          Générer la liste de courses
        </Link>
      </div>
    </>
  );
}
