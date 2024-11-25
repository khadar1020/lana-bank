mod activate;
mod config;
mod disbursal;
mod entity;
pub mod error;
mod history;
mod interest_accrual;
mod jobs;
mod processes;
mod publisher;
mod repo;

use std::collections::HashMap;

use authz::PermissionCheck;

use crate::{
    audit::{AuditInfo, AuditSvc},
    authorization::{Authorization, CreditFacilityAction, Object},
    customer::Customers,
    data_export::Export,
    governance::Governance,
    job::{error::JobError, *},
    ledger::{credit_facility::*, Ledger},
    outbox::Outbox,
    price::Price,
    primitives::{
        CreditFacilityId, CreditFacilityStatus, CustomerId, DisbursalId, PriceOfOneBTC, Satoshis,
        Subject, UsdCents,
    },
    terms::{CollateralizationState, TermValues},
};

pub use config::*;
pub use disbursal::{disbursal_cursor::*, *};
pub use entity::*;
use error::*;
pub use history::*;
pub use interest_accrual::*;
use jobs::*;
pub use processes::approve_credit_facility::*;
pub use processes::approve_disbursal::*;
use publisher::CreditFacilityPublisher;
use repo::CreditFacilityRepo;
pub use repo::{
    credit_facility_cursor::*, CreditFacilitiesSortBy, FindManyCreditFacilities, ListDirection,
    Sort,
};
use tracing::instrument;

#[derive(Clone)]
pub struct CreditFacilities {
    authz: Authorization,
    customers: Customers,
    credit_facility_repo: CreditFacilityRepo,
    disbursal_repo: DisbursalRepo,
    governance: Governance,
    jobs: Jobs,
    ledger: Ledger,
    price: Price,
    config: CreditFacilityConfig,
    approve_disbursal: ApproveDisbursal,
    approve_credit_facility: ApproveCreditFacility,
}

