import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useEditLock } from "../useEditLock";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("../../lib/tauri", () => ({ invoke: invokeMock }));
vi.mock("react-hot-toast", () => ({ default: { error: vi.fn() } }));

describe("useEditLock", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("bloque l'édition pendant l'acquisition et après une erreur", async () => {
    let rejectAcquire!: (error: Error) => void;
    invokeMock.mockReturnValueOnce(new Promise((_, reject) => { rejectAcquire = reject; }));

    const { result } = renderHook(() => useEditLock("personnes", 1, true));
    expect(result.current.lockBlocked).toBe(true);

    await act(async () => rejectAcquire(new Error("offline")));
    await waitFor(() => expect(result.current.lockLoading).toBe(false));
    expect(result.current.lockBlocked).toBe(true);
  });

  it("arrête les renouvellements après leur premier échec", async () => {
    vi.useFakeTimers();
    invokeMock
      .mockResolvedValueOnce({ acquired: true, resource_type: "personnes", resource_id: 1 })
      .mockRejectedValueOnce(new Error("offline"));

    const { result } = renderHook(() => useEditLock("personnes", 1, true));
    await act(async () => Promise.resolve());
    expect(result.current.lockBlocked).toBe(false);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(4 * 60 * 1000);
    });
    expect(result.current.lockBlocked).toBe(true);
    expect(invokeMock.mock.calls.filter((call) => call[0] === "acquire_edit_lock")).toHaveLength(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(8 * 60 * 1000);
    });
    expect(invokeMock.mock.calls.filter((call) => call[0] === "acquire_edit_lock")).toHaveLength(2);
  });
});
