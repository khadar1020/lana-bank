mod cursor;
mod entity;
pub mod error;
mod repo;

use crate::{entity::*, ledger::*, primitives::UserId};

pub use cursor::*;
pub use entity::*;
use error::UserError;
pub use repo::UserRepo;
use uuid::Uuid;

#[derive(Clone)]
pub struct Users {
    _pool: sqlx::PgPool,
    repo: UserRepo,
    ledger: Ledger,
}

impl Users {
    pub fn new(pool: &sqlx::PgPool, ledger: &Ledger) -> Self {
        let repo = UserRepo::new(pool);
        Self {
            _pool: pool.clone(),
            repo,
            ledger: ledger.clone(),
        }
    }

    pub fn repo(&self) -> &UserRepo {
        &self.repo
    }

    pub async fn create_user(&self, id: Uuid, email: String) -> Result<User, UserError> {
        let (ledger_account_ids, ledger_account_addresses) =
            self.ledger.create_accounts_for_user(&email).await?;
        let new_user = NewUser::builder()
            .id(id)
            .email(email)
            .account_ids(ledger_account_ids)
            .account_addresses(ledger_account_addresses)
            .build()
            .expect("Could not build User");

        let EntityUpdate { entity: user, .. } = self.repo.create(new_user).await?;
        Ok(user)
    }

    pub async fn find_by_id(&self, id: UserId) -> Result<Option<User>, UserError> {
        match self.repo.find_by_id(id).await {
            Ok(user) => Ok(Some(user)),
            Err(UserError::CouldNotFindById(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn list(
        &self,
        query: crate::query::PaginatedQueryArgs<UserByNameCursor>,
    ) -> Result<crate::query::PaginatedQueryRet<User, UserByNameCursor>, UserError> {
        self.repo.list(query).await
    }
}