impl CreditFacilities {
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        pool: &sqlx::PgPool,
        config: CreditFacilityConfig,
        governance: &Governance,
        jobs: &Jobs,
        export: &Export,
        authz: &Authorization,
        customers: &Customers,
        ledger: &Ledger,
        price: &Price,
        outbox: &Outbox,
    ) -> Result<Self, CreditFacilityError> {
        let publisher = CreditFacilityPublisher::new(export, outbox);
        let credit_facility_repo = CreditFacilityRepo::new(pool, &publisher);
        let disbursal_repo = DisbursalRepo::new(pool, export);
        let approve_disbursal = ApproveDisbursal::new(
            &disbursal_repo,
            &credit_facility_repo,
            authz.audit(),
            governance,
            ledger,
        );
        let approve_credit_facility = ApproveCreditFacility::new(
            &credit_facility_repo,
            ledger,
            price,
            jobs,
            authz.audit(),
            governance,
        );
        jobs.add_initializer_and_spawn_unique(
            cvl::CreditFacilityProcessingJobInitializer::new(
                credit_facility_repo.clone(),
                price,
                authz.audit(),
            ),
            cvl::CreditFacilityJobConfig {
                job_interval: std::time::Duration::from_secs(30),
                upgrade_buffer_cvl_pct: config.upgrade_buffer_cvl_pct,
            },
        )
        .await?;
        jobs.add_initializer(interest::CreditFacilityProcessingJobInitializer::new(
            ledger,
            credit_facility_repo.clone(),
            authz.audit(),
        ));
        jobs.add_initializer_and_spawn_unique(
            CreditFacilityApprovalJobInitializer::new(outbox, &approve_credit_facility),
            CreditFacilityApprovalJobConfig,
        )
        .await?;
        jobs.add_initializer_and_spawn_unique(
            DisbursalApprovalJobInitializer::new(outbox, &approve_disbursal),
            DisbursalApprovalJobConfig,
        )
        .await?;
        let _ = governance
            .init_policy(APPROVE_CREDIT_FACILITY_PROCESS)
            .await;
        let _ = governance.init_policy(APPROVE_DISBURSAL_PROCESS).await;

        Ok(Self {
            authz: authz.clone(),
            customers: customers.clone(),
            credit_facility_repo,
            disbursal_repo,
            governance: governance.clone(),
            jobs: jobs.clone(),
            ledger: ledger.clone(),
            price: price.clone(),
            config,
            approve_disbursal,
            approve_credit_facility,
        })
    }

    pub async fn subject_can_create(
        &self,
        sub: &Subject,
        enforce: bool,
    ) -> Result<Option<AuditInfo>, CreditFacilityError> {
        Ok(self
            .authz
            .evaluate_permission(
                sub,
                Object::CreditFacility,
                CreditFacilityAction::Create,
                enforce,
            )
            .await?)
    }

    #[instrument(name = "credit_facility.initiate", skip(self), err)]
    pub async fn initiate(
        &self,
        sub: &Subject,
        customer_id: impl Into<CustomerId> + std::fmt::Debug,
        facility: UsdCents,
        terms: TermValues,
    ) -> Result<CreditFacility, CreditFacilityError> {
        let customer_id = customer_id.into();

        let audit_info = self
            .subject_can_create(sub, true)
            .await?
            .expect("audit info missing");

        let customer = match self.customers.find_by_id(sub, customer_id).await? {
            Some(customer) => customer,
            None => return Err(CreditFacilityError::CustomerNotFound(customer_id)),
        };

        let id = CreditFacilityId::new();
        let new_credit_facility = NewCreditFacility::builder()
            .id(id)
            .approval_process_id(id)
            .customer_id(customer_id)
            .terms(terms)
            .facility(facility)
            .account_ids(CreditFacilityAccountIds::new())
            .customer_account_ids(customer.account_ids)
            .audit_info(audit_info)
            .build()
            .expect("could not build new credit facility");

        let mut db = self.credit_facility_repo.begin_op().await?;
        self.governance
            .start_process(&mut db, id, id.to_string(), APPROVE_CREDIT_FACILITY_PROCESS)
            .await?;
        let credit_facility = self
            .credit_facility_repo
            .create_in_op(&mut db, new_credit_facility)
            .await?;
        self.ledger
            .create_accounts_for_credit_facility(credit_facility.id, credit_facility.account_ids)
            .await?;

        db.commit().await?;

        Ok(credit_facility)
    }

    #[instrument(name = "credit_facility.find", skip(self), err)]
    pub async fn find_by_id(
        &self,
        sub: &Subject,
        id: impl Into<CreditFacilityId> + std::fmt::Debug,
    ) -> Result<Option<CreditFacility>, CreditFacilityError> {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::Read)
            .await?;

        match self.credit_facility_repo.find_by_id(id.into()).await {
            Ok(credit_facility) => Ok(Some(credit_facility)),
            Err(e) if e.was_not_found() => Ok(None),
            Err(e) => Err(e),
        }
    }

    #[instrument(name = "credit_facility.balance", skip(self), err)]
    pub async fn balance(
        &self,
        sub: &Subject,
        id: impl Into<CreditFacilityId> + std::fmt::Debug,
    ) -> Result<CreditFacilityBalance, CreditFacilityError> {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::Read)
            .await?;

        let credit_facility = self.credit_facility_repo.find_by_id(id.into()).await?;

        Ok(credit_facility.balances())
    }

    pub async fn subject_can_initiate_disbursal(
        &self,
        sub: &Subject,
        enforce: bool,
    ) -> Result<Option<AuditInfo>, CreditFacilityError> {
        Ok(self
            .authz
            .evaluate_permission(
                sub,
                Object::CreditFacility,
                CreditFacilityAction::InitiateDisbursal,
                enforce,
            )
            .await?)
    }

    #[instrument(name = "credit_facility.initiate_disbursal", skip(self), err)]
    #[es_entity::retry_on_concurrent_modification]
    pub async fn initiate_disbursal(
        &self,
        sub: &Subject,
        credit_facility_id: CreditFacilityId,
        amount: UsdCents,
    ) -> Result<Disbursal, CreditFacilityError> {
        let audit_info = self
            .subject_can_initiate_disbursal(sub, true)
            .await?
            .expect("audit info missing");

        let mut credit_facility = self
            .credit_facility_repo
            .find_by_id(credit_facility_id)
            .await?;
        let balances = self
            .ledger
            .get_credit_facility_balance(credit_facility.account_ids)
            .await?;
        balances.check_disbursal_amount(amount)?;

        let mut db = self.credit_facility_repo.begin_op().await?;
        let now = crate::time::now();
        let new_disbursal = credit_facility.initiate_disbursal(amount, now, audit_info)?;
        self.governance
            .start_process(
                &mut db,
                new_disbursal.approval_process_id,
                new_disbursal.approval_process_id.to_string(),
                APPROVE_DISBURSAL_PROCESS,
            )
            .await?;
        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;
        let disbursal = self
            .disbursal_repo
            .create_in_op(&mut db, new_disbursal)
            .await?;

        db.commit().await?;
        Ok(disbursal)
    }

    #[instrument(name = "credit_facility.find_disbursal_by_id", skip(self), err)]
    pub async fn find_disbursal_by_id(
        &self,
        sub: &Subject,
        id: impl Into<DisbursalId> + std::fmt::Debug,
    ) -> Result<Option<Disbursal>, CreditFacilityError> {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::Read)
            .await?;

        match self.disbursal_repo.find_by_id(id.into()).await {
            Ok(loan) => Ok(Some(loan)),
            Err(e) if e.was_not_found() => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn ensure_up_to_date_disbursal_status(
        &self,
        disbursal: &Disbursal,
    ) -> Result<Option<Disbursal>, CreditFacilityError> {
        self.approve_disbursal.execute_from_svc(disbursal).await
    }

    pub async fn ensure_up_to_date_status(
        &self,
        credit_facility: &CreditFacility,
    ) -> Result<Option<CreditFacility>, CreditFacilityError> {
        self.approve_credit_facility
            .execute_from_svc(credit_facility)
            .await
    }

    pub async fn subject_can_update_collateral(
        &self,
        sub: &Subject,
        enforce: bool,
    ) -> Result<Option<AuditInfo>, CreditFacilityError> {
        Ok(self
            .authz
            .evaluate_permission(
                sub,
                Object::CreditFacility,
                CreditFacilityAction::UpdateCollateral,
                enforce,
            )
            .await?)
    }

    #[es_entity::retry_on_concurrent_modification]
    #[instrument(name = "credit_facility.update_collateral", skip(self), err)]
    pub async fn update_collateral(
        &self,
        sub: &Subject,
        credit_facility_id: CreditFacilityId,
        updated_collateral: Satoshis,
    ) -> Result<CreditFacility, CreditFacilityError> {
        let audit_info = self
            .subject_can_update_collateral(sub, true)
            .await?
            .expect("audit info missing");

        let price = self.price.usd_cents_per_btc().await?;

        let mut credit_facility = self
            .credit_facility_repo
            .find_by_id(credit_facility_id)
            .await?;

        let credit_facility_collateral_update =
            credit_facility.initiate_collateral_update(updated_collateral)?;

        let mut db = self.credit_facility_repo.begin_op().await?;
        let executed_at = self
            .ledger
            .update_credit_facility_collateral(credit_facility_collateral_update.clone())
            .await?;

        credit_facility.confirm_collateral_update(
            credit_facility_collateral_update,
            executed_at,
            audit_info,
            price,
            self.config.upgrade_buffer_cvl_pct,
        );

        activate::execute(
            &mut credit_facility,
            &mut db,
            &self.ledger,
            self.authz.audit(),
            &self.jobs,
            price,
        )
        .await?;
        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;
        db.commit().await?;
        Ok(credit_facility)
    }

    pub async fn subject_can_record_payment(
        &self,
        sub: &Subject,
        enforce: bool,
    ) -> Result<Option<AuditInfo>, CreditFacilityError> {
        Ok(self
            .authz
            .evaluate_permission(
                sub,
                Object::CreditFacility,
                CreditFacilityAction::RecordPayment,
                enforce,
            )
            .await?)
    }

    #[instrument(name = "credit_facility.record_payment", skip(self), err)]
    pub async fn record_payment(
        &self,
        sub: &Subject,
        credit_facility_id: CreditFacilityId,
        amount: UsdCents,
    ) -> Result<CreditFacility, CreditFacilityError> {
        let mut db = self.credit_facility_repo.begin_op().await?;

        let audit_info = self
            .subject_can_record_payment(sub, true)
            .await?
            .expect("audit info missing");

        let price = self.price.usd_cents_per_btc().await?;

        let mut credit_facility = self
            .credit_facility_repo
            .find_by_id(credit_facility_id)
            .await?;

        let facility_balances = self
            .ledger
            .get_credit_facility_balance(credit_facility.account_ids)
            .await?
            .into();

        if credit_facility.outstanding() != facility_balances {
            return Err(CreditFacilityError::ReceivableBalanceMismatch);
        }

        let customer = self
            .customers
            .repo()
            .find_by_id(credit_facility.customer_id)
            .await?;
        self.ledger
            .get_customer_balance(customer.account_ids)
            .await?
            .check_withdraw_amount(amount)?;

        let repayment = credit_facility.initiate_repayment(amount)?;
        let executed_at = self
            .ledger
            .record_credit_facility_repayment(repayment.clone())
            .await?;
        credit_facility.confirm_repayment(
            repayment,
            executed_at,
            audit_info,
            price,
            self.config.upgrade_buffer_cvl_pct,
        );
        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;

        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;

        db.commit().await?;

        Ok(credit_facility)
    }

    // #[instrument(name = "credit_facility.list", skip(self), err)]
    pub async fn list(
        &self,
        sub: &Subject,
        query: es_entity::PaginatedQueryArgs<CreditFacilitiesCursor>,
        filter: FindManyCreditFacilities,
        sort: impl Into<Sort<CreditFacilitiesSortBy>>,
    ) -> Result<
        es_entity::PaginatedQueryRet<CreditFacility, CreditFacilitiesCursor>,
        CreditFacilityError,
    > {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::List)
            .await?;
        self.credit_facility_repo
            .find_many(filter, sort.into(), query)
            .await
    }

    #[instrument(
        name = "credit_facility.list_by_created_at_for_status",
        skip(self),
        err
    )]
    pub async fn list_by_created_at_for_status(
        &self,
        sub: &Subject,
        status: CreditFacilityStatus,
        query: es_entity::PaginatedQueryArgs<CreditFacilitiesByCreatedAtCursor>,
        direction: impl Into<es_entity::ListDirection> + std::fmt::Debug,
    ) -> Result<
        es_entity::PaginatedQueryRet<CreditFacility, CreditFacilitiesByCreatedAtCursor>,
        CreditFacilityError,
    > {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::List)
            .await?;
        self.credit_facility_repo
            .list_for_status_by_created_at(status, query, direction.into())
            .await
    }

    #[instrument(
        name = "credit_facility.list_by_created_at_for_collateralization_state",
        skip(self),
        err
    )]
    pub async fn list_by_created_at_for_collateralization_state(
        &self,
        sub: &Subject,
        collateralization_state: CollateralizationState,
        query: es_entity::PaginatedQueryArgs<CreditFacilitiesByCreatedAtCursor>,
        direction: impl Into<es_entity::ListDirection> + std::fmt::Debug,
    ) -> Result<
        es_entity::PaginatedQueryRet<CreditFacility, CreditFacilitiesByCreatedAtCursor>,
        CreditFacilityError,
    > {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::List)
            .await?;
        self.credit_facility_repo
            .list_for_collateralization_state_by_created_at(
                collateralization_state,
                query,
                direction.into(),
            )
            .await
    }

    #[instrument(
        name = "credit_facility.list_by_collateralization_ratio",
        skip(self),
        err
    )]
    pub async fn list_by_collateralization_ratio(
        &self,
        sub: &Subject,
        query: es_entity::PaginatedQueryArgs<CreditFacilitiesByCollateralizationRatioCursor>,
        direction: impl Into<es_entity::ListDirection> + std::fmt::Debug,
    ) -> Result<
        es_entity::PaginatedQueryRet<
            CreditFacility,
            CreditFacilitiesByCollateralizationRatioCursor,
        >,
        CreditFacilityError,
    > {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::List)
            .await?;
        self.credit_facility_repo
            .list_by_collateralization_ratio(query, direction.into())
            .await
    }

    #[instrument(
        name = "credit_facility.list_by_collateralization_ratio_for_status",
        skip(self),
        err
    )]
    pub async fn list_by_collateralization_ratio_for_status(
        &self,
        sub: &Subject,
        status: CreditFacilityStatus,
        query: es_entity::PaginatedQueryArgs<CreditFacilitiesByCollateralizationRatioCursor>,
        direction: impl Into<es_entity::ListDirection> + std::fmt::Debug,
    ) -> Result<
        es_entity::PaginatedQueryRet<
            CreditFacility,
            CreditFacilitiesByCollateralizationRatioCursor,
        >,
        CreditFacilityError,
    > {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::List)
            .await?;
        self.credit_facility_repo
            .list_for_status_by_collateralization_ratio(status, query, direction.into())
            .await
    }

    #[instrument(
        name = "credit_facility.list_by_collateralization_ratio_for_collateralization_state",
        skip(self),
        err
    )]
    pub async fn list_by_collateralization_ratio_for_collateralization_state(
        &self,
        sub: &Subject,
        collateralization_state: CollateralizationState,
        query: es_entity::PaginatedQueryArgs<CreditFacilitiesByCollateralizationRatioCursor>,
        direction: impl Into<es_entity::ListDirection> + std::fmt::Debug,
    ) -> Result<
        es_entity::PaginatedQueryRet<
            CreditFacility,
            CreditFacilitiesByCollateralizationRatioCursor,
        >,
        CreditFacilityError,
    > {
        self.authz
            .enforce_permission(sub, Object::CreditFacility, CreditFacilityAction::List)
            .await?;
        self.credit_facility_repo
            .list_for_collateralization_state_by_collateralization_ratio(
                collateralization_state,
                query,
                direction.into(),
            )
            .await
    }

    pub async fn subject_can_complete(
        &self,
        sub: &Subject,
        enforce: bool,
    ) -> Result<Option<AuditInfo>, CreditFacilityError> {
        Ok(self
            .authz
            .evaluate_permission(
                sub,
                Object::CreditFacility,
                CreditFacilityAction::Complete,
                enforce,
            )
            .await?)
    }

    #[instrument(name = "credit_facility.complete", skip(self), err)]
    pub async fn complete_facility(
        &self,
        sub: &Subject,
        credit_facility_id: impl Into<CreditFacilityId> + std::fmt::Debug,
    ) -> Result<CreditFacility, CreditFacilityError> {
        let credit_facility_id = credit_facility_id.into();

        let audit_info = self
            .subject_can_complete(sub, true)
            .await?
            .expect("audit info missing");

        let price = self.price.usd_cents_per_btc().await?;

        let mut credit_facility = self
            .credit_facility_repo
            .find_by_id(credit_facility_id)
            .await?;

        let completion = credit_facility.initiate_completion()?;

        let executed_at = self
            .ledger
            .complete_credit_facility(completion.clone())
            .await?;
        credit_facility.confirm_completion(
            completion,
            executed_at,
            audit_info,
            price,
            self.config.upgrade_buffer_cvl_pct,
        );

        let mut db = self.credit_facility_repo.begin_op().await?;
        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;
        db.commit().await?;

        Ok(credit_facility)
    }

    pub async fn list_disbursals(
        &self,
        sub: &Subject,
        query: es_entity::PaginatedQueryArgs<DisbursalsCursor>,
        filter: FindManyDisbursals,
        sort: impl Into<Sort<DisbursalsSortBy>>,
    ) -> Result<es_entity::PaginatedQueryRet<Disbursal, DisbursalsCursor>, CreditFacilityError>
    {
        self.authz
            .enforce_permission(
                sub,
                Object::CreditFacility,
                CreditFacilityAction::ListDisbursals,
            )
            .await?;

        let disbursals = self
            .disbursal_repo
            .find_many(filter, sort.into(), query)
            .await?;
        Ok(disbursals)
    }

    pub async fn find_all<T: From<CreditFacility>>(
        &self,
        ids: &[CreditFacilityId],
    ) -> Result<HashMap<CreditFacilityId, T>, CreditFacilityError> {
        self.credit_facility_repo.find_all(ids).await
    }

    pub async fn find_all_disbursals<T: From<Disbursal>>(
        &self,
        ids: &[DisbursalId],
    ) -> Result<HashMap<DisbursalId, T>, CreditFacilityError> {
        Ok(self.disbursal_repo.find_all(ids).await?)
    }
}