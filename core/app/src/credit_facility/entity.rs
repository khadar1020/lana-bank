use chrono::{DateTime, Utc};
use derive_builder::Builder;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use std::collections::HashSet;

use es_entity::*;

use crate::{
    ledger::{credit_facility::*, customer::CustomerLedgerAccountIds},
    primitives::*,
    terms::{CVLData, CVLPct, CollateralizationState, InterestPeriod, TermValues},
};

use super::{
    disbursement::*, history, CreditFacilityCollateralUpdate, CreditFacilityError,
    NewInterestAccrual,
};

#[derive(EsEvent, Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[es_event(id = "CreditFacilityId")]
pub enum CreditFacilityEvent {
    Initialized {
        id: CreditFacilityId,
        customer_id: CustomerId,
        terms: TermValues,
        facility: UsdCents,
        account_ids: CreditFacilityAccountIds,
        customer_account_ids: CustomerLedgerAccountIds,
        audit_info: AuditInfo,
    },
    ApprovalAdded {
        approving_user_id: UserId,
        approving_user_roles: HashSet<Role>,
        recorded_at: DateTime<Utc>,
        audit_info: AuditInfo,
    },
    Approved {
        tx_id: LedgerTxId,
        recorded_at: DateTime<Utc>,
        audit_info: AuditInfo,
    },
    DisbursementInitiated {
        disbursement_id: DisbursementId,
        idx: DisbursementIdx,
        amount: UsdCents,
        audit_info: AuditInfo,
    },
    DisbursementConcluded {
        idx: DisbursementIdx,
        tx_id: LedgerTxId,
        audit_info: AuditInfo,
        recorded_at: DateTime<Utc>,
    },
    InterestAccrualStarted {
        interest_accrual_id: InterestAccrualId,
        idx: InterestAccrualIdx,
        started_at: DateTime<Utc>,
        audit_info: AuditInfo,
    },
    InterestAccrualConcluded {
        idx: InterestAccrualIdx,
        tx_id: LedgerTxId,
        tx_ref: String,
        amount: UsdCents,
        accrued_at: DateTime<Utc>,
        audit_info: AuditInfo,
    },
    CollateralUpdated {
        tx_id: LedgerTxId,
        tx_ref: String,
        total_collateral: Satoshis,
        abs_diff: Satoshis,
        action: CollateralAction,
        recorded_in_ledger_at: DateTime<Utc>,
        audit_info: AuditInfo,
    },
    CollateralizationChanged {
        state: CollateralizationState,
        collateral: Satoshis,
        outstanding: CreditFacilityReceivable,
        price: PriceOfOneBTC,
        recorded_at: DateTime<Utc>,
        audit_info: AuditInfo,
    },
    PaymentRecorded {
        tx_id: LedgerTxId,
        tx_ref: String,
        disbursement_amount: UsdCents,
        interest_amount: UsdCents,
        audit_info: AuditInfo,
        recorded_in_ledger_at: DateTime<Utc>,
    },
    Completed {
        completed_at: DateTime<Utc>,
        audit_info: AuditInfo,
    },
}

pub struct CreditFacilityApproval {
    pub user_id: UserId,
    pub approved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditFacilityReceivable {
    pub disbursed: UsdCents,
    pub interest: UsdCents,
}

impl From<CreditFacilityBalance> for CreditFacilityReceivable {
    fn from(balance: CreditFacilityBalance) -> Self {
        Self {
            disbursed: balance.disbursed_receivable,
            interest: balance.accrued_interest_receivable,
        }
    }
}

impl CreditFacilityReceivable {
    pub fn total(&self) -> UsdCents {
        self.interest + self.disbursed
    }

    pub fn disbursed_cvl(&self, collateral: Satoshis) -> CVLData {
        CVLData::new(collateral, self.total())
    }

    pub fn total_cvl(&self, collateral: Satoshis, facility_remaining: UsdCents) -> CVLData {
        CVLData::new(collateral, self.total() + facility_remaining)
    }

    fn allocate_payment(
        &self,
        amount: UsdCents,
    ) -> Result<CreditFacilityPaymentAmounts, CreditFacilityError> {
        if self.total() < amount {
            return Err(
                CreditFacilityError::PaymentExceedsOutstandingCreditFacilityAmount(
                    amount,
                    self.total(),
                ),
            );
        }

        let mut remaining = amount;

        let interest = std::cmp::min(amount, self.interest);
        remaining -= interest;

        let disbursement = std::cmp::min(remaining, self.disbursed);
        remaining -= disbursement;

        Ok(CreditFacilityPaymentAmounts {
            interest,
            disbursement,
        })
    }
}

pub struct FacilityCVLData {
    pub total: CVLData,
    pub disbursed: CVLData,
}

impl FacilityCVLData {
    pub fn cvl(&self, price: PriceOfOneBTC) -> FacilityCVL {
        FacilityCVL {
            total: self.total.cvl(price),
            disbursed: self.disbursed.cvl(price),
        }
    }
}

pub struct FacilityCVL {
    pub total: CVLPct,
    pub disbursed: CVLPct,
}

impl FacilityCVL {
    fn is_approval_allowed(&self, terms: TermValues) -> Result<(), CreditFacilityError> {
        if self.total < terms.margin_call_cvl {
            return Err(CreditFacilityError::BelowMarginLimit);
        }
        Ok(())
    }
}

#[derive(EsEntity, Builder)]
#[builder(pattern = "owned", build_fn(error = "EsEntityError"))]
pub struct CreditFacility {
    pub id: CreditFacilityId,
    pub customer_id: CustomerId,
    pub terms: TermValues,
    pub account_ids: CreditFacilityAccountIds,
    pub customer_account_ids: CustomerLedgerAccountIds,
    #[builder(setter(strip_option), default)]
    pub approved_at: Option<DateTime<Utc>>,
    #[builder(setter(strip_option), default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub(super) events: EntityEvents<CreditFacilityEvent>,
}

impl CreditFacility {
    pub fn created_at(&self) -> DateTime<Utc> {
        self.events
            .entity_first_persisted_at()
            .expect("entity_first_persisted_at not found")
    }

    pub(super) fn disbursement_id_from_idx(&self, idx: DisbursementIdx) -> Option<DisbursementId> {
        self.events.iter_all().find_map(|event| match event {
            CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: id,
                idx: i,
                ..
            } if i == &idx => Some(*id),
            _ => None,
        })
    }

