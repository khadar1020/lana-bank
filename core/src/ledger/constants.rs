use uuid::{uuid, Uuid};

// Journal
pub(super) const CORE_JOURNAL_ID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");

// Integrations
pub(super) const ON_BALANCE_SHEET_BFX_INTEGRATION_ID: Uuid =
    uuid!("00000000-0000-0000-0000-200000000000");
pub(super) const _OFF_BALANCE_SHEET_BFX_INTEGRATION_ID: Uuid =
    uuid!("10000000-0000-0000-0000-200000000000");

// Reports Account Sets
pub(super) const CHART_OF_ACCOUNTS_ACCOUNT_SET_ID: Uuid =
    uuid!("00000000-0000-0000-0000-100000000001");
pub(super) const TRIAL_BALANCE_ACCOUNT_SET_ID: Uuid = uuid!("00000000-0000-0000-0000-100000000002");
pub(super) const BALANCE_SHEET_ACCOUNT_SET_ID: Uuid = uuid!("00000000-0000-0000-0000-100000000003");
pub(super) const NET_INCOME_ACCOUNT_SET_ID: Uuid = uuid!("00000000-0000-0000-0000-100000000004");

pub(super) const OBS_CHART_OF_ACCOUNTS_ACCOUNT_SET_ID: Uuid =
    uuid!("10000000-0000-0000-0000-100000000001");
pub(super) const OBS_TRIAL_BALANCE_ACCOUNT_SET_ID: Uuid =
    uuid!("10000000-0000-0000-0000-100000000002");

// Account Sets
pub(super) const LOANS_RECEIVABLE_CONTROL_ACCOUNT_SET_ID: Uuid =
    uuid!("00000000-0000-0000-0000-110000000001");
pub(super) const CUSTOMER_CHECKING_CONTROL_ACCOUNT_SET_ID: Uuid =
    uuid!("00000000-0000-0000-0000-120000000001");
pub(super) const INTEREST_REVENUE_CONTROL_ACCOUNT_SET_ID: Uuid =
    uuid!("00000000-0000-0000-0000-140000000001");
pub(super) const CUSTOMER_COLLATERAL_CONTROL_ACCOUNT_SET_ID: Uuid =
    uuid!("00000000-0000-0000-0000-210000000001");
pub(super) const LOANS_COLLATERAL_CONTROL_ACCOUNT_SET_ID: Uuid =
    uuid!("00000000-0000-0000-0000-210000000002");

// Accounts for templates
pub(super) const BANK_SHAREHOLDER_EQUITY_CODE: &str = "BANK.SHAREHOLDER_EQUITY";
pub(super) const BANK_RESERVE_FROM_SHAREHOLDER_CODE: &str = "BANK.RESERVE_FROM_SHAREHOLDER";

// Templates
pub(super) const APPROVE_LOAN_CODE: &str = "APPROVE_LOAN";
pub(super) const INCUR_INTEREST_CODE: &str = "INCUR_INTEREST";
pub(super) const RECORD_PAYMENT_CODE: &str = "RECORD_PAYMENT";
pub(super) const COMPLETE_LOAN_CODE: &str = "COMPLETE_LOAN";
pub(super) const ADD_EQUITY_CODE: &str = "ADD_EQUITY";
