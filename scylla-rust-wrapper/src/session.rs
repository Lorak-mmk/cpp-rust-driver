use crate::argconv::*;
use crate::cass_error;
use crate::future::{CassFuture, CassResultValue};
use crate::statement::CassStatement;
use crate::statement::Statement;
use scylla::{QueryResult, Session, SessionBuilder};
use std::sync::Arc;
use tokio::sync::RwLock;

type CassSession = Arc<RwLock<Option<Session>>>;

#[no_mangle]
pub unsafe extern "C" fn cass_session_new() -> *mut CassSession {
    Box::into_raw(Box::new(Arc::new(RwLock::new(None))))
}

#[no_mangle]
pub unsafe extern "C" fn cass_session_connect(
    session_raw: *mut CassSession,
    session_builder_raw: *const SessionBuilder,
) -> *const CassFuture {
    let session_opt = ptr_to_ref(session_raw);
    let builder = ptr_to_ref(session_builder_raw);

    CassFuture::make_raw(async move {
        // TODO: Proper error handling
        let session = builder
            .build()
            .await
            .map_err(|_| cass_error::LIB_NO_HOSTS_AVAILABLE)?;

        *session_opt.write().await = Some(session);
        Ok(CassResultValue::Empty)
    })
}

#[no_mangle]
pub unsafe extern "C" fn cass_session_execute(
    session_raw: *mut CassSession,
    statement_raw: *const CassStatement,
) -> *const CassFuture {
    let session_opt = ptr_to_ref(session_raw);
    let statement_opt = ptr_to_ref(statement_raw);
    let bound_values = statement_opt.bound_values.clone();

    let statement = statement_opt.statement.clone();

    CassFuture::make_raw(async move {
        let session_guard = session_opt.read().await;
        let session = session_guard.as_ref().unwrap();

        let query_res: QueryResult = match statement {
            Statement::Simple(query) => session.query(query, bound_values).await,
            Statement::Prepared(prepared) => session.execute(&prepared, bound_values).await,
        }
        .map_err(|_| cass_error::LIB_NO_HOSTS_AVAILABLE)?; // TODO: Proper error handling

        Ok(CassResultValue::QueryResult(Arc::new(query_res)))
    })
}

#[no_mangle]
pub unsafe extern "C" fn cass_session_free(session_raw: *mut CassSession) {
    free_boxed(session_raw);
}