    pub fn initial_facility(&self) -> UsdCents {
        for event in self.events.iter_all() {
            match event {
                CreditFacilityEvent::Initialized { facility, .. } => return *facility,
                _ => continue,
            }
        }
        UsdCents::ZERO
    }

    pub fn initial_disbursement_recorded_at(&self) -> Option<DateTime<Utc>> {
        self.events.iter_all().find_map(|event| match event {
            CreditFacilityEvent::DisbursementConcluded { recorded_at, .. } => Some(*recorded_at),
            _ => None,
        })
    }

    fn total_disbursed(&self) -> UsdCents {
        let mut amounts = std::collections::HashMap::new();
        self.events
            .iter_all()
            .fold(UsdCents::from(0), |mut total_sum, event| {
                match event {
                    CreditFacilityEvent::DisbursementInitiated { idx, amount, .. } => {
                        amounts.insert(*idx, *amount);
                    }
                    CreditFacilityEvent::DisbursementConcluded { idx, .. } => {
                        if let Some(amount) = amounts.remove(idx) {
                            total_sum += amount;
                        }
                    }
                    _ => (),
                }
                total_sum
            })
    }

    fn facility_remaining(&self) -> UsdCents {
        self.initial_facility() - self.total_disbursed()
    }

    fn interest_accrued(&self) -> UsdCents {
        self.events
            .iter_all()
            .filter_map(|event| match event {
                CreditFacilityEvent::InterestAccrualConcluded { amount, .. } => Some(*amount),
                _ => None,
            })
            .fold(UsdCents::ZERO, |acc, amount| acc + amount)
    }

    fn disbursed_payments(&self) -> UsdCents {
        self.events
            .iter_all()
            .filter_map(|event| match event {
                CreditFacilityEvent::PaymentRecorded {
                    disbursement_amount,
                    ..
                } => Some(*disbursement_amount),
                _ => None,
            })
            .fold(UsdCents::ZERO, |acc, amount| acc + amount)
    }

    fn interest_payments(&self) -> UsdCents {
        self.events
            .iter_all()
            .filter_map(|event| match event {
                CreditFacilityEvent::PaymentRecorded {
                    interest_amount, ..
                } => Some(*interest_amount),
                _ => None,
            })
            .fold(UsdCents::ZERO, |acc, amount| acc + amount)
    }

    pub fn history(&self) -> Vec<history::CreditFacilityHistoryEntry> {
        history::project(self.events.iter_all())
    }

    pub(super) fn is_approved(&self) -> bool {
        for event in self.events.iter_all() {
            match event {
                CreditFacilityEvent::Approved { .. } => return true,
                _ => continue,
            }
        }
        false
    }

    pub(super) fn is_expired(&self) -> bool {
        self.expires_at
            .map_or(false, |expires_at| Utc::now() > expires_at)
    }

    pub fn status(&self) -> CreditFacilityStatus {
        if self.is_completed() {
            CreditFacilityStatus::Closed
        } else if self.is_expired() {
            CreditFacilityStatus::Expired
        } else if self.is_approved() {
            CreditFacilityStatus::Active
        } else if self.is_fully_collateralized() {
            CreditFacilityStatus::PendingApproval
        } else {
            CreditFacilityStatus::PendingCollateralization
        }
    }

    fn approval_threshold_met(&self) -> bool {
        let mut n_admin = 0;
        let mut n_bank_manager = 0;

        for event in self.events.iter_all() {
            if let CreditFacilityEvent::ApprovalAdded {
                approving_user_roles,
                ..
            } = event
            {
                if approving_user_roles.contains(&Role::Superuser) {
                    return true;
                } else if approving_user_roles.contains(&Role::Admin) {
                    n_admin += 1;
                } else {
                    n_bank_manager += 1;
                }
            }
        }

        n_admin >= 1 && n_admin + n_bank_manager >= 2
    }

    fn has_user_previously_approved(&self, user_id: UserId) -> bool {
        for event in self.events.iter_all() {
            match event {
                CreditFacilityEvent::ApprovalAdded {
                    approving_user_id, ..
                } => {
                    if user_id == *approving_user_id {
                        return true;
                    }
                }
                _ => continue,
            }
        }
        false
    }

    pub(super) fn add_approval(
        &mut self,
        approving_user_id: UserId,
        approving_user_roles: HashSet<Role>,
        audit_info: AuditInfo,
        price: PriceOfOneBTC,
    ) -> Result<Option<CreditFacilityApprovalData>, CreditFacilityError> {
        if self.has_user_previously_approved(approving_user_id) {
            return Err(CreditFacilityError::UserCannotApproveTwice);
        }

        if self.is_approved() {
            return Err(CreditFacilityError::AlreadyApproved);
        }

        if self.collateral() == Satoshis::ZERO {
            return Err(CreditFacilityError::NoCollateral);
        }

        self.facility_cvl_data()
            .cvl(price)
            .is_approval_allowed(self.terms)?;

        self.events.push(CreditFacilityEvent::ApprovalAdded {
            approving_user_id,
            approving_user_roles,
            audit_info,
            recorded_at: Utc::now(),
        });

        if self.approval_threshold_met() {
            let tx_ref = format!("{}-approval", self.id);
            Ok(Some(CreditFacilityApprovalData {
                facility: self.initial_facility(),
                tx_ref,
                tx_id: LedgerTxId::new(),
                credit_facility_account_ids: self.account_ids,
                customer_account_ids: self.customer_account_ids,
            }))
        } else {
            Ok(None)
        }
    }

    pub(super) fn confirm_approval(
        &mut self,
        CreditFacilityApprovalData { tx_id, .. }: CreditFacilityApprovalData,
        executed_at: DateTime<Utc>,
        audit_info: AuditInfo,
    ) {
        self.approved_at = Some(executed_at);
        self.expires_at = Some(self.terms.duration.expiration_date(executed_at));
        self.events.push(CreditFacilityEvent::Approved {
            tx_id,
            audit_info,
            recorded_at: executed_at,
        });
    }

