import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

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
  ContactModal: () => <div>Contact chargé</div>,
}));

describe("CategoriesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("appelle get_personne avant d'ouvrir la modale contact", async () => {
    const user = userEvent.setup();
    invokeMock.mockImplementation((command: string) => {
      if (command === "lister_categories") {
        return Promise.resolve([{ id_categorie: 1, nom_categorie: "Test" }]);
      }
      if (command === "lister_personnes_categorie_detaillee") {
        return Promise.resolve([
          { id_personne: 1, nom: "Dupont", prenom: "Jean", type_entree: "personne" },
        ]);
      }
      if (command === "get_personne") {
        return Promise.resolve({
          id_personne: 1,
          nom: "Dupont",
          prenom: "Jean",
          updated_at: "2026-06-09 12:00:00",
        });
      }
      return Promise.resolve([]);
    });

    const { CategoriesPage } = await import("../CategoriesPage");
    render(<CategoriesPage />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("lister_categories");
    });

    expect(invokeMock.mock.calls.filter((call) => call[0] === "get_personne")).toHaveLength(0);

    await user.selectOptions(screen.getByRole("combobox"), "1");
    await user.click(await screen.findByText("Dupont"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("get_personne", { id: 1 });
      expect(screen.getByText("Contact chargé")).toBeInTheDocument();
    });
  });
});
