use super::*;

#[tauri::command]
pub fn get_current_session(app: AppHandle) -> Result<CurrentSession, String> {
    let conn = db::get_conn(&app)?;
    auth::get_current_session(&conn)
}

#[tauri::command]
pub fn login(app: AppHandle, credentials: LoginInput) -> Result<CurrentSession, String> {
    let conn = db::get_conn(&app)?;
    auth::login(&conn, credentials.username.trim(), &credentials.password)
}

#[tauri::command]
pub fn logout() -> Result<(), String> {
    auth::logout()
}

#[tauri::command]
pub fn lister_permissions(app: AppHandle) -> Result<Vec<Permission>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    Ok(auth::list_permissions())
}

#[tauri::command]
pub fn lister_roles(app: AppHandle) -> Result<Vec<RoleDetails>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    auth::list_roles(&conn)
}

#[tauri::command]
pub fn sauvegarder_role(app: AppHandle, role: RoleInput) -> Result<RoleDetails, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    auth::save_role(&conn, role)
}

#[tauri::command]
pub fn supprimer_role(app: AppHandle, role_id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.roles")?;
    auth::delete_role(&conn, role_id)
}

#[tauri::command]
pub fn lister_users(app: AppHandle) -> Result<Vec<UserSummary>, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::list_users(&conn)
}

#[tauri::command]
pub fn sauvegarder_user(app: AppHandle, user: UserInput) -> Result<UserSummary, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::save_user(&conn, user)
}

#[tauri::command]
pub fn supprimer_user(app: AppHandle, user_id: i64) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::delete_user(&conn, user_id)
}

#[tauri::command]
pub fn changer_mot_de_passe_user(
    app: AppHandle,
    payload: PasswordChangeInput,
) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.users")?;
    auth::admin_set_password(&conn, payload.user_id, &payload.new_password)
}

#[tauri::command]
pub fn changer_mon_mot_de_passe(
    app: AppHandle,
    payload: OwnPasswordChangeInput,
) -> Result<(), String> {
    let conn = db::get_conn(&app)?;
    auth::change_own_password(&conn, &payload.current_password, &payload.new_password)
}

#[tauri::command]
pub fn get_security_settings(app: AppHandle) -> Result<SecuritySettings, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.settings")?;
    auth::get_security_settings(&conn)
}

#[tauri::command]
pub fn sauvegarder_security_settings(
    app: AppHandle,
    settings: SecuritySettings,
) -> Result<SecuritySettings, String> {
    let conn = db::get_conn(&app)?;
    auth::require_permission(&conn, "admin.settings")?;
    auth::save_security_settings(&conn, settings)
}
