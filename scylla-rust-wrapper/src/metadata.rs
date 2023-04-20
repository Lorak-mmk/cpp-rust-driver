use crate::argconv::*;
use crate::cass_types::get_column_type_from_cql_type;
use crate::cass_types::CassDataType;
use crate::types::*;
use scylla::transport::topology::{ColumnKind, CqlType, Table};
use std::collections::HashMap;
use std::os::raw::c_char;
use std::sync::Arc;
use std::sync::Weak;

include!(concat!(env!("OUT_DIR"), "/cppdriver_column_type.rs"));

pub struct CassSchemaMeta {
    pub keyspaces: HashMap<String, CassKeyspaceMeta>,
}

impl BoxFFI for CassSchemaMeta {}

pub struct CassKeyspaceMeta {
    pub name: String,

    // User defined type name to type
    pub user_defined_type_data_type: HashMap<String, Arc<CassDataType>>,
    pub tables: HashMap<String, Arc<CassTableMeta>>,
    pub views: HashMap<String, Arc<CassMaterializedViewMeta>>,
}

// Owned by CassSchemaMeta
impl RefFFI for CassKeyspaceMeta {}

pub struct CassTableMeta {
    pub name: String,
    pub columns_metadata: HashMap<String, CassColumnMeta>,
    pub partition_keys: Vec<String>,
    pub clustering_keys: Vec<String>,
    pub views: HashMap<String, Arc<CassMaterializedViewMeta>>,
}

// Either:
// - owned by CassMaterializedViewMeta - won't be given to user
// - Owned by CassKeyspaceMeta (in Arc), referenced (Weak) by CassMaterializedViewMeta
impl RefFFI for CassTableMeta {}

pub struct CassMaterializedViewMeta {
    pub name: String,
    pub view_metadata: CassTableMeta,
    pub base_table: Weak<CassTableMeta>,
}

// Shared ownership by CassKeyspaceMeta and CassTableMeta
impl RefFFI for CassMaterializedViewMeta {}

pub struct CassColumnMeta {
    pub name: String,
    pub column_type: CassDataType,
    pub column_kind: CassColumnType,
}

// Owned by CassTableMeta
impl RefFFI for CassColumnMeta {}

