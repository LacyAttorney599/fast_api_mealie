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
