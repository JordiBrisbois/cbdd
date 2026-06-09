import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useReferentials } from "../useReferentials";

const allowed = vi.hoisted(() => new Set<string>());
const invokeMock = vi.hoisted(() => vi.fn());

vi.mock("../../lib/auth", () => ({
  useAuth: () => ({ can: (permission: string) => allowed.has(permission) }),
}));
vi.mock("../../lib/tauri", () => ({ invoke: invokeMock }));
vi.mock("react-hot-toast", () => ({ default: { error: vi.fn() } }));

describe("useReferentials", () => {
  beforeEach(() => {
    allowed.clear();
    vi.clearAllMocks();
  });

  it("ne charge que les référentiels autorisés et conserve les succès indépendants", async () => {
    allowed.add("categories.read");
    allowed.add("structures.read");
    invokeMock.mockImplementation((command: string) => {
      if (command === "lister_categories") return Promise.reject(new Error("categories indisponibles"));
      if (command === "lister_structures") return Promise.resolve([{ id_structure: 1, nom_structure: "CRVI" }]);
      throw new Error(`appel inattendu: ${command}`);
    });

    const { result } = renderHook(() => useReferentials());

    await waitFor(() => expect(result.current.structures.loading).toBe(false));
    await waitFor(() => expect(result.current.categories.error).toContain("categories indisponibles"));

    expect(result.current.structures.data).toEqual([{ id_structure: 1, nom_structure: "CRVI" }]);
    expect(invokeMock.mock.calls.map((call) => call[0]).sort()).toEqual([
      "lister_categories",
      "lister_structures",
    ]);
  });

  it("respecte le chargement paresseux des personnes", async () => {
    allowed.add("personnes.read");
    invokeMock.mockResolvedValue([{ id_personne: 1 }]);

    const { result } = renderHook(() => useReferentials({ personnesImmediate: false }));
    expect(invokeMock).not.toHaveBeenCalledWith("lister_personnes");

    await result.current.personnes.reload();
    expect(invokeMock).toHaveBeenCalledWith("lister_personnes");
  });
});
