import type { Schemas } from '../api/client'

export type NutritionTarget = Schemas['NutritionTarget']

/** Calories from grams, as the backend counts them: 4 per gram of protein and carbohydrates, 9 of fat. */
export function kcal(proteinG: number, fatG: number, carbsG: number): number {
  return 4 * proteinG + 9 * fatG + 4 * carbsG
}

/** `1650 ккал · Б 100 · Ж 50 · В 200`. */
export function nutritionSummary(target: NutritionTarget): string {
  return `${target.kcal} ккал · Б ${target.protein_g} · Ж ${target.fat_g} · В ${target.carbs_g}`
}

/** The three macros in the order they are always shown. */
export function macros(target: NutritionTarget): { label: string; grams: number; kcal: number }[] {
  return [
    { label: 'Білки', grams: target.protein_g, kcal: 4 * target.protein_g },
    { label: 'Жири', grams: target.fat_g, kcal: 9 * target.fat_g },
    { label: 'Вуглеводи', grams: target.carbs_g, kcal: 4 * target.carbs_g },
  ]
}
