import type { RoleInput, UserInput } from "../../types";

export const DEFAULT_UPDATE_ENDPOINT = "https://github.com/JordiBrisbois/cbdd/releases/latest/download/latest.json";

export const EMPTY_USER: UserInput = {
  username: "",
  display_name: "",
  is_active: true,
  must_change_password: false,
  role_ids: [],
  password: "",
};

export const EMPTY_ROLE: RoleInput = {
  nom_role: "",
  permission_codes: [],
};
