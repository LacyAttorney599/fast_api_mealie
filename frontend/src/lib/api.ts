export interface RecipeSummary {
  slug: string;
  name: string;
  time: string | null;
  tag: string | null;
  image_url: string | null;
  color: string;
}

export interface Ingredient {
  qty: string;
  food: string;
}

export interface RecipeDetail {
  slug: string;
  name: string;
  description: string;
  image_url: string | null;
  time: string | null;
  servings: string | null;
  tags: string[];
  ingredients: Ingredient[];
  steps: string[];
  in_season: boolean;
}

export async function fetchRecipes(search = "", cookbook = ""): Promise<RecipeSummary[]> {
  const params = new URLSearchParams();
  if (search) params.set("search", search);
  if (cookbook) params.set("cookbook", cookbook);
  const query = params.toString();
  const response = await fetch(`/api/recipes${query ? `?${query}` : ""}`);
  if (!response.ok) {
    throw new Error(`Échec du chargement des recettes (${response.status})`);
  }
  return response.json();
}

export interface Cookbook {
  slug: string;
  name: string;
  manual: boolean;
}

export async function fetchCookbooks(): Promise<Cookbook[]> {
  const response = await fetch("/api/cookbooks");
  if (!response.ok) {
    throw new Error(`Échec du chargement des livres de recettes (${response.status})`);
  }
  return response.json();
}

export interface RecipeCategory {
  id: string;
  name: string;
}

export async function fetchCookbookCategories(): Promise<RecipeCategory[]> {
  const response = await fetch("/api/cookbooks/categories");
  if (!response.ok) {
    throw new Error(`Échec du chargement des catégories (${response.status})`);
  }
  return response.json();
}

export async function createCookbook(name: string, categoryId?: string): Promise<Cookbook> {
  const response = await fetch("/api/cookbooks", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name, category_id: categoryId ?? null }),
  });
  if (!response.ok) {
    throw new Error(`Échec de la création du livre (${response.status})`);
  }
  return response.json();
}

export async function addRecipeToCookbook(cookbookSlug: string, recipeSlug: string): Promise<void> {
  const response = await fetch(`/api/cookbooks/${cookbookSlug}/recipes/${recipeSlug}`, { method: "POST" });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.detail ?? `Échec de l'ajout au livre (${response.status})`);
  }
}

export async function removeRecipeFromCookbook(cookbookSlug: string, recipeSlug: string): Promise<void> {
  const response = await fetch(`/api/cookbooks/${cookbookSlug}/recipes/${recipeSlug}`, { method: "DELETE" });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.detail ?? `Échec du retrait du livre (${response.status})`);
  }
}

export async function fetchRecipe(slug: string): Promise<RecipeDetail> {
  const response = await fetch(`/api/recipes/${slug}`);
  if (!response.ok) {
    throw new Error(`Échec du chargement de la recette (${response.status})`);
  }
  return response.json();
}

export type MealType = "lunch" | "dinner";

export interface MealPlanEntry {
  id: number;
  date: string;
  entry_type: MealType;
  name: string;
  recipe_slug: string | null;
}

export async function fetchMealPlan(start: string, end: string): Promise<MealPlanEntry[]> {
  const response = await fetch(`/api/mealplan?start=${start}&end=${end}`);
  if (!response.ok) {
    throw new Error(`Échec du chargement du planning (${response.status})`);
  }
  return response.json();
}

export async function createMealPlanEntry(
  date: string,
  entryType: MealType,
  recipeSlug: string,
): Promise<MealPlanEntry> {
  const response = await fetch("/api/mealplan", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ date, entry_type: entryType, recipe_slug: recipeSlug }),
  });
  if (!response.ok) {
    throw new Error(`Échec de l'ajout au planning (${response.status})`);
  }
  return response.json();
}

export async function deleteMealPlanEntry(id: number): Promise<void> {
  const response = await fetch(`/api/mealplan/${id}`, { method: "DELETE" });
  if (!response.ok) {
    throw new Error(`Échec de la suppression (${response.status})`);
  }
}

export interface ShoppingItem {
  id: string;
  quantity: string;
  food: string;
  checked: boolean;
  category: string;
}

export interface ShoppingCategory {
  name: string;
  items: ShoppingItem[];
}

export interface ShoppingList {
  categories: ShoppingCategory[];
  total_items: number;
  recipe_count: number;
}

