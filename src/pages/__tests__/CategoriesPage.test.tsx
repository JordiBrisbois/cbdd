import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor } from "@testing-library/react";

vi.mock("react-hot-toast", () => ({
  default: { error: vi.fn() },
}));

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("../../lib/tauri", () => ({
  invoke: invokeMock,
}));

vi.mock("../../lib/auth", () => ({
  useAuth: () => ({ can: () => true }),
}));

vi.mock("../../modals/ContactModal", () => ({
  ContactModal: () => null,
}));

describe("CategoriesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("appelle get_personne avant d'ouvrir la modale contact", async () => {
    invokeMock
      .mockResolvedValueOnce([{ id_categorie: 1, nom_categorie: "Test" }])
      .mockResolvedValue([
        { id_personne: 1, nom: "Dupont", prenom: "Jean", type_entree: "personne" },
      ]);

    const { CategoriesPage } = await import("../CategoriesPage");
    render(<CategoriesPage />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("lister_categories");
    });

    const getPersonneCalls = invokeMock.mock.calls.filter(
      (c: unknown[]) => c[0] === "get_personne",
    );
    expect(getPersonneCalls).toHaveLength(0);
  });
});
