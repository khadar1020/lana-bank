mod job;

use tracing::instrument;

use ::job::JobId;
use audit::AuditSvc;
use authz::PermissionCheck;
use core_price::Price;
use outbox::OutboxEventMarker;

use crate::{
    error::CoreCreditError, interest_accruals, ledger::CreditLedger, overdue,
    primitives::CreditFacilityId, CoreCreditAction, CoreCreditEvent, CoreCreditObject,
    CreditFacility, CreditFacilityRepo, DisbursalRepo, Jobs, LedgerTxId, ObligationRepo,
};

pub use job::*;

pub struct ActivateCreditFacility<Perms, E>
where
    Perms: PermissionCheck,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    obligation_repo: ObligationRepo,
    credit_facility_repo: CreditFacilityRepo<E>,
    disbursal_repo: DisbursalRepo,
    ledger: CreditLedger,
    price: Price,
    jobs: Jobs,
    audit: Perms::Audit,
}

impl<Perms, E> Clone for ActivateCreditFacility<Perms, E>
where
    Perms: PermissionCheck,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    fn clone(&self) -> Self {
        Self {
            obligation_repo: self.obligation_repo.clone(),
            credit_facility_repo: self.credit_facility_repo.clone(),
            disbursal_repo: self.disbursal_repo.clone(),
            ledger: self.ledger.clone(),
            price: self.price.clone(),
            jobs: self.jobs.clone(),
            audit: self.audit.clone(),
        }
    }
}
impl<Perms, E> ActivateCreditFacility<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action: From<CoreCreditAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object: From<CoreCreditObject>,
    E: OutboxEventMarker<CoreCreditEvent>,
{
    pub fn new(
        obligation_repo: &ObligationRepo,
        credit_facility_repo: &CreditFacilityRepo<E>,
        disbursal_repo: &DisbursalRepo,
        ledger: &CreditLedger,
        price: &Price,
        jobs: &Jobs,
        audit: &Perms::Audit,
    ) -> Self {
        Self {
            obligation_repo: obligation_repo.clone(),
            credit_facility_repo: credit_facility_repo.clone(),
            disbursal_repo: disbursal_repo.clone(),
            ledger: ledger.clone(),
            price: price.clone(),
            jobs: jobs.clone(),
            audit: audit.clone(),
        }
    }

    #[es_entity::retry_on_concurrent_modification(any_error = true)]
    #[instrument(name = "credit_facility.activation.execute", skip(self))]
    pub async fn execute(
        &self,
        id: impl es_entity::RetryableInto<CreditFacilityId>,
    ) -> Result<CreditFacility, CoreCreditError> {
        let id = id.into();
        let mut credit_facility = self.credit_facility_repo.find_by_id(id).await?;

        let mut db = self.credit_facility_repo.begin_op().await?;

        let audit_info = self
            .audit
            .record_system_entry_in_tx(
                db.tx(),
                CoreCreditObject::all_credit_facilities(),
                CoreCreditAction::CREDIT_FACILITY_ACTIVATE,
            )
            .await?;

        let price = self.price.usd_cents_per_btc().await?;
        let now = db.now();

        let Ok(es_entity::Idempotent::Executed((credit_facility_activation, next_accrual_period))) =
            credit_facility.activate(now, price, audit_info.clone())
        else {
            return Ok(credit_facility);
        };

        self.jobs
            .create_and_spawn_at_in_op(
                &mut db,
                JobId::new(),
                overdue::CreditFacilityJobConfig::<Perms, E> {
                    credit_facility_id: id,
                    _phantom: std::marker::PhantomData,
                },
                credit_facility.disbursed_overdue_at(),
            )
            .await?;

        let new_disbursal = credit_facility.initiate_disbursal(
            credit_facility.structuring_fee(),
            now,
            price,
            Some(credit_facility.approval_process_id),
            audit_info.clone(),
        )?;
        let mut disbursal = self
            .disbursal_repo
            .create_in_op(&mut db, new_disbursal)
            .await?;

        let tx_id = LedgerTxId::new();
        let new_obligation = disbursal
            .approval_process_concluded(tx_id, true, audit_info.clone())
            .expect("First instance of idempotent action ignored")
            .expect("First disbursal obligation was already created");
        credit_facility
            .disbursal_concluded(
                &disbursal,
                tx_id,
                Some(new_obligation.id()),
                now,
                audit_info.clone(),
            )
            .expect("First instance of idempotent action ignored");

        self.obligation_repo
            .create_in_op(&mut db, new_obligation)
            .await?;
        self.disbursal_repo
            .update_in_op(&mut db, &mut disbursal)
            .await?;
        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;

        let accrual_id = credit_facility
            .interest_accrual_cycle_in_progress()
            .expect("First accrual not found")
            .id;
        self.jobs
            .create_and_spawn_at_in_op(
                &mut db,
                accrual_id,
                interest_accruals::CreditFacilityJobConfig::<Perms, E> {
                    credit_facility_id: id,
                    _phantom: std::marker::PhantomData,
                },
                next_accrual_period.end,
            )
            .await?;

        self.ledger
            .activate_credit_facility(db, credit_facility_activation)
            .await?;

        Ok(credit_facility)
    }
}