pub unsafe fn create_table_metadata(
    keyspace_name: &str,
    table_name: &str,
    table_metadata: &Table,
    user_defined_types: &HashMap<String, Vec<(String, CqlType)>>,
) -> CassTableMeta {
    let mut columns_metadata = HashMap::new();
    table_metadata
        .columns
        .iter()
        .for_each(|(column_name, column_metadata)| {
            let cass_column_meta = CassColumnMeta {
                name: column_name.clone(),
                column_type: get_column_type_from_cql_type(
                    &column_metadata.type_,
                    user_defined_types,
                    keyspace_name,
                ),
                column_kind: match column_metadata.kind {
                    ColumnKind::Regular => CassColumnType::CASS_COLUMN_TYPE_REGULAR,
                    ColumnKind::Static => CassColumnType::CASS_COLUMN_TYPE_STATIC,
                    ColumnKind::Clustering => CassColumnType::CASS_COLUMN_TYPE_CLUSTERING_KEY,
                    ColumnKind::PartitionKey => CassColumnType::CASS_COLUMN_TYPE_PARTITION_KEY,
                },
            };

            columns_metadata.insert(column_name.clone(), cass_column_meta);
        });

    CassTableMeta {
        name: table_name.to_owned(),
        columns_metadata,
        partition_keys: table_metadata.partition_key.clone(),
        clustering_keys: table_metadata.clustering_key.clone(),
        views: HashMap::new(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_schema_meta_free(schema_meta: CassMutPtr<CassSchemaMeta>) {
    BoxFFI::free(schema_meta);
}

#[no_mangle]
pub unsafe extern "C" fn cass_schema_meta_keyspace_by_name(
    schema_meta: CassConstPtr<CassSchemaMeta>,
    keyspace_name: *const c_char,
) -> CassConstPtr<CassKeyspaceMeta> {
    cass_schema_meta_keyspace_by_name_n(schema_meta, keyspace_name, strlen(keyspace_name))
}

#[no_mangle]
pub unsafe extern "C" fn cass_schema_meta_keyspace_by_name_n(
    schema_meta: CassConstPtr<CassSchemaMeta>,
    keyspace_name: *const c_char,
    keyspace_name_length: size_t,
) -> CassConstPtr<CassKeyspaceMeta> {
    if keyspace_name.is_null() {
        return CassConstPtr::null();
    }

    let metadata = BoxFFI::as_ref(schema_meta).unwrap();
    let keyspace = ptr_to_cstr_n(keyspace_name, keyspace_name_length).unwrap();

    let keyspace_meta = metadata.keyspaces.get(keyspace);

    match keyspace_meta {
        Some(meta) => RefFFI::as_ptr(meta),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_keyspace_meta_name(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    name: *mut *const c_char,
    name_length: *mut size_t,
) {
    let keyspace_meta = RefFFI::as_ref(keyspace_meta).unwrap();
    write_str_to_c(keyspace_meta.name.as_str(), name, name_length)
}

#[no_mangle]
pub unsafe extern "C" fn cass_keyspace_meta_user_type_by_name(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    type_: *const c_char,
) -> CassConstPtr<CassDataType> {
    cass_keyspace_meta_user_type_by_name_n(keyspace_meta, type_, strlen(type_))
}

#[no_mangle]
pub unsafe extern "C" fn cass_keyspace_meta_user_type_by_name_n(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    type_: *const c_char,
    type_length: size_t,
) -> CassConstPtr<CassDataType> {
    if type_.is_null() {
        return CassConstPtr::null();
    }

    let keyspace_meta = RefFFI::as_ref(keyspace_meta).unwrap();
    let user_type_name = ptr_to_cstr_n(type_, type_length).unwrap();

    match keyspace_meta
        .user_defined_type_data_type
        .get(user_type_name)
    {
        Some(udt) => ArcFFI::as_ptr(udt),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_keyspace_meta_table_by_name(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    table: *const c_char,
) -> CassConstPtr<CassTableMeta> {
    cass_keyspace_meta_table_by_name_n(keyspace_meta, table, strlen(table))
}

#[no_mangle]
pub unsafe extern "C" fn cass_keyspace_meta_table_by_name_n(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    table: *const c_char,
    table_length: size_t,
) -> CassConstPtr<CassTableMeta> {
    if table.is_null() {
        return CassConstPtr::null();
    }

    let keyspace_meta = RefFFI::as_ref(keyspace_meta).unwrap();
    let table_name = ptr_to_cstr_n(table, table_length).unwrap();

    let table_meta = keyspace_meta.tables.get(table_name);

    match table_meta {
        Some(meta) => RefFFI::as_ptr(meta.as_ref()) as CassConstPtr<CassTableMeta>,
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_name(
    table_meta: CassConstPtr<CassTableMeta>,
    name: *mut *const c_char,
    name_length: *mut size_t,
) {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();
    write_str_to_c(table_meta.name.as_str(), name, name_length)
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_column_count(
    table_meta: CassConstPtr<CassTableMeta>,
) -> size_t {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();
    table_meta.columns_metadata.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_partition_key(
    table_meta: CassConstPtr<CassTableMeta>,
    index: size_t,
) -> CassConstPtr<CassColumnMeta> {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();

    match table_meta.partition_keys.get(index as usize) {
        Some(column_name) => match table_meta.columns_metadata.get(column_name) {
            Some(column_meta) => RefFFI::as_ptr(column_meta),
            None => CassConstPtr::null(),
        },
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_partition_key_count(
    table_meta: CassConstPtr<CassTableMeta>,
) -> size_t {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();
    table_meta.partition_keys.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_clustering_key(
    table_meta: CassConstPtr<CassTableMeta>,
    index: size_t,
) -> CassConstPtr<CassColumnMeta> {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();

    match table_meta.clustering_keys.get(index as usize) {
        Some(column_name) => match table_meta.columns_metadata.get(column_name) {
            Some(column_meta) => RefFFI::as_ptr(column_meta),
            None => CassConstPtr::null(),
        },
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_clustering_key_count(
    table_meta: CassConstPtr<CassTableMeta>,
) -> size_t {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();
    table_meta.clustering_keys.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_column_by_name(
    table_meta: CassConstPtr<CassTableMeta>,
    column: *const c_char,
) -> CassConstPtr<CassColumnMeta> {
    cass_table_meta_column_by_name_n(table_meta, column, strlen(column))
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_column_by_name_n(
    table_meta: CassConstPtr<CassTableMeta>,
    column: *const c_char,
    column_length: size_t,
) -> CassConstPtr<CassColumnMeta> {
    if column.is_null() {
        return CassConstPtr::null();
    }

    let table_meta = RefFFI::as_ref(table_meta).unwrap();
    let column_name = ptr_to_cstr_n(column, column_length).unwrap();

    match table_meta.columns_metadata.get(column_name) {
        Some(column_meta) => RefFFI::as_ptr(column_meta),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_column_meta_name(
    column_meta: CassConstPtr<CassColumnMeta>,
    name: *mut *const c_char,
    name_length: *mut size_t,
) {
    let column_meta = RefFFI::as_ref(column_meta).unwrap();
    write_str_to_c(column_meta.name.as_str(), name, name_length)
}

#[no_mangle]
pub unsafe extern "C" fn cass_column_meta_data_type(
    column_meta: CassConstPtr<CassColumnMeta>,
) -> CassConstPtr<CassDataType> {
    let column_meta = RefFFI::as_ref(column_meta).unwrap();

    &column_meta.column_type as CassConstPtr<CassDataType>
}

#[no_mangle]
pub unsafe extern "C" fn cass_column_meta_type(
    column_meta: CassConstPtr<CassColumnMeta>,
) -> CassColumnType {
    let column_meta = RefFFI::as_ref(column_meta).unwrap();
    column_meta.column_kind
}

#[no_mangle]
pub unsafe extern "C" fn cass_keyspace_meta_materialized_view_by_name(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    view: *const c_char,
) -> CassConstPtr<CassMaterializedViewMeta> {
    cass_keyspace_meta_materialized_view_by_name_n(keyspace_meta, view, strlen(view))
}

#[no_mangle]
pub unsafe extern "C" fn cass_keyspace_meta_materialized_view_by_name_n(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    view: *const c_char,
    view_length: size_t,
) -> CassConstPtr<CassMaterializedViewMeta> {
    if view.is_null() {
        return CassConstPtr::null();
    }

    let keyspace_meta = RefFFI::as_ref(keyspace_meta).unwrap();
    let view_name = ptr_to_cstr_n(view, view_length).unwrap();

    match keyspace_meta.views.get(view_name) {
        Some(view_meta) => RefFFI::as_ptr(view_meta.as_ref()),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_materialized_view_by_name(
    table_meta: CassConstPtr<CassTableMeta>,
    view: *const c_char,
) -> CassConstPtr<CassMaterializedViewMeta> {
    cass_table_meta_materialized_view_by_name_n(table_meta, view, strlen(view))
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_materialized_view_by_name_n(
    table_meta: CassConstPtr<CassTableMeta>,
    view: *const c_char,
    view_length: size_t,
) -> CassConstPtr<CassMaterializedViewMeta> {
    if view.is_null() {
        return CassConstPtr::null();
    }

    let table_meta = RefFFI::as_ref(table_meta).unwrap();
    let view_name = ptr_to_cstr_n(view, view_length).unwrap();

    match table_meta.views.get(view_name) {
        Some(view_meta) => RefFFI::as_ptr(view_meta.as_ref()),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_materialized_view_count(
    table_meta: CassConstPtr<CassTableMeta>,
) -> size_t {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();
    table_meta.views.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_table_meta_materialized_view(
    table_meta: CassConstPtr<CassTableMeta>,
    index: size_t,
) -> CassConstPtr<CassMaterializedViewMeta> {
    let table_meta = RefFFI::as_ref(table_meta).unwrap();

    match table_meta.views.iter().nth(index as usize) {
        Some(view_meta) => RefFFI::as_ptr(view_meta.1.as_ref()),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_column_by_name(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
    column: *const c_char,
) -> CassConstPtr<CassColumnMeta> {
    cass_materialized_view_meta_column_by_name_n(view_meta, column, strlen(column))
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_column_by_name_n(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
    column: *const c_char,
    column_length: size_t,
) -> CassConstPtr<CassColumnMeta> {
    if column.is_null() {
        return CassConstPtr::null();
    }

    let view_meta = RefFFI::as_ref(view_meta).unwrap();
    let column_name = ptr_to_cstr_n(column, column_length).unwrap();

    match view_meta.view_metadata.columns_metadata.get(column_name) {
        Some(column_meta) => RefFFI::as_ptr(column_meta),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_name(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
    name: *mut *const c_char,
    name_length: *mut size_t,
) {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();
    write_str_to_c(view_meta.name.as_str(), name, name_length)
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_base_table(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
) -> CassConstPtr<CassTableMeta> {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();
    RefFFI::as_ptr(view_meta.base_table.upgrade().unwrap().as_ref())
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_column_count(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
) -> size_t {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();
    view_meta.view_metadata.columns_metadata.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_column(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
    index: size_t,
) -> CassConstPtr<CassColumnMeta> {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();

    match view_meta
        .view_metadata
        .columns_metadata
        .iter()
        .nth(index as usize)
    {
        Some(column_entry) => RefFFI::as_ptr(column_entry.1),
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_partition_key_count(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
) -> size_t {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();
    view_meta.view_metadata.partition_keys.len() as size_t
}

pub unsafe extern "C" fn cass_materialized_view_meta_partition_key(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
    index: size_t,
) -> CassConstPtr<CassColumnMeta> {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();

    match view_meta.view_metadata.partition_keys.get(index as usize) {
        Some(column_name) => match view_meta.view_metadata.columns_metadata.get(column_name) {
            Some(column_meta) => RefFFI::as_ptr(column_meta),
            None => CassConstPtr::null(),
        },
        None => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_materialized_view_meta_clustering_key_count(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
) -> size_t {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();
    view_meta.view_metadata.clustering_keys.len() as size_t
}

pub unsafe extern "C" fn cass_materialized_view_meta_clustering_key(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
    index: size_t,
) -> CassConstPtr<CassColumnMeta> {
    let view_meta = RefFFI::as_ref(view_meta).unwrap();

    match view_meta.view_metadata.clustering_keys.get(index as usize) {
        Some(column_name) => match view_meta.view_metadata.columns_metadata.get(column_name) {
            Some(column_meta) => RefFFI::as_ptr(column_meta),
            None => CassConstPtr::null(),
        },
        None => CassConstPtr::null(),
    }
}
