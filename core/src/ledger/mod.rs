mod cala;
mod config;
mod constants;
pub mod error;
mod tx_template;
mod unallocated_collateral;

use cala_types::primitives::TxTemplateId;
use uuid::Uuid;

use crate::primitives::LedgerAccountId;

use cala::*;
pub use config::*;
use error::*;
pub use unallocated_collateral::*;

#[derive(Clone)]
pub struct Ledger {
    pub cala: CalaClient,
}

impl Ledger {
    pub async fn init(config: LedgerConfig) -> Result<Self, LedgerError> {
        let cala = CalaClient::new(config.cala_url);
        Self::initialize_journal(&cala).await?;
        Self::initialize_global_accounts(&cala).await?;
        Self::initialize_tx_templates(&cala).await?;
        Ok(Ledger { cala })
    }

    pub async fn get_unallocated_collateral(
        &self,
        id: LedgerAccountId,
    ) -> Result<UnallocatedCollateral, LedgerError> {
        self.cala
            .find_account_by_id(id)
            .await?
            .ok_or(LedgerError::AccountNotFound)
    }

    pub async fn create_unallocated_collateral_account_for_user(
        &self,
        bitfinex_username: &str,
    ) -> Result<LedgerAccountId, LedgerError> {
        Self::assert_account_exists(
            &self.cala,
            &format!("USERS.UNALLOCATED_COLLATERAL.{}", bitfinex_username),
            &format!("USERS.UNALLOCATED_COLLATERAL.{}", bitfinex_username),
            &format!("lava:usr:bfx-{}", bitfinex_username),
        )
        .await
    }

    pub async fn create_accounts_for_loan(
        &self,
        id: impl Into<Uuid>,
    ) -> Result<LedgerAccountId, LedgerError> {
        let id = id.into();
        Self::assert_account_exists(
            &self.cala,
            &format!("lava:loan-{}", id),
            &format!("lava:loan-{}", id),
            &format!("lava:loan-{}", id),
        )
        .await
    }

    async fn initialize_journal(cala: &CalaClient) -> Result<(), LedgerError> {
        if cala
            .find_journal_by_id(constants::LAVA_JOURNAL_ID)
            .await
            .is_ok()
        {
            return Ok(());
        }

        let err = match cala.create_lava_journal(constants::LAVA_JOURNAL_ID).await {
            Ok(_) => return Ok(()),
            Err(e) => e,
        };

        cala.find_journal_by_id(constants::LAVA_JOURNAL_ID)
            .await
            .map_err(|_| err)?;
        Ok(())
    }

    async fn initialize_global_accounts(cala: &CalaClient) -> Result<(), LedgerError> {
        Self::assert_account_exists(
            cala,
            constants::LOAN_OMINBUS_EXTERNAL_ID,
            constants::LOAN_OMINBUS_EXTERNAL_ID,
            constants::LOAN_OMINBUS_EXTERNAL_ID,
        )
        .await?;
        Ok(())
    }

    async fn assert_account_exists(
        cala: &CalaClient,
        name: &str,
        code: &str,
        external_id: &str,
    ) -> Result<LedgerAccountId, LedgerError> {
        if let Ok(Some(id)) = cala
            .find_account_by_external_id(external_id.to_owned())
            .await
        {
            return Ok(id);
        }

        let err = match cala
            .create_account(name.to_owned(), code.to_owned(), external_id.to_owned())
            .await
        {
            Ok(id) => return Ok(id),
            Err(e) => e,
        };

        cala.find_account_by_external_id(external_id.to_owned())
            .await
            .map_err(|_| err)?
            .ok_or_else(|| LedgerError::CouldNotAssertAccountExits)
    }

    async fn assert_topup_unallocated_collateral_tx_template_exists(
        cala: &CalaClient,
        template_code: &str,
    ) -> Result<TxTemplateId, LedgerError> {
        if let Ok(id) = cala
            .find_tx_template_by_code::<TxTemplateId>(template_code.to_owned())
            .await
        {
            return Ok(id);
        }

        let template_id = TxTemplateId::new();
        let err = match cala
            .create_topup_unallocated_collateral_tx_template(template_id)
            .await
        {
            Ok(id) => {
                return Ok(id);
            }
            Err(e) => e,
        };

        Ok(cala
            .find_tx_template_by_code::<TxTemplateId>(template_code.to_owned())
            .await
            .map_err(|_| err)?)
    }

    async fn initialize_tx_templates(cala: &CalaClient) -> Result<(), LedgerError> {
        Self::assert_topup_unallocated_collateral_tx_template_exists(
            cala,
            constants::TOPUP_UNALLOCATED_COLLATERAL_CODE,
        )
        .await?;
        Ok(())
    }
}
