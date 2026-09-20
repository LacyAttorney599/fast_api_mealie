export interface RecipeSummary {
  slug: string;
  name: string;
  time: string | null;
  tag: string | null;
  image_url: string | null;
  color: string;
}

export async function fetchRecipes(search = ""): Promise<RecipeSummary[]> {
  const params = search ? `?search=${encodeURIComponent(search)}` : "";
  const response = await fetch(`/api/recipes${params}`);
  if (!response.ok) {
    throw new Error(`Échec du chargement des recettes (${response.status})`);
  }
  return response.json();
}
