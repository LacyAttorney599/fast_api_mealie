import type { DraftIngredient, RecipeDraft } from "./api";

interface ParseResult {
  recipes: RecipeDraft[];
  error: string | null;
}

function parseIngredient(raw: unknown): DraftIngredient | null {
  if (!raw || typeof raw !== "object") return null;
  const item = raw as Record<string, unknown>;
  if (typeof item.aliment !== "string" || !item.aliment.trim()) return null;

  const rawQty = item.quantite;
  const quantite =
    rawQty !== undefined && rawQty !== null && rawQty !== "" && !Number.isNaN(Number(rawQty))
      ? Number(rawQty)
      : null;

  const rawUnit = item.unite;
  const unite = typeof rawUnit === "string" && rawUnit.trim() && rawUnit !== "—" ? rawUnit.trim() : null;

  return { quantite, unite, aliment: item.aliment.trim() };
}

export function parseRecipesJson(text: string): ParseResult {
  if (!text.trim()) {
    return { recipes: [], error: null };
  }

  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { recipes: [], error: "JSON invalide — vérifiez la syntaxe." };
  }

  if (!Array.isArray(data)) {
    return { recipes: [], error: "Le JSON doit être un tableau de recettes." };
  }

  const recipes: RecipeDraft[] = [];
  for (const raw of data) {
    if (!raw || typeof raw !== "object") continue;
    const item = raw as Record<string, unknown>;
    if (typeof item.nom !== "string" || !item.nom.trim()) continue;

    const ingredients = Array.isArray(item.ingredients)
      ? item.ingredients.map(parseIngredient).filter((ing): ing is DraftIngredient => ing !== null)
      : [];

    const etapes = Array.isArray(item.etapes)
      ? item.etapes.filter((step): step is string => typeof step === "string" && step.trim().length > 0)
      : [];

    recipes.push({ nom: item.nom.trim(), ingredients, etapes });
  }

  if (recipes.length === 0) {
    return { recipes: [], error: "Aucune recette valide trouvée dans ce JSON (chaque objet doit avoir un champ \"nom\")." };
  }

  return { recipes, error: null };
}