    pub fn approvals(&self) -> Vec<CreditFacilityApproval> {
        let mut approvals = Vec::new();

        for event in self.events.iter_all().rev() {
            if let CreditFacilityEvent::ApprovalAdded {
                approving_user_id,
                recorded_at,
                ..
            } = event
            {
                approvals.push(CreditFacilityApproval {
                    user_id: *approving_user_id,
                    approved_at: *recorded_at,
                });
            }
        }

        approvals
    }

    pub(super) fn initiate_disbursement(
        &mut self,
        audit_info: AuditInfo,
        amount: UsdCents,
    ) -> Result<NewDisbursement, CreditFacilityError> {
        if self.is_expired() {
            return Err(CreditFacilityError::AlreadyExpired);
        }

        if self.is_disbursement_in_progress() {
            return Err(CreditFacilityError::DisbursementInProgress);
        }

        let idx = self
            .events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::DisbursementInitiated { idx, .. } => Some(idx.next()),
                _ => None,
            })
            .unwrap_or(DisbursementIdx::FIRST);

        let id = DisbursementId::new();

        self.events
            .push(CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: id,
                idx,
                amount,
                audit_info,
            });

        Ok(NewDisbursement::builder()
            .id(id)
            .credit_facility_id(self.id)
            .idx(idx)
            .amount(amount)
            .account_ids(self.account_ids)
            .customer_account_ids(self.customer_account_ids)
            .audit_info(audit_info)
            .build()
            .expect("could not build new disbursement"))
    }

    pub(super) fn confirm_disbursement(
        &mut self,
        disbursement: &Disbursement,
        tx_id: LedgerTxId,
        executed_at: DateTime<Utc>,
        audit_info: AuditInfo,
    ) {
        self.events
            .push(CreditFacilityEvent::DisbursementConcluded {
                idx: disbursement.idx,
                recorded_at: executed_at,
                tx_id,
                audit_info,
            });
    }

    fn is_disbursement_in_progress(&self) -> bool {
        for event in self.events.iter_all().rev() {
            if let CreditFacilityEvent::DisbursementInitiated { .. } = event {
                return true;
            }
            if let CreditFacilityEvent::DisbursementConcluded { .. } = event {
                return false;
            }
        }

        false
    }

    fn next_interest_accrual_period(&self) -> Result<Option<InterestPeriod>, CreditFacilityError> {
        let last_accrual_start_date = self.events.iter_all().rev().find_map(|event| match event {
            CreditFacilityEvent::InterestAccrualStarted { started_at, .. } => Some(*started_at),
            _ => None,
        });

        let interval = self.terms.accrual_interval;
        let full_period = match last_accrual_start_date {
            Some(last_accrual_start_date) => interval.period_from(last_accrual_start_date).next(),
            None => interval.period_from(
                self.initial_disbursement_recorded_at()
                    .ok_or(CreditFacilityError::NoDisbursementsApprovedYet)?,
            ),
        };

        Ok(full_period.truncate(self.expires_at.expect("Facility is already approved")))
    }

    pub(super) fn start_interest_accrual(
        &mut self,
        audit_info: AuditInfo,
    ) -> Result<Option<NewInterestAccrual>, CreditFacilityError> {
        let accrual_starts_at = match self.next_interest_accrual_period()? {
            Some(period) => period,
            None => return Ok(None),
        }
        .start;
        if accrual_starts_at > Utc::now() {
            return Err(CreditFacilityError::InterestAccrualWithInvalidFutureStartDate);
        }

        let idx = self
            .events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::InterestAccrualStarted { idx, .. } => Some(idx.next()),
                _ => None,
            })
            .unwrap_or(InterestAccrualIdx::FIRST);
        let id = InterestAccrualId::new();
        self.events
            .push(CreditFacilityEvent::InterestAccrualStarted {
                interest_accrual_id: id,
                idx,
                started_at: accrual_starts_at,
                audit_info,
            });

        Ok(Some(
            NewInterestAccrual::builder()
                .id(id)
                .credit_facility_id(self.id)
                .idx(idx)
                .started_at(accrual_starts_at)
                .facility_expires_at(self.expires_at.expect("Facility is already approved"))
                .terms(self.terms)
                .audit_info(audit_info)
                .build()
                .expect("could not build new interest accrual"),
        ))
    }

    pub fn confirm_interest_accrual(
        &mut self,
        CreditFacilityInterestAccrual {
            interest,
            tx_ref,
            tx_id,
            ..
        }: CreditFacilityInterestAccrual,
        idx: InterestAccrualIdx,
        executed_at: DateTime<Utc>,
        audit_info: AuditInfo,
    ) {
        self.events
            .push(CreditFacilityEvent::InterestAccrualConcluded {
                idx,
                tx_id,
                tx_ref,
                amount: interest,
                accrued_at: executed_at,
                audit_info,
            });
    }

    pub fn interest_accrual_in_progress(&self) -> Option<InterestAccrualId> {
        self.events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::InterestAccrualConcluded { .. } => Some(None),
                CreditFacilityEvent::InterestAccrualStarted {
                    interest_accrual_id: id,
                    ..
                } => Some(Some(*id)),
                _ => None,
            })
            .and_then(|id| id)
    }

    pub fn outstanding(&self) -> CreditFacilityReceivable {
        CreditFacilityReceivable {
            disbursed: self.total_disbursed() - self.disbursed_payments(),
            interest: self.interest_accrued() - self.interest_payments(),
        }
    }

    pub fn can_be_completed(&self) -> bool {
        self.outstanding().total().is_zero()
    }

    pub fn collateral(&self) -> Satoshis {
        self.events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::CollateralUpdated {
                    total_collateral, ..
                } => Some(*total_collateral),
                _ => None,
            })
            .unwrap_or(Satoshis::ZERO)
    }

    pub fn facility_cvl_data(&self) -> FacilityCVLData {
        let total = self
            .outstanding()
            .total_cvl(self.collateral(), self.facility_remaining());
        let disbursed = self.outstanding().disbursed_cvl(self.collateral());

        FacilityCVLData { total, disbursed }
    }

    pub(super) fn initiate_repayment(
        &self,
        amount: UsdCents,
    ) -> Result<CreditFacilityRepayment, CreditFacilityError> {
        let amounts = self.outstanding().allocate_payment(amount)?;

        let tx_ref = format!("{}-payment-{}", self.id, self.count_recorded_payments() + 1);

        let res = CreditFacilityRepayment {
            tx_id: LedgerTxId::new(),
            tx_ref,
            credit_facility_account_ids: self.account_ids,
            customer_account_ids: self.customer_account_ids,
            amounts,
        };

        Ok(res)
    }

    pub fn confirm_repayment(
        &mut self,
        repayment: CreditFacilityRepayment,
        recorded_at: DateTime<Utc>,
        audit_info: AuditInfo,
        price: PriceOfOneBTC,
        upgrade_buffer_cvl_pct: CVLPct,
    ) {
        self.events.push(CreditFacilityEvent::PaymentRecorded {
            tx_id: repayment.tx_id,
            tx_ref: repayment.tx_ref,
            disbursement_amount: repayment.amounts.disbursement,
            interest_amount: repayment.amounts.interest,
            audit_info,
            recorded_in_ledger_at: recorded_at,
        });

        self.maybe_update_collateralization(price, upgrade_buffer_cvl_pct, audit_info);
    }

    fn count_recorded_payments(&self) -> usize {
        self.events
            .iter_all()
            .filter(|event| matches!(event, CreditFacilityEvent::PaymentRecorded { .. }))
            .count()
    }

    pub fn last_collateralization_state(&self) -> CollateralizationState {
        if self.is_completed() {
            return CollateralizationState::NoCollateral;
        }

        self.events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::CollateralizationChanged { state, .. } => Some(*state),
                _ => None,
            })
            .unwrap_or(CollateralizationState::NoCollateral)
    }

    pub fn is_fully_collateralized(&self) -> bool {
        self.last_collateralization_state() == CollateralizationState::FullyCollateralized
    }

    pub fn maybe_update_collateralization(
        &mut self,
        price: PriceOfOneBTC,
        upgrade_buffer_cvl_pct: CVLPct,
        audit_info: AuditInfo,
    ) -> Option<CollateralizationState> {
        let facility_cvl = self.facility_cvl_data().cvl(price);
        let last_collateralization_state = self.last_collateralization_state();

        let collateralization_update =
            match self.status() {
                CreditFacilityStatus::PendingCollateralization
                | CreditFacilityStatus::PendingApproval => facility_cvl
                    .total
                    .collateralization_update(self.terms, last_collateralization_state, None, true),
                CreditFacilityStatus::Active | CreditFacilityStatus::Expired => {
                    let cvl = if self.total_disbursed() == UsdCents::ZERO {
                        facility_cvl.total
                    } else {
                        facility_cvl.disbursed
                    };

                    cvl.collateralization_update(
                        self.terms,
                        last_collateralization_state,
                        Some(upgrade_buffer_cvl_pct),
                        false,
                    )
                }
                CreditFacilityStatus::Closed => Some(CollateralizationState::NoCollateral),
            };

        if let Some(calculated_collateralization) = collateralization_update {
            self.events
                .push(CreditFacilityEvent::CollateralizationChanged {
                    state: calculated_collateralization,
                    collateral: self.collateral(),
                    outstanding: self.outstanding(),
                    price,
                    recorded_at: Utc::now(),
                    audit_info,
                });

            return Some(calculated_collateralization);
        }

        None
    }

    fn count_collateral_adjustments(&self) -> usize {
        self.events
            .iter_all()
            .filter(|event| matches!(event, CreditFacilityEvent::CollateralUpdated { .. }))
            .count()
    }

    pub(super) fn initiate_collateral_update(
        &self,
        updated_collateral: Satoshis,
    ) -> Result<CreditFacilityCollateralUpdate, CreditFacilityError> {
        let current_collateral = self.collateral();
        let diff =
            SignedSatoshis::from(updated_collateral) - SignedSatoshis::from(current_collateral);

        if diff == SignedSatoshis::ZERO {
            return Err(CreditFacilityError::CollateralNotUpdated(
                current_collateral,
                updated_collateral,
            ));
        }

        let (collateral, action) = if diff > SignedSatoshis::ZERO {
            (Satoshis::try_from(diff)?, CollateralAction::Add)
        } else {
            (Satoshis::try_from(diff.abs())?, CollateralAction::Remove)
        };

        let tx_ref = format!(
            "{}-collateral-{}",
            self.id,
            self.count_collateral_adjustments() + 1
        );

        let tx_id = LedgerTxId::new();

        Ok(CreditFacilityCollateralUpdate {
            abs_diff: collateral,
            credit_facility_account_ids: self.account_ids,
            tx_ref,
            tx_id,
            action,
        })
    }

    pub(super) fn confirm_collateral_update(
        &mut self,
        CreditFacilityCollateralUpdate {
            tx_id,
            tx_ref,
            abs_diff,
            action,
            ..
        }: CreditFacilityCollateralUpdate,
        executed_at: DateTime<Utc>,
        audit_info: AuditInfo,
        price: PriceOfOneBTC,
        upgrade_buffer_cvl_pct: CVLPct,
    ) -> Option<CollateralizationState> {
        let mut total_collateral = self.collateral();
        total_collateral = match action {
            CollateralAction::Add => total_collateral + abs_diff,
            CollateralAction::Remove => total_collateral - abs_diff,
        };
        self.events.push(CreditFacilityEvent::CollateralUpdated {
            tx_id,
            tx_ref,
            total_collateral,
            abs_diff,
            action,
            recorded_in_ledger_at: executed_at,
            audit_info,
        });

        self.maybe_update_collateralization(price, upgrade_buffer_cvl_pct, audit_info)
    }

    fn is_completed(&self) -> bool {
        self.events
            .iter_all()
            .any(|event| matches!(event, CreditFacilityEvent::Completed { .. }))
    }

    pub(super) fn initiate_completion(
        &self,
    ) -> Result<CreditFacilityCompletion, CreditFacilityError> {
        if self.is_completed() {
            return Err(CreditFacilityError::AlreadyCompleted);
        }
        if !self.outstanding().total().is_zero() {
            return Err(CreditFacilityError::OutstandingAmount);
        }

        Ok(CreditFacilityCompletion {
            tx_id: LedgerTxId::new(),
            tx_ref: format!("{}-completion", self.id),
            collateral: self.collateral(),
            credit_facility_account_ids: self.account_ids,
            customer_account_ids: self.customer_account_ids,
        })
    }

    pub(super) fn collateralization_ratio(&self) -> Option<Decimal> {
        let amount = if self.status() == CreditFacilityStatus::PendingCollateralization
            || self.status() == CreditFacilityStatus::PendingApproval
            || self.total_disbursed() == UsdCents::ZERO
        {
            self.initial_facility()
        } else {
            self.outstanding().total()
        };

        if amount > UsdCents::ZERO {
            Some(
                rust_decimal::Decimal::from(self.collateral().into_inner())
                    / Decimal::from(amount.into_inner()),
            )
        } else {
            None
        }
    }

    pub(super) fn confirm_completion(
        &mut self,
        CreditFacilityCompletion {
            tx_id,
            tx_ref,
            collateral,
            ..
        }: CreditFacilityCompletion,
        executed_at: DateTime<Utc>,
        audit_info: AuditInfo,
        price: PriceOfOneBTC,
        upgrade_buffer_cvl_pct: CVLPct,
    ) {
        self.confirm_collateral_update(
            CreditFacilityCollateralUpdate {
                credit_facility_account_ids: self.account_ids,
                tx_id,
                tx_ref,
                abs_diff: collateral,
                action: CollateralAction::Remove,
            },
            executed_at,
            audit_info,
            price,
            upgrade_buffer_cvl_pct,
        );

        self.events.push(CreditFacilityEvent::Completed {
            completed_at: executed_at,
            audit_info,
        });
    }
}

impl TryFromEvents<CreditFacilityEvent> for CreditFacility {
    fn try_from_events(events: EntityEvents<CreditFacilityEvent>) -> Result<Self, EsEntityError> {
        let mut builder = CreditFacilityBuilder::default();
        let mut terms = None;
        for event in events.iter_all() {
            match event {
                CreditFacilityEvent::Initialized {
                    id,
                    customer_id,
                    account_ids,
                    customer_account_ids,
                    terms: t,
                    ..
                } => {
                    terms = Some(*t);
                    builder = builder
                        .id(*id)
                        .customer_id(*customer_id)
                        .terms(*t)
                        .account_ids(*account_ids)
                        .customer_account_ids(*customer_account_ids)
                }
                CreditFacilityEvent::Approved { recorded_at, .. } => {
                    builder = builder.approved_at(*recorded_at).expires_at(
                        terms
                            .expect("terms should be set")
                            .duration
                            .expiration_date(*recorded_at),
                    )
                }
                CreditFacilityEvent::ApprovalAdded { .. } => (),
                CreditFacilityEvent::DisbursementInitiated { .. } => (),
                CreditFacilityEvent::DisbursementConcluded { .. } => (),
                CreditFacilityEvent::InterestAccrualStarted { .. } => (),
                CreditFacilityEvent::InterestAccrualConcluded { .. } => (),
                CreditFacilityEvent::CollateralUpdated { .. } => (),
                CreditFacilityEvent::CollateralizationChanged { .. } => (),
                CreditFacilityEvent::PaymentRecorded { .. } => (),
                CreditFacilityEvent::Completed { .. } => (),
            }
        }
        builder.events(events).build()
    }
}

#[derive(Debug, Builder)]
pub struct NewCreditFacility {
    #[builder(setter(into))]
    pub(super) id: CreditFacilityId,
    #[builder(setter(into))]
    pub(super) customer_id: CustomerId,
    terms: TermValues,
    facility: UsdCents,
    account_ids: CreditFacilityAccountIds,
    customer_account_ids: CustomerLedgerAccountIds,
    #[builder(setter(into))]
    pub(super) audit_info: AuditInfo,
}

impl NewCreditFacility {
    pub fn builder() -> NewCreditFacilityBuilder {
        NewCreditFacilityBuilder::default()
    }
}

impl IntoEvents<CreditFacilityEvent> for NewCreditFacility {
    fn into_events(self) -> EntityEvents<CreditFacilityEvent> {
        EntityEvents::init(
            self.id,
            [CreditFacilityEvent::Initialized {
                id: self.id,
                audit_info: self.audit_info,
                customer_id: self.customer_id,
                terms: self.terms,
                facility: self.facility,
                account_ids: self.account_ids,
                customer_account_ids: self.customer_account_ids,
            }],
        )
    }
}

#[cfg(test)]
mod test {
    use rust_decimal_macros::dec;

    use crate::{
        credit_facility::*,
        terms::{Duration, InterestInterval},
    };

    use super::*;

    fn default_terms() -> TermValues {
        TermValues::builder()
            .annual_rate(dec!(12))
            .duration(Duration::Months(3))
            .accrual_interval(InterestInterval::EndOfMonth)
            .incurrence_interval(InterestInterval::EndOfDay)
            .liquidation_cvl(dec!(105))
            .margin_call_cvl(dec!(125))
            .initial_cvl(dec!(140))
            .build()
            .expect("should build a valid term")
    }

    fn dummy_audit_info() -> AuditInfo {
        AuditInfo {
            audit_entry_id: AuditEntryId::from(1),
            sub: Subject::from(UserId::new()),
        }
    }

    fn default_price() -> PriceOfOneBTC {
        PriceOfOneBTC::new(UsdCents::from(5000000))
    }

    fn default_upgrade_buffer_cvl_pct() -> CVLPct {
        CVLPct::new(5)
    }

    fn facility_from(events: &Vec<CreditFacilityEvent>) -> CreditFacility {
        CreditFacility::try_from_events(EntityEvents::init(CreditFacilityId::new(), events.clone()))
            .unwrap()
    }

    fn initial_events() -> Vec<CreditFacilityEvent> {
        vec![CreditFacilityEvent::Initialized {
            id: CreditFacilityId::new(),
            audit_info: dummy_audit_info(),
            customer_id: CustomerId::new(),
            facility: UsdCents::from(100),
            terms: default_terms(),
            account_ids: CreditFacilityAccountIds::new(),
            customer_account_ids: CustomerLedgerAccountIds::new(),
        }]
    }

    #[test]
    fn allocate_payment() {
        let outstanding = CreditFacilityReceivable {
            disbursed: UsdCents::from(100),
            interest: UsdCents::from(2),
        };
        assert!(outstanding.allocate_payment(UsdCents::from(200)).is_err());
        assert!(outstanding.allocate_payment(UsdCents::from(100)).is_ok());
    }

    #[test]
    fn is_disbursement_in_progress() {
        let mut events = initial_events();

        let first_idx = DisbursementIdx::FIRST;
        events.push(CreditFacilityEvent::DisbursementInitiated {
            disbursement_id: DisbursementId::new(),
            idx: first_idx,
            amount: UsdCents::ONE,
            audit_info: dummy_audit_info(),
        });
        assert!(matches!(
            facility_from(&events).initiate_disbursement(dummy_audit_info(), UsdCents::ONE),
            Err(CreditFacilityError::DisbursementInProgress)
        ));

        events.push(CreditFacilityEvent::DisbursementConcluded {
            idx: first_idx,
            tx_id: LedgerTxId::new(),
            recorded_at: Utc::now(),
            audit_info: dummy_audit_info(),
        });
        assert!(facility_from(&events)
            .initiate_disbursement(dummy_audit_info(), UsdCents::ONE)
            .is_ok());
    }

    #[test]
    fn interest_accrued() {
        let mut events = initial_events();
        events.extend([
            CreditFacilityEvent::InterestAccrualConcluded {
                idx: InterestAccrualIdx::FIRST,
                tx_id: LedgerTxId::new(),
                tx_ref: "".to_string(),
                amount: UsdCents::from(10),
                accrued_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::InterestAccrualConcluded {
                idx: InterestAccrualIdx::FIRST.next(),
                tx_id: LedgerTxId::new(),
                tx_ref: "".to_string(),
                amount: UsdCents::from(20),
                accrued_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
        ]);
        let credit_facility = facility_from(&events);

        assert_eq!(credit_facility.interest_accrued(), UsdCents::from(30));
    }

    #[test]
    fn outstanding() {
        let mut events = initial_events();
        events.extend([
            CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: DisbursementId::new(),
                idx: DisbursementIdx::FIRST,
                amount: UsdCents::from(100),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::DisbursementConcluded {
                idx: DisbursementIdx::FIRST,
                tx_id: LedgerTxId::new(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
        ]);
        let credit_facility = facility_from(&events);

        assert_eq!(
            credit_facility.outstanding(),
            CreditFacilityReceivable {
                disbursed: UsdCents::from(100),
                interest: UsdCents::ZERO
            }
        );
    }

    #[test]
    fn collateral() {
        let mut credit_facility = facility_from(&initial_events());
        assert_eq!(credit_facility.collateral(), Satoshis::ZERO);

        let credit_facility_collateral_update = credit_facility
            .initiate_collateral_update(Satoshis::from(10000))
            .unwrap();
        credit_facility.confirm_collateral_update(
            credit_facility_collateral_update,
            Utc::now(),
            dummy_audit_info(),
            default_price(),
            default_upgrade_buffer_cvl_pct(),
        );
        assert_eq!(credit_facility.collateral(), Satoshis::from(10000));

        let credit_facility_collateral_update = credit_facility
            .initiate_collateral_update(Satoshis::from(5000))
            .unwrap();
        credit_facility.confirm_collateral_update(
            credit_facility_collateral_update,
            Utc::now(),
            dummy_audit_info(),
            default_price(),
            default_upgrade_buffer_cvl_pct(),
        );
        assert_eq!(credit_facility.collateral(), Satoshis::from(5000));
    }

    #[test]
    fn collateralization_ratio() {
        let events = initial_events();
        let mut credit_facility = facility_from(&events);
        assert_eq!(
            credit_facility.collateralization_ratio(),
            Some(Decimal::ZERO)
        );

        let credit_facility_collateral_update = credit_facility
            .initiate_collateral_update(Satoshis::from(500))
            .unwrap();
        credit_facility.confirm_collateral_update(
            credit_facility_collateral_update,
            Utc::now(),
            dummy_audit_info(),
            default_price(),
            default_upgrade_buffer_cvl_pct(),
        );
        assert_eq!(credit_facility.collateralization_ratio(), Some(dec!(5)));
    }

    #[test]
    fn collateralization_ratio_when_active_disbursement() {
        let mut events = initial_events();
        let mut roles = std::collections::HashSet::new();
        roles.insert(Role::Admin);
        events.extend([
            CreditFacilityEvent::CollateralUpdated {
                tx_id: LedgerTxId::new(),
                tx_ref: "tx-ref".to_string(),
                total_collateral: Satoshis::from(500),
                abs_diff: Satoshis::from(500),
                action: CollateralAction::Add,
                recorded_in_ledger_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::ApprovalAdded {
                approving_user_id: UserId::new(),
                approving_user_roles: roles.clone(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::ApprovalAdded {
                approving_user_id: UserId::new(),
                approving_user_roles: roles,
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::Approved {
                tx_id: LedgerTxId::new(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: DisbursementId::new(),
                idx: DisbursementIdx::FIRST,
                amount: UsdCents::from(10),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::DisbursementConcluded {
                idx: DisbursementIdx::FIRST,
                tx_id: LedgerTxId::new(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
        ]);

        let credit_facility = facility_from(&events);
        assert_eq!(credit_facility.collateralization_ratio(), Some(dec!(50)));
    }

    #[test]
    fn next_interest_accrual_period_handles_first_and_second_periods() {
        let mut events = initial_events();
        events.extend([
            CreditFacilityEvent::Approved {
                tx_id: LedgerTxId::new(),
                audit_info: dummy_audit_info(),
                recorded_at: Utc::now(),
            },
            CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: DisbursementId::new(),
                idx: DisbursementIdx::FIRST,
                amount: UsdCents::from(100),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::DisbursementConcluded {
                idx: DisbursementIdx::FIRST,
                tx_id: LedgerTxId::new(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
        ]);
        let mut credit_facility = facility_from(&events);

        let first_period = credit_facility
            .next_interest_accrual_period()
            .unwrap()
            .unwrap();
        let InterestPeriod { start, .. } = first_period;
        assert_eq!(
            Utc::now().format("%Y-%m-%d").to_string(),
            start.format("%Y-%m-%d").to_string()
        );

        let new_accrual = credit_facility
            .start_interest_accrual(dummy_audit_info())
            .unwrap()
            .unwrap();
        let accrual = InterestAccrual::try_from_events(new_accrual.into_events()).unwrap();

        let second_period = credit_facility
            .next_interest_accrual_period()
            .unwrap()
            .unwrap();
        assert_eq!(first_period.next(), second_period);

        credit_facility.confirm_interest_accrual(
            CreditFacilityInterestAccrual {
                interest: UsdCents::ONE,
                tx_ref: "tx_ref".to_string(),
                tx_id: LedgerTxId::new(),
                credit_facility_account_ids: credit_facility.account_ids,
            },
            accrual.idx,
            accrual
                .terms
                .incurrence_interval
                .period_from(accrual.started_at)
                .end,
            dummy_audit_info(),
        );
    }

    #[test]
    fn next_interest_accrual_period_handles_last_period() {
        let mut events = initial_events();
        events.extend([
            CreditFacilityEvent::Approved {
                tx_id: LedgerTxId::new(),
                audit_info: dummy_audit_info(),
                recorded_at: Utc::now(),
            },
            CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: DisbursementId::new(),
                idx: DisbursementIdx::FIRST,
                amount: UsdCents::from(100),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::DisbursementConcluded {
                idx: DisbursementIdx::FIRST,
                tx_id: LedgerTxId::new(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
        ]);
        let mut credit_facility = facility_from(&events);

        let new_accrual = credit_facility
            .start_interest_accrual(dummy_audit_info())
            .unwrap()
            .unwrap();
        let mut accrual = InterestAccrual::try_from_events(new_accrual.into_events()).unwrap();
        let mut next_accrual_period = credit_facility.next_interest_accrual_period().unwrap();
        while next_accrual_period.is_some() {
            credit_facility.confirm_interest_accrual(
                CreditFacilityInterestAccrual {
                    interest: UsdCents::ONE,
                    tx_ref: "tx_ref".to_string(),
                    tx_id: LedgerTxId::new(),
                    credit_facility_account_ids: credit_facility.account_ids,
                },
                accrual.idx,
                accrual
                    .terms
                    .incurrence_interval
                    .period_from(accrual.started_at)
                    .end,
                dummy_audit_info(),
            );

            let new_idx = accrual.idx.next();
            let accrual_starts_at = next_accrual_period.unwrap().start;
            let id = InterestAccrualId::new();
            credit_facility
                .events
                .push(CreditFacilityEvent::InterestAccrualStarted {
                    interest_accrual_id: id,
                    idx: new_idx,
                    started_at: accrual_starts_at,
                    audit_info: dummy_audit_info(),
                });
            let new_accrual = NewInterestAccrual::builder()
                .id(id)
                .credit_facility_id(credit_facility.id)
                .idx(new_idx)
                .started_at(accrual_starts_at)
                .facility_expires_at(
                    credit_facility
                        .expires_at
                        .expect("Facility is already approved"),
                )
                .terms(credit_facility.terms)
                .audit_info(dummy_audit_info())
                .build()
                .unwrap();
            accrual = InterestAccrual::try_from_events(new_accrual.into_events()).unwrap();

            next_accrual_period = credit_facility.next_interest_accrual_period().unwrap();
        }
        assert_eq!(
            accrual.started_at.format("%Y-%m").to_string(),
            credit_facility
                .expires_at
                .unwrap()
                .format("%Y-%m")
                .to_string()
        );
    }

    mod approve {
        use super::*;

        fn bank_manager_role() -> HashSet<Role> {
            let mut roles = HashSet::new();
            roles.insert(Role::BankManager);
            roles
        }

        fn admin_role() -> HashSet<Role> {
            let mut roles = HashSet::new();
            roles.insert(Role::Admin);
            roles
        }

        fn add_approvals(credit_facility: &mut CreditFacility) -> CreditFacilityApprovalData {
            let first_approval = credit_facility.add_approval(
                UserId::new(),
                bank_manager_role(),
                dummy_audit_info(),
                default_price(),
            );
            assert!(first_approval.is_ok());

            let second_approval = credit_facility.add_approval(
                UserId::new(),
                admin_role(),
                dummy_audit_info(),
                default_price(),
            );
            assert!(second_approval.is_ok());

            second_approval
                .unwrap()
                .expect("should return a credit facility approval")
        }

        #[test]
        fn cvl_is_approval_allowed() {
            let terms = default_terms();

            let facility_cvl = FacilityCVL {
                total: terms.margin_call_cvl - CVLPct::from(dec!(1)),
                disbursed: CVLPct::ZERO,
            };
            assert!(matches!(
                facility_cvl.is_approval_allowed(terms),
                Err(CreditFacilityError::BelowMarginLimit),
            ));

            let facility_cvl = FacilityCVL {
                total: terms.margin_call_cvl,
                disbursed: CVLPct::ZERO,
            };
            assert!(matches!(facility_cvl.is_approval_allowed(terms), Ok(())));
        }

        #[test]
        fn prevent_double_approve() {
            let mut credit_facility = facility_from(&initial_events());
            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );
            let credit_facility_approval = add_approvals(&mut credit_facility);
            credit_facility.confirm_approval(
                credit_facility_approval,
                Utc::now(),
                dummy_audit_info(),
            );

            let third_approval = credit_facility.add_approval(
                UserId::new(),
                bank_manager_role(),
                dummy_audit_info(),
                default_price(),
            );
            assert!(matches!(
                third_approval,
                Err(CreditFacilityError::AlreadyApproved)
            ));
        }

        #[test]
        fn check_approved_at() {
            let mut credit_facility = facility_from(&initial_events());
            assert_eq!(credit_facility.approved_at, None);
            assert_eq!(credit_facility.expires_at, None);

            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );
            let approval_time = Utc::now();

            let credit_facility_approval = add_approvals(&mut credit_facility);
            credit_facility.confirm_approval(
                credit_facility_approval,
                approval_time,
                dummy_audit_info(),
            );
            assert_eq!(credit_facility.approved_at, Some(approval_time));
            assert!(credit_facility.expires_at.is_some())
        }

        #[test]
        fn cannot_approve_if_credit_facility_has_no_collateral() {
            let mut credit_facility = facility_from(&initial_events());
            let res = credit_facility.add_approval(
                UserId::new(),
                bank_manager_role(),
                dummy_audit_info(),
                default_price(),
            );
            assert!(matches!(res, Err(CreditFacilityError::NoCollateral)));
        }

        #[test]
        fn reject_credit_facility_approval_below_margin_limit() {
            let mut credit_facility = facility_from(&initial_events());

            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(100))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );

            let first_approval = credit_facility.add_approval(
                UserId::new(),
                bank_manager_role(),
                dummy_audit_info(),
                default_price(),
            );
            assert!(matches!(
                first_approval,
                Err(CreditFacilityError::BelowMarginLimit)
            ));
        }

        #[test]
        fn two_admins_can_approve() {
            let mut credit_facility = facility_from(&initial_events());
            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );
            let _first_admin_approval = credit_facility
                .add_approval(
                    UserId::new(),
                    admin_role(),
                    dummy_audit_info(),
                    default_price(),
                )
                .unwrap();

            let _second_admin_approval = credit_facility
                .add_approval(
                    UserId::new(),
                    admin_role(),
                    dummy_audit_info(),
                    default_price(),
                )
                .unwrap();

            assert!(credit_facility.approval_threshold_met());
        }

        #[test]
        fn admin_and_bank_manager_can_approve() {
            let mut credit_facility = facility_from(&initial_events());
            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );
            let _admin_approval = credit_facility
                .add_approval(
                    UserId::new(),
                    admin_role(),
                    dummy_audit_info(),
                    default_price(),
                )
                .unwrap();

            let _bank_manager_approval = credit_facility
                .add_approval(
                    UserId::new(),
                    bank_manager_role(),
                    dummy_audit_info(),
                    default_price(),
                )
                .unwrap();

            assert!(credit_facility.approval_threshold_met());
        }

        #[test]
        fn user_with_both_admin_and_bank_manager_role_cannot_approve() {
            let mut credit_facility = facility_from(&initial_events());
            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );
            let admin_and_bank_manager =
                admin_role().union(&bank_manager_role()).cloned().collect();
            let _approval = credit_facility
                .add_approval(
                    UserId::new(),
                    admin_and_bank_manager,
                    dummy_audit_info(),
                    default_price(),
                )
                .unwrap();

            assert!(!credit_facility.approval_threshold_met());
        }

        #[test]
        fn two_bank_managers_cannot_approve() {
            let mut credit_facility = facility_from(&initial_events());
            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );
            let _first_bank_manager_approval = credit_facility
                .add_approval(
                    UserId::new(),
                    bank_manager_role(),
                    dummy_audit_info(),
                    default_price(),
                )
                .unwrap();
            let _second_bank_manager_approval = credit_facility
                .add_approval(
                    UserId::new(),
                    bank_manager_role(),
                    dummy_audit_info(),
                    default_price(),
                )
                .unwrap();

            assert!(!credit_facility.approval_threshold_met());
        }

        #[test]
        fn same_user_cannot_approve_twice() {
            let mut credit_facility = facility_from(&initial_events());
            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );

            let user_id = UserId::new();

            let first_approval = credit_facility.add_approval(
                user_id,
                bank_manager_role(),
                dummy_audit_info(),
                default_price(),
            );

            assert!(first_approval.is_ok());

            let second_approval = credit_facility.add_approval(
                user_id,
                bank_manager_role(),
                dummy_audit_info(),
                default_price(),
            );

            assert!(matches!(
                second_approval,
                Err(CreditFacilityError::UserCannotApproveTwice)
            ));
        }

        #[test]
        fn status() {
            let mut credit_facility = facility_from(&initial_events());
            assert_eq!(
                credit_facility.status(),
                CreditFacilityStatus::PendingCollateralization
            );

            let credit_facility_collateral_update = credit_facility
                .initiate_collateral_update(Satoshis::from(10000))
                .unwrap();
            credit_facility.confirm_collateral_update(
                credit_facility_collateral_update,
                Utc::now(),
                dummy_audit_info(),
                default_price(),
                default_upgrade_buffer_cvl_pct(),
            );
            assert_eq!(
                credit_facility.status(),
                CreditFacilityStatus::PendingApproval
            );
            let credit_facility_approval = add_approvals(&mut credit_facility);
            credit_facility.confirm_approval(
                credit_facility_approval,
                Utc::now(),
                dummy_audit_info(),
            );
            assert_eq!(credit_facility.status(), CreditFacilityStatus::Active);
        }
    }

    #[test]
    fn confirm_repayment() {
        let mut events = initial_events();
        events.extend([
            CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: DisbursementId::new(),
                idx: DisbursementIdx::FIRST,
                amount: UsdCents::from(100),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::DisbursementConcluded {
                idx: DisbursementIdx::FIRST,
                tx_id: LedgerTxId::new(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
        ]);
        let mut credit_facility = facility_from(&events);

        let repayment_amount = UsdCents::from(5);
        let repayment = credit_facility
            .initiate_repayment(repayment_amount)
            .unwrap();
        let outstanding_before_repayment = credit_facility.outstanding();

        credit_facility.confirm_repayment(
            repayment,
            Utc::now(),
            dummy_audit_info(),
            default_price(),
            default_upgrade_buffer_cvl_pct(),
        );
        assert_eq!(
            outstanding_before_repayment.total() - credit_facility.outstanding().total(),
            repayment_amount
        );
    }

    #[test]
    fn confirm_completion() {
        let mut events = initial_events();
        events.extend([
            CreditFacilityEvent::DisbursementInitiated {
                disbursement_id: DisbursementId::new(),
                idx: DisbursementIdx::FIRST,
                amount: UsdCents::from(100),
                audit_info: dummy_audit_info(),
            },
            CreditFacilityEvent::DisbursementConcluded {
                idx: DisbursementIdx::FIRST,
                tx_id: LedgerTxId::new(),
                recorded_at: Utc::now(),
                audit_info: dummy_audit_info(),
            },
        ]);
        let mut credit_facility = facility_from(&events);

        let repayment_amount = UsdCents::from(100);
        let repayment = credit_facility
            .initiate_repayment(repayment_amount)
            .unwrap();
        let outstanding_before_repayment = credit_facility.outstanding();

        credit_facility.confirm_repayment(
            repayment,
            Utc::now(),
            dummy_audit_info(),
            default_price(),
            default_upgrade_buffer_cvl_pct(),
        );
        assert_eq!(
            outstanding_before_repayment.total() - credit_facility.outstanding().total(),
            repayment_amount
        );

        let completion = credit_facility.initiate_completion().unwrap();
        credit_facility.confirm_completion(
            completion,
            Utc::now(),
            dummy_audit_info(),
            default_price(),
            default_upgrade_buffer_cvl_pct(),
        );
        assert!(credit_facility.is_completed());
        assert!(credit_facility.status() == CreditFacilityStatus::Closed);
    }
}