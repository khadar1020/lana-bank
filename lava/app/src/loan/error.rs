use thiserror::Error;

use crate::primitives::*;

#[derive(Error, Debug)]
pub enum LoanError {
    #[error("LoanError - Sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("LoanError - LedgerError: {0}")]
    LedgerError(#[from] crate::ledger::error::LedgerError),
    #[error("LoanError - UserError: {0}")]
    UserError(#[from] crate::user::error::UserError),
    #[error("LoanError - PriceError: {0}")]
    PriceError(#[from] crate::price::error::PriceError),
    #[error("LoanError - LoanTermsError: {0}")]
    LoanTermsError(#[from] crate::terms::error::TermsError),
    #[error("LoanError - ConversionError: {0}")]
    ConversionError(#[from] crate::primitives::ConversionError),
    #[error("LoanError - UserError: '{0}'")]
    CustomerError(#[from] crate::customer::error::CustomerError),
    #[error("LoanError - UserNotFound: {0}")]
    CustomerNotFound(CustomerId),
    #[error("LoanError - UserNotAllowedToCreateLoan: {0}")]
    CustomerNotAllowedToCreateLoan(CustomerId),
    #[error("LoanError - JobError: {0}")]
    JobError(#[from] crate::job::error::JobError),
    #[error("LoanError - AlreadyCompleted")]
    AlreadyCompleted,
    #[error("LoanError - AlreadyApproved")]
    AlreadyApproved,
    #[error("LoanError - UserCannotApproveTwice")]
    UserCannotApproveTwice,
    #[error("LoanError - NotApprovedYet")]
    NotApprovedYet,
    #[error("LoanError - AllInterestAccrualsGeneratedForLoan")]
    AllInterestAccrualsGeneratedForLoan,
    #[error("LoanError - InterestPeriodStartDateInFuture")]
    InterestPeriodStartDateInFuture,
    #[error("LoanError - PaymentExceedsOutstandingLoanAmount: {0} > {1}")]
    PaymentExceedsOutstandingLoanAmount(UsdCents, UsdCents),
    #[error("LoanError - UnexpectedZeroPrincipalAmount: totalAmount({0}), interestAmount({1})")]
    UnexpectedZeroPrincipalAmount(UsdCents, UsdCents),
    #[error("LoanError - TermsNotSet")]
    TermsNotSet,
    #[error("LoanError - AuthorizationError: {0}")]
    AuthorizationError(#[from] crate::authorization::error::AuthorizationError),
    #[error("LoanError - CollateralNotUpdated: before({0}), after({1})")]
    CollateralNotUpdated(Satoshis, Satoshis),
    #[error("LoanError - NoCollateral")]
    NoCollateral,
    #[error("LoanError - BelowMarginLimit")]
    BelowMarginLimit,
    #[error("LoanError - NotFound")]
    NotFound,
    #[error("LoanError - SubjectIsNotUser")]
    SubjectIsNotUser,
}

impl From<es_entity::EsEntityError> for LoanError {
    fn from(e: es_entity::EsEntityError) -> Self {
        match e {
            es_entity::EsEntityError::NotFound => LoanError::NotFound,
            es_entity::EsEntityError::UninitializedFieldError(e) => {
                panic!("Inconsistent data when initializing a Loan entity: {:?}", e)
            }
        }
    }
}