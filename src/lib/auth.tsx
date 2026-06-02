import { createContext, useContext } from "react";
import type { CurrentSession } from "../types";

export interface AuthContextValue {
  session: CurrentSession | null;
  refreshSession: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  can: (permission: string) => boolean;
  isAdmin: boolean;
}

export const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth() {
  const value = useContext(AuthContext);
  if (!value) {
    throw new Error("AuthContext indisponible");
  }
  return value;
}
