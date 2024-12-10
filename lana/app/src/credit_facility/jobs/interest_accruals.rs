use async_trait::async_trait;
use credit_facility::CreditFacilityInterestAccrual;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    audit::*,
    authorization::{CreditFacilityAction, Object},
    credit_facility::{interest_incurrences, repo::*, CreditFacilityError},
    job::{error::*, *},
    ledger::*,
    primitives::CreditFacilityId,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct CreditFacilityJobConfig {
    pub credit_facility_id: CreditFacilityId,
}
impl JobConfig for CreditFacilityJobConfig {
    type Initializer = CreditFacilityProcessingJobInitializer;
}

pub struct CreditFacilityProcessingJobInitializer {
    ledger: Ledger,
    credit_facility_repo: CreditFacilityRepo,
    jobs: Jobs,
    audit: Audit,
}

impl CreditFacilityProcessingJobInitializer {
    pub fn new(
        ledger: &Ledger,
        credit_facility_repo: CreditFacilityRepo,
        jobs: &Jobs,
        audit: &Audit,
    ) -> Self {
        Self {
            ledger: ledger.clone(),
            credit_facility_repo,
            jobs: jobs.clone(),
            audit: audit.clone(),
        }
    }
}

const CREDIT_FACILITY_INTEREST_PROCESSING_JOB: JobType =
    JobType::new("credit-facility-interest-accrual-processing");
impl JobInitializer for CreditFacilityProcessingJobInitializer {
    fn job_type() -> JobType
    where
        Self: Sized,
    {
        CREDIT_FACILITY_INTEREST_PROCESSING_JOB
    }

    fn init(&self, job: &Job) -> Result<Box<dyn JobRunner>, Box<dyn std::error::Error>> {
        Ok(Box::new(CreditFacilityProcessingJobRunner {
            config: job.config()?,
            credit_facility_repo: self.credit_facility_repo.clone(),
            ledger: self.ledger.clone(),
            jobs: self.jobs.clone(),
            audit: self.audit.clone(),
        }))
    }
}

pub struct CreditFacilityProcessingJobRunner {
    config: CreditFacilityJobConfig,
    credit_facility_repo: CreditFacilityRepo,
    ledger: Ledger,
    jobs: Jobs,
    audit: Audit,
}

impl CreditFacilityProcessingJobRunner {
    #[es_entity::retry_on_concurrent_modification]
    async fn record_interest_accrual(
        &self,
        db: &mut es_entity::DbOp<'_>,
        audit_info: &AuditInfo,
    ) -> Result<CreditFacilityInterestAccrual, CreditFacilityError> {
        let mut credit_facility = self
            .credit_facility_repo
            .find_by_id(self.config.credit_facility_id)
            .await?;

        let interest_accrual = credit_facility.record_interest_accrual(audit_info.clone())?;
        self.credit_facility_repo
            .update_in_op(db, &mut credit_facility)
            .await?;

        Ok(interest_accrual)
    }
}

#[async_trait]
impl JobRunner for CreditFacilityProcessingJobRunner {
    #[instrument(
        name = "credit-facility.interest-accruals.job",
        skip(self, current_job),
        fields(attempt)
    )]
    async fn run(
        &self,
        current_job: CurrentJob,
    ) -> Result<JobCompletion, Box<dyn std::error::Error>> {
        let span = tracing::Span::current();
        span.record("attempt", current_job.attempt());

        let mut db = self.credit_facility_repo.begin_op().await?;
        let audit_info = self
            .audit
            .record_system_entry_in_tx(
                db.tx(),
                Object::CreditFacility,
                CreditFacilityAction::RecordInterest,
            )
            .await?;

        let interest_accrual = self.record_interest_accrual(&mut db, &audit_info).await?;
        self.ledger
            .record_credit_facility_interest_accrual(interest_accrual)
            .await?;

        let mut credit_facility = self
            .credit_facility_repo
            .find_by_id_in_tx(db.tx(), self.config.credit_facility_id)
            .await?;
        let periods = match credit_facility.start_interest_accrual(audit_info)? {
            Some(periods) => periods,
            None => {
                println!(
                    "Credit Facility interest accrual job completed for credit_facility: {:?}",
                    self.config.credit_facility_id
                );

                return Ok(JobCompletion::CompleteWithOp(db));
            }
        };

        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;

        match self
            .jobs
            .create_and_spawn_at_in_op(
                &mut db,
                credit_facility
                    .interest_accrual_in_progress()
                    .expect("Expected accrual missing")
                    .id,
                interest_incurrences::CreditFacilityJobConfig {
                    credit_facility_id: credit_facility.id,
                },
                periods.incurrence.end,
            )
            .await
        {
            Ok(_) | Err(JobError::DuplicateId) => (),
            Err(err) => Err(err)?,
        };

        self.credit_facility_repo
            .update_in_op(&mut db, &mut credit_facility)
            .await?;

        return Ok(JobCompletion::RescheduleAtWithOp(db, periods.accrual.end));
    }
}