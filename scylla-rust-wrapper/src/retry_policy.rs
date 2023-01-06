use scylla::retry_policy::{DefaultRetryPolicy, FallthroughRetryPolicy};
use scylla::transport::downgrading_consistency_retry_policy::DowngradingConsistencyRetryPolicy;
use std::sync::Arc;

use crate::argconv::ArcFFI;

pub enum RetryPolicy {
    DefaultRetryPolicy(DefaultRetryPolicy),
    FallthroughRetryPolicy(FallthroughRetryPolicy),
    DowngradingConsistencyRetryPolicy(DowngradingConsistencyRetryPolicy),
}

pub type CassRetryPolicy = RetryPolicy;

impl ArcFFI for CassRetryPolicy {}

#[no_mangle]
pub extern "C" fn cass_retry_policy_default_new() -> *const CassRetryPolicy {
    ArcFFI::into_ptr(Arc::new(RetryPolicy::DefaultRetryPolicy(
        DefaultRetryPolicy,
    )))
}

#[no_mangle]
pub extern "C" fn cass_retry_policy_downgrading_consistency_new() -> *const CassRetryPolicy {
    ArcFFI::into_ptr(Arc::new(RetryPolicy::DowngradingConsistencyRetryPolicy(
        DowngradingConsistencyRetryPolicy,
    )))
}

#[no_mangle]
pub extern "C" fn cass_retry_policy_fallthrough_new() -> *const CassRetryPolicy {
    ArcFFI::into_ptr(Arc::new(RetryPolicy::FallthroughRetryPolicy(
        FallthroughRetryPolicy,
    )))
}

#[no_mangle]
pub unsafe extern "C" fn cass_retry_policy_free(retry_policy: *const CassRetryPolicy) {
    ArcFFI::free(retry_policy);
}
