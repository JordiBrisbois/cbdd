import { useCallback } from "react";
import type { Categorie, Personne, Structure, Fonction } from "../types";
import { invoke } from "../lib/tauri";
import { useAuth } from "../lib/auth";
import { useAsyncData } from "./useAsyncData";

export interface ReferentialResult<T> { data: T; loading: boolean; error: string | undefined; reload: () => Promise<T>; }

export function useReferentials() {
  const { can } = useAuth();
  const canCategories = can("categories.read");
  const canPersonnes = can("personnes.read");
  const canStructures = can("structures.read");
  const canFonctions = can("fonctions.read") || can("affiliations.read");

  const loadCategories = useCallback(() => {
    if (!canCategories) return Promise.resolve([]);
    return invoke<Categorie[]>("lister_categories");
  }, [canCategories]);
  const cats = useAsyncData(loadCategories, [], {
    errorMessage: canCategories ? "Impossible de charger les catégories" : undefined,
  });

  const loadPersonnes = useCallback(() => {
    if (!canPersonnes) return Promise.resolve([]);
    return invoke<Personne[]>("lister_personnes");
  }, [canPersonnes]);
  const pers = useAsyncData(loadPersonnes, [], {
    errorMessage: canPersonnes ? "Impossible de charger les personnes" : undefined,
  });

  const loadStructures = useCallback(() => {
    if (!canStructures) return Promise.resolve([]);
    return invoke<Structure[]>("lister_structures");
  }, [canStructures]);
  const structs = useAsyncData(loadStructures, [], {
    errorMessage: canStructures ? "Impossible de charger les structures" : undefined,
  });

  const loadFonctions = useCallback(() => {
    if (!canFonctions) return Promise.resolve([]);
    return invoke<Fonction[]>("lister_fonctions");
  }, [canFonctions]);
  const foncs = useAsyncData(loadFonctions, [], {
    errorMessage: canFonctions ? "Impossible de charger les fonctions" : undefined,
  });

  return {
    categories: { data: cats.data, loading: cats.loading, error: undefined, reload: cats.reload } as ReferentialResult<Categorie[]>,
    personnes: { data: pers.data, loading: pers.loading, error: undefined, reload: pers.reload } as ReferentialResult<Personne[]>,
    structures: { data: structs.data, loading: structs.loading, error: undefined, reload: structs.reload } as ReferentialResult<Structure[]>,
    fonctions: { data: foncs.data, loading: foncs.loading, error: undefined, reload: foncs.reload } as ReferentialResult<Fonction[]>,
    reloadAll: () => { cats.reload().catch(() => {}); pers.reload().catch(() => {}); structs.reload().catch(() => {}); foncs.reload().catch(() => {}); },
  };
}