export async function fetchShoppingList(): Promise<ShoppingList> {
  const response = await fetch("/api/shoppinglist");
  if (!response.ok) {
    throw new Error(`Échec du chargement de la liste de courses (${response.status})`);
  }
  return response.json();
}

export async function toggleShoppingItem(id: string, checked: boolean): Promise<void> {
  const response = await fetch(`/api/shoppinglist/items/${id}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ checked }),
  });
  if (!response.ok) {
    throw new Error(`Échec de la mise à jour (${response.status})`);
  }
}

export async function addShoppingItem(text: string): Promise<void> {
  const response = await fetch("/api/shoppinglist/items", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
  });
  if (!response.ok) {
    throw new Error(`Échec de l'ajout (${response.status})`);
  }
}

export async function clearCheckedShoppingItems(): Promise<void> {
  const response = await fetch("/api/shoppinglist/clear-checked", { method: "POST" });
  if (!response.ok) {
    throw new Error(`Échec de la suppression (${response.status})`);
  }
}

export async function clearAllShoppingItems(): Promise<void> {
  const response = await fetch("/api/shoppinglist/clear-all", { method: "POST" });
  if (!response.ok) {
    throw new Error(`Échec de la suppression (${response.status})`);
  }
}

export async function generateShoppingList(start: string, end: string): Promise<void> {
  const response = await fetch("/api/shoppinglist/generate", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ start, end }),
  });
  if (!response.ok) {
    throw new Error(`Échec de la génération (${response.status})`);
  }
}

export interface DraftIngredient {
  quantite: number | null;
  unite: string | null;
  aliment: string;
}

export interface RecipeDraft {
  nom: string;
  ingredients: DraftIngredient[];
  etapes: string[];
}

export async function createRecipe(draft: RecipeDraft): Promise<{ slug: string }> {
  const response = await fetch("/api/recipes", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(draft),
  });
  if (!response.ok) {
    throw new Error(`Échec de l'enregistrement (${response.status})`);
  }
  return response.json();
}

export async function fetchRecipeDraft(slug: string): Promise<RecipeDraft> {
  const response = await fetch(`/api/recipes/${slug}/edit`);
  if (!response.ok) {
    throw new Error(`Échec du chargement (${response.status})`);
  }
  return response.json();
}

export async function updateRecipe(slug: string, draft: RecipeDraft): Promise<{ slug: string }> {
  const response = await fetch(`/api/recipes/${slug}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(draft),
  });
  if (!response.ok) {
    throw new Error(`Échec de l'enregistrement (${response.status})`);
  }
  return response.json();
}

export async function importPhoto(file: File): Promise<RecipeDraft> {
  const formData = new FormData();
  formData.append("image", file);
  const response = await fetch("/api/import/photo", {
    method: "POST",
    body: formData,
  });
  if (!response.ok) {
    throw new Error(`Échec de l'import (${response.status})`);
  }
  return response.json();
}

export interface BulkImportResult {
  nom: string;
  success: boolean;
  slug: string | null;
  error: string | null;
}

export async function createRecipesBulk(drafts: RecipeDraft[]): Promise<BulkImportResult[]> {
  const response = await fetch("/api/recipes/bulk", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(drafts),
  });
  if (!response.ok) {
    throw new Error(`Échec de l'import (${response.status})`);
  }
  return response.json();
}

export interface SeasonalRecipe {
  slug: string;
  name: string;
  in_season: boolean;
}

export async function fetchSeasonalRecipes(): Promise<SeasonalRecipe[]> {
  const response = await fetch("/api/recipes/seasonal");
  if (!response.ok) {
    throw new Error(`Échec du chargement (${response.status})`);
  }
  return response.json();
}

export interface CurrentSeason {
  month: number;
  produce: string[];
}

export async function fetchCurrentSeason(): Promise<CurrentSeason> {
  const response = await fetch("/api/seasons/current");
  if (!response.ok) {
    throw new Error(`Échec du chargement (${response.status})`);
  }
  return response.json();
}

export async function generateAiMealPlan(start: string, end: string): Promise<MealPlanEntry[]> {
  const response = await fetch("/api/mealplan/generate-ai", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ start, end }),
  });
  if (!response.ok) {
    throw new Error(`Échec de la génération (${response.status})`);
  }
  return response.json();
}
