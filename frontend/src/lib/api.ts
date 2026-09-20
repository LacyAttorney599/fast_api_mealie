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
}

export async function fetchRecipes(search = ""): Promise<RecipeSummary[]> {
  const params = search ? `?search=${encodeURIComponent(search)}` : "";
  const response = await fetch(`/api/recipes${params}`);
  if (!response.ok) {
    throw new Error(`Échec du chargement des recettes (${response.status})`);
  }
  return response.json();
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
