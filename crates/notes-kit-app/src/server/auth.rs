use leptos::prelude::*;
use notes_kit_core::models::UserInfo;

#[server(Login, "/api")]
pub async fn login(email: String, password: String) -> Result<(), ServerFnError> {
    use notes_kit_core::models::Credentials;
    use notes_kit_server::auth::AuthSession;

    let mut auth_session: AuthSession = leptos_axum::extract().await?;

    let user = auth_session
        .authenticate(Credentials::Password { email, password })
        .await
        .map_err(|e| ServerFnError::new(format!("{e:?}")))?
        .ok_or_else(|| ServerFnError::new("Invalid email or password"))?;

    auth_session
        .login(&user)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server(Logout, "/api")]
pub async fn logout() -> Result<(), ServerFnError> {
    use notes_kit_server::auth::AuthSession;

    let mut auth_session: AuthSession = leptos_axum::extract().await?;
    auth_session
        .logout()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    use notes_kit_server::auth::AuthSession;

    let auth_session: AuthSession = leptos_axum::extract().await?;
    Ok(auth_session.user.map(|u| UserInfo::from(u.0)))
}
