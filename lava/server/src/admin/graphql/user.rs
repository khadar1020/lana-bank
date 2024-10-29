use async_graphql::*;

use crate::{admin::AdminAuthContext, shared_graphql::primitives::UUID};
use lava_app::{app::LavaApp, authorization::VisibleNavigationItems, primitives::Role};

#[derive(InputObject)]
pub struct UserCreateInput {
    pub email: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(complex)]
pub struct User {
    user_id: UUID,
    email: String,
    roles: Vec<Role>,
}

#[ComplexObject]
impl User {
    async fn visible_navigation_items(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<VisibleNavigationItems> {
        let app = ctx.data_unchecked::<LavaApp>();
        let AdminAuthContext { sub } = ctx.data()?;
        let permissions = app.get_visible_nav_items(sub).await?;
        Ok(permissions)
    }

    async fn can_create_customer(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let app = ctx.data_unchecked::<LavaApp>();
        let AdminAuthContext { sub } = ctx.data()?;
        Ok(app
            .customers()
            .user_can_create_customer(sub, false)
            .await
            .is_ok())
    }

    async fn can_create_user(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let app = ctx.data_unchecked::<LavaApp>();
        let AdminAuthContext { sub } = ctx.data()?;
        Ok(app.users().can_create_user(sub, false).await.is_ok())
    }

    async fn can_assign_role_to_user(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let app = ctx.data_unchecked::<LavaApp>();
        let AdminAuthContext { sub } = ctx.data()?;
        Ok(app
            .users()
            .can_assign_role_to_user(sub, None, false)
            .await
            .is_ok())
    }

    async fn can_revoke_role_from_user(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let app = ctx.data_unchecked::<LavaApp>();
        let AdminAuthContext { sub } = ctx.data()?;
        Ok(app
            .users()
            .can_revoke_role_from_user(sub, None, false)
            .await
            .is_ok())
    }

    async fn can_create_terms_template(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let app = ctx.data_unchecked::<LavaApp>();
        let AdminAuthContext { sub } = ctx.data()?;
        Ok(app
            .terms_templates()
            .user_can_create_terms_template(sub, false)
            .await
            .is_ok())
    }

    async fn can_update_terms_template(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let app = ctx.data_unchecked::<LavaApp>();
        let AdminAuthContext { sub } = ctx.data()?;
        Ok(app
            .terms_templates()
            .user_can_update_terms_template(sub, false)
            .await
            .is_ok())
    }
}

#[derive(SimpleObject)]
pub struct UserCreatePayload {
    user: User,
}

impl From<lava_app::user::User> for User {
    fn from(user: lava_app::user::User) -> Self {
        Self {
            user_id: UUID::from(user.id),
            roles: user.current_roles().into_iter().map(Role::from).collect(),
            email: user.email,
        }
    }
}

impl From<lava_app::user::User> for UserCreatePayload {
    fn from(user: lava_app::user::User) -> Self {
        Self {
            user: User::from(user),
        }
    }
}

#[derive(InputObject)]
pub struct UserAssignRoleInput {
    pub id: UUID,
    pub role: Role,
}

#[derive(SimpleObject)]
pub struct UserAssignRolePayload {
    user: User,
}

impl From<lava_app::user::User> for UserAssignRolePayload {
    fn from(user: lava_app::user::User) -> Self {
        Self {
            user: User::from(user),
        }
    }
}

#[derive(InputObject)]
pub struct UserRevokeRoleInput {
    pub id: UUID,
    pub role: Role,
}

#[derive(SimpleObject)]
pub struct UserRevokeRolePayload {
    user: User,
}

impl From<lava_app::user::User> for UserRevokeRolePayload {
    fn from(user: lava_app::user::User) -> Self {
        Self {
            user: User::from(user),
        }
    }
}