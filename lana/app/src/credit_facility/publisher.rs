const BQ_TABLE_NAME: &str = "credit_facility_events";

use lana_events::{CreditEvent, FacilityCollateralUpdateAction};

use crate::{data_export::Export, outbox::Outbox};

use super::{entity::*, error::*};

#[derive(Clone)]
pub struct CreditFacilityPublisher {
    pub(super) export: Export,
    outbox: Outbox,
}

impl CreditFacilityPublisher {
    pub fn new(export: &Export, outbox: &Outbox) -> Self {
        Self {
            export: export.clone(),
            outbox: outbox.clone(),
        }
    }

    pub async fn publish(
        &self,
        db: &mut es_entity::DbOp<'_>,
        entity: &CreditFacility,
        new_events: es_entity::LastPersisted<'_, CreditFacilityEvent>,
    ) -> Result<(), CreditFacilityError> {
        self.export
            .es_entity_export(db, BQ_TABLE_NAME, new_events.clone())
            .await?;

        use CreditFacilityEvent::*;
        let publish_events = new_events
            .filter_map(|event| match &event.event {
                Initialized { .. } => Some(CreditEvent::FacilityCreated {
                    id: entity.id,
                    created_at: entity.created_at(),
                }),
                Activated { activated_at, .. } => Some(CreditEvent::FacilityActivated {
                    id: entity.id,
                    activated_at: *activated_at,
                }),
                Completed { completed_at, .. } => Some(CreditEvent::FacilityCompleted {
                    id: entity.id,
                    completed_at: *completed_at,
                }),
                DisbursalConcluded {
                    idx,
                    tx_id: Some(_),
                    recorded_at,
                    ..
                } => {
                    let amount = entity.disbursal_amount_from_idx(*idx);
                    Some(CreditEvent::DisbursalExecuted {
                        id: entity.id,
                        amount,
                        recorded_at: *recorded_at,
                    })
                }
                PaymentRecorded {
                    disbursal_amount,
                    interest_amount,
                    recorded_in_ledger_at,
                    ..
                } => Some(CreditEvent::FacilityRepaymentRecorded {
                    id: entity.id,
                    disbursal_amount: *disbursal_amount,
                    interest_amount: *interest_amount,
                    recorded_at: *recorded_in_ledger_at,
                }),
                CollateralUpdated {
                    total_collateral,
                    abs_diff,
                    action,
                    recorded_in_ledger_at,
                    ..
                } => {
                    let action = match action {
                        crate::primitives::CollateralAction::Add => {
                            FacilityCollateralUpdateAction::Add
                        }
                        crate::primitives::CollateralAction::Remove => {
                            FacilityCollateralUpdateAction::Remove
                        }
                    };

                    Some(CreditEvent::FacilityCollateralUpdated {
                        new_amount: *total_collateral,
                        abs_diff: *abs_diff,
                        action,
                        recorded_at: *recorded_in_ledger_at,
                    })
                }

                _ => None,
            })
            .collect::<Vec<_>>();
        self.outbox
            .publish_all_persisted(db.tx(), publish_events)
            .await?;
        Ok(())
    }
}