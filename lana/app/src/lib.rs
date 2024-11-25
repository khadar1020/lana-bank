#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![cfg_attr(feature = "fail-on-warnings", deny(clippy::all))]

pub mod app;
pub mod applicant;
pub mod authorization;
pub mod credit_facility;
pub mod customer;
pub mod data_export;
pub mod deposit;
pub mod document;
pub mod ledger;
pub mod price;
pub mod primitives;
pub mod report;
pub mod service_account;
pub mod storage;
pub mod terms;
pub mod terms_template;
mod time;
pub mod withdrawal;

pub mod outbox {
    pub type Outbox = outbox::Outbox<lana_events::LanaEvent>;
}

pub mod dashboard {
    pub type Dashboard = dashboard::Dashboard<crate::authorization::Authorization>;
    pub use dashboard::DashboardValues;
}

pub mod user {
    pub use core_user::{error, User};
    pub type Users = core_user::Users<crate::audit::Audit, lana_events::LanaEvent>;
}

pub mod job {
    pub use job::*;
}

pub mod governance {
    use crate::authorization::Authorization;
    use lana_events::LanaEvent;
    pub type Governance = governance::Governance<Authorization, LanaEvent>;
    pub use crate::credit_facility::APPROVE_CREDIT_FACILITY_PROCESS;
    pub use crate::credit_facility::APPROVE_DISBURSAL_PROCESS;
    pub use crate::withdrawal::APPROVE_WITHDRAWAL_PROCESS;
}

pub mod audit {
    use crate::{
        authorization::{LanaAction, LanaObject},
        primitives::Subject,
    };

    pub use audit::{error, AuditCursor, AuditEntryId, AuditInfo, AuditSvc};
    pub type Audit = audit::Audit<Subject, LanaObject, LanaAction>;
    pub type AuditEntry = audit::AuditEntry<Subject, LanaObject, LanaAction>;
}