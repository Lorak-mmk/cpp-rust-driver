use crate::argconv::*;
use crate::cass_error::CassError;
use crate::cass_types::{cass_data_type_type, CassDataType, CassDataTypeInner, CassValueType};
use crate::inet::CassInet;
use crate::metadata::{
    CassColumnMeta, CassKeyspaceMeta, CassMaterializedViewMeta, CassSchemaMeta, CassTableMeta,
};
use crate::statement::CassStatement;
use crate::types::*;
use crate::uuid::CassUuid;
use scylla::frame::response::result::{ColumnSpec, CqlValue};
use scylla::{BufMut, Bytes, BytesMut};
use std::convert::TryInto;
use std::os::raw::c_char;
use std::slice;
use std::sync::Arc;
use uuid::Uuid;

pub struct CassResult {
    pub rows: Option<Vec<CassRow>>,
    pub metadata: Arc<CassResultData>,
}

impl ArcFFI for CassResult {}

pub struct CassResultData {
    pub paging_state: Option<Bytes>,
    pub col_specs: Vec<ColumnSpec>,
    pub tracing_id: Option<Uuid>,
}

/// The lifetime of CassRow is bound to CassResult.
/// It will be freed, when CassResult is freed.(see #[cass_result_free])
pub struct CassRow {
    pub columns: Vec<CassValue>,
    pub result_metadata: Arc<CassResultData>,
}

impl RefFFI for CassRow {}

pub enum Value {
    RegularValue(CqlValue),
    CollectionValue(Collection),
}

pub enum Collection {
    List(Vec<CassValue>),
    Map(Vec<(CassValue, CassValue)>),
    Set(Vec<CassValue>),
    UserDefinedType {
        keyspace: String,
        type_name: String,
        fields: Vec<(String, Option<CassValue>)>,
    },
    Tuple(Vec<Option<CassValue>>),
}

pub struct CassValue {
    pub value: Option<Value>,
    pub value_type: Arc<CassDataType>,
}

impl RefFFI for CassValue {}

pub struct CassResultIterator {
    result: Arc<CassResult>,
    position: Option<usize>,
}

pub struct CassRowIterator {
    row: &'static CassRow,
    position: Option<usize>,
}

pub struct CassCollectionIterator {
    value: &'static CassValue,
    count: u64,
    position: Option<usize>,
}

pub struct CassMapIterator {
    value: &'static CassValue,
    count: u64,
    position: Option<usize>,
}

pub struct CassUdtIterator {
    value: &'static CassValue,
    count: u64,
    position: Option<usize>,
}

pub struct CassSchemaMetaIterator {
    value: &'static CassSchemaMeta,
    count: usize,
    position: Option<usize>,
}

pub struct CassKeyspaceMetaIterator {
    value: &'static CassKeyspaceMeta,
    count: usize,
    position: Option<usize>,
}

pub struct CassTableMetaIterator {
    value: &'static CassTableMeta,
    count: usize,
    position: Option<usize>,
}

pub struct CassViewMetaIterator {
    value: &'static CassMaterializedViewMeta,
    count: usize,
    position: Option<usize>,
}

pub enum CassIterator {
    CassResultIterator(CassResultIterator),
    CassRowIterator(CassRowIterator),
    CassCollectionIterator(CassCollectionIterator),
    CassMapIterator(CassMapIterator),
    CassUdtIterator(CassUdtIterator),
    CassSchemaMetaIterator(CassSchemaMetaIterator),
    CassKeyspaceMetaTableIterator(CassKeyspaceMetaIterator),
    CassKeyspaceMetaUserTypeIterator(CassKeyspaceMetaIterator),
    CassKeyspaceMetaViewIterator(CassKeyspaceMetaIterator),
    CassTableMetaIterator(CassTableMetaIterator),
    CassViewMetaIterator(CassViewMetaIterator),
}

impl BoxFFI for CassIterator {}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_free(iterator: CassMutPtr<CassIterator>) {
    BoxFFI::free(iterator);
}

// After creating an iterator we have to call next() before accessing the value
#[no_mangle]
pub unsafe extern "C" fn cass_iterator_next(iterator: CassMutPtr<CassIterator>) -> cass_bool_t {
    let mut iter = BoxFFI::as_mut_ref(iterator).unwrap();

    match &mut iter {
        CassIterator::CassResultIterator(result_iterator) => {
            let new_pos: usize = result_iterator.position.map_or(0, |prev_pos| prev_pos + 1);

            result_iterator.position = Some(new_pos);

            match &result_iterator.result.rows {
                Some(rs) => (new_pos < rs.len()) as cass_bool_t,
                None => false as cass_bool_t,
            }
        }
        CassIterator::CassRowIterator(row_iterator) => {
            let new_pos: usize = row_iterator.position.map_or(0, |prev_pos| prev_pos + 1);

            row_iterator.position = Some(new_pos);

            (new_pos < row_iterator.row.columns.len()) as cass_bool_t
        }
        CassIterator::CassCollectionIterator(collection_iterator) => {
            let new_pos: usize = collection_iterator
                .position
                .map_or(0, |prev_pos| prev_pos + 1);

            collection_iterator.position = Some(new_pos);

            (new_pos < collection_iterator.count.try_into().unwrap()) as cass_bool_t
        }
        CassIterator::CassMapIterator(map_iterator) => {
            let new_pos: usize = map_iterator.position.map_or(0, |prev_pos| prev_pos + 1);

            map_iterator.position = Some(new_pos);

            (new_pos < map_iterator.count.try_into().unwrap()) as cass_bool_t
        }
        CassIterator::CassUdtIterator(udt_iterator) => {
            let new_pos: usize = udt_iterator.position.map_or(0, |prev_pos| prev_pos + 1);

            udt_iterator.position = Some(new_pos);

            (new_pos < udt_iterator.count.try_into().unwrap()) as cass_bool_t
        }
        CassIterator::CassSchemaMetaIterator(schema_meta_iterator) => {
            let new_pos: usize = schema_meta_iterator
                .position
                .map_or(0, |prev_pos| prev_pos + 1);

            schema_meta_iterator.position = Some(new_pos);

            (new_pos < schema_meta_iterator.count) as cass_bool_t
        }
        CassIterator::CassKeyspaceMetaTableIterator(keyspace_meta_iterator) => {
            let new_pos: usize = keyspace_meta_iterator
                .position
                .map_or(0, |prev_pos| prev_pos + 1);

            keyspace_meta_iterator.position = Some(new_pos);

            (new_pos < keyspace_meta_iterator.count) as cass_bool_t
        }
        CassIterator::CassKeyspaceMetaUserTypeIterator(keyspace_meta_iterator) => {
            let new_pos: usize = keyspace_meta_iterator
                .position
                .map_or(0, |prev_pos| prev_pos + 1);

            keyspace_meta_iterator.position = Some(new_pos);

            (new_pos < keyspace_meta_iterator.count) as cass_bool_t
        }
        CassIterator::CassKeyspaceMetaViewIterator(keyspace_meta_iterator) => {
            let new_pos: usize = keyspace_meta_iterator
                .position
                .map_or(0, |prev_pos| prev_pos + 1);

            keyspace_meta_iterator.position = Some(new_pos);

            (new_pos < keyspace_meta_iterator.count) as cass_bool_t
        }
        CassIterator::CassTableMetaIterator(table_iterator) => {
            let new_pos: usize = table_iterator.position.map_or(0, |prev_pos| prev_pos + 1);

            table_iterator.position = Some(new_pos);

            (new_pos < table_iterator.count) as cass_bool_t
        }
        CassIterator::CassViewMetaIterator(view_iterator) => {
            let new_pos: usize = view_iterator.position.map_or(0, |prev_pos| prev_pos + 1);

            view_iterator.position = Some(new_pos);

            (new_pos < view_iterator.count) as cass_bool_t
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_row(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassRow> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    // Defined only for result iterator, for other types should return null
    if let CassIterator::CassResultIterator(result_iterator) = iter {
        let iter_position = match result_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let row: &CassRow = match result_iterator
            .result
            .rows
            .as_ref()
            .and_then(|rs| rs.get(iter_position))
        {
            Some(row) => row,
            None => return CassConstPtr::null(),
        };

        return RefFFI::as_ptr(row);
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_column(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassValue> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    // Defined only for row iterator, for other types should return null
    if let CassIterator::CassRowIterator(row_iterator) = iter {
        let iter_position = match row_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let value = match row_iterator.row.columns.get(iter_position) {
            Some(col) => col,
            None => return CassConstPtr::null(),
        };

        return RefFFI::as_ptr(value);
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_value(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassValue> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    // Defined only for collections(list and set) or tuple iterator, for other types should return null
    if let CassIterator::CassCollectionIterator(collection_iterator) = iter {
        let iter_position = match collection_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let value = match &collection_iterator.value.value {
            Some(Value::CollectionValue(Collection::List(list))) => list.get(iter_position),
            Some(Value::CollectionValue(Collection::Set(set))) => set.get(iter_position),
            Some(Value::CollectionValue(Collection::Tuple(tuple))) => {
                tuple.get(iter_position).and_then(|x| x.as_ref())
            }
            _ => return CassConstPtr::null(),
        };

        return match value {
            Some(v) => RefFFI::as_ptr(v),
            None => CassConstPtr::null(),
        };
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_map_key(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassValue> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    if let CassIterator::CassMapIterator(map_iterator) = iter {
        let iter_position = match map_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let entry = match &map_iterator.value.value {
            Some(Value::CollectionValue(Collection::Map(map))) => map.get(iter_position),
            _ => return CassConstPtr::null(),
        };

        return match entry {
            Some(e) => RefFFI::as_ptr(&e.0),
            None => CassConstPtr::null(),
        };
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_map_value(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassValue> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    if let CassIterator::CassMapIterator(map_iterator) = iter {
        let iter_position = match map_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let entry = match &map_iterator.value.value {
            Some(Value::CollectionValue(Collection::Map(map))) => map.get(iter_position),
            _ => return CassConstPtr::null(),
        };

        if entry.is_none() {
            return CassConstPtr::null();
        }

        return RefFFI::as_ptr(&entry.unwrap().1);
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_user_type_field_name(
    iterator: CassConstPtr<CassIterator>,
    name: *mut *const c_char,
    name_length: *mut size_t,
) -> CassError {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    if let CassIterator::CassUdtIterator(udt_iterator) = iter {
        let iter_position = match udt_iterator.position {
            Some(pos) => pos,
            None => return CassError::CASS_ERROR_LIB_BAD_PARAMS,
        };

        let udt_entry_opt = match &udt_iterator.value.value {
            Some(Value::CollectionValue(Collection::UserDefinedType { fields, .. })) => {
                fields.get(iter_position)
            }
            _ => return CassError::CASS_ERROR_LIB_BAD_PARAMS,
        };

        match udt_entry_opt {
            Some(udt_entry) => {
                let field_name = &udt_entry.0;
                write_str_to_c(field_name.as_str(), name, name_length);
            }
            None => return CassError::CASS_ERROR_LIB_BAD_PARAMS,
        }

        return CassError::CASS_OK;
    }

    CassError::CASS_ERROR_LIB_BAD_PARAMS
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_user_type_field_value(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassValue> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    if let CassIterator::CassUdtIterator(udt_iterator) = iter {
        let iter_position = match udt_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let udt_entry_opt = match &udt_iterator.value.value {
            Some(Value::CollectionValue(Collection::UserDefinedType { fields, .. })) => {
                fields.get(iter_position)
            }
            _ => return CassConstPtr::null(),
        };

        return match udt_entry_opt {
            Some(udt_entry) => match &udt_entry.1 {
                Some(value) => RefFFI::as_ptr(value),
                None => CassConstPtr::null(),
            },
            None => CassConstPtr::null(),
        };
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_keyspace_meta(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassKeyspaceMeta> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    if let CassIterator::CassSchemaMetaIterator(schema_meta_iterator) = iter {
        let iter_position = match schema_meta_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let schema_meta_entry_opt = &schema_meta_iterator
            .value
            .keyspaces
            .iter()
            .nth(iter_position);

        return match schema_meta_entry_opt {
            Some(schema_meta_entry) => RefFFI::as_ptr(schema_meta_entry.1),
            None => CassConstPtr::null(),
        };
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_table_meta(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassTableMeta> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    if let CassIterator::CassKeyspaceMetaTableIterator(keyspace_meta_iterator) = iter {
        let iter_position = match keyspace_meta_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let table_meta_entry_opt = keyspace_meta_iterator
            .value
            .tables
            .iter()
            .nth(iter_position);

        return match table_meta_entry_opt {
            Some(table_meta_entry) => RefFFI::as_ptr(table_meta_entry.1.as_ref()),
            None => CassConstPtr::null(),
        };
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_user_type(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassDataType> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    if let CassIterator::CassKeyspaceMetaUserTypeIterator(keyspace_meta_iterator) = iter {
        let iter_position = match keyspace_meta_iterator.position {
            Some(pos) => pos,
            None => return CassConstPtr::null(),
        };

        let udt_to_type_entry_opt = keyspace_meta_iterator
            .value
            .user_defined_type_data_type
            .iter()
            .nth(iter_position);

        return match udt_to_type_entry_opt {
            Some(udt_to_type_entry) => ArcFFI::as_ptr(udt_to_type_entry.1),
            None => CassConstPtr::null(),
        };
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_column_meta(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassColumnMeta> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    match iter {
        CassIterator::CassTableMetaIterator(table_meta_iterator) => {
            let iter_position = match table_meta_iterator.position {
                Some(pos) => pos,
                None => return CassConstPtr::null(),
            };

            let column_meta_entry_opt = table_meta_iterator
                .value
                .columns_metadata
                .iter()
                .nth(iter_position);

            match column_meta_entry_opt {
                Some(column_meta_entry) => RefFFI::as_ptr(column_meta_entry.1),
                None => CassConstPtr::null(),
            }
        }
        CassIterator::CassViewMetaIterator(view_meta_iterator) => {
            let iter_position = match view_meta_iterator.position {
                Some(pos) => pos,
                None => return CassConstPtr::null(),
            };

            let column_meta_entry_opt = view_meta_iterator
                .value
                .view_metadata
                .columns_metadata
                .iter()
                .nth(iter_position);

            match column_meta_entry_opt {
                Some(column_meta_entry) => RefFFI::as_ptr(column_meta_entry.1),
                None => CassConstPtr::null(),
            }
        }
        _ => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_get_materialized_view_meta(
    iterator: CassConstPtr<CassIterator>,
) -> CassConstPtr<CassMaterializedViewMeta> {
    let iter = BoxFFI::as_ref(iterator).unwrap();

    match iter {
        CassIterator::CassKeyspaceMetaViewIterator(keyspace_meta_iterator) => {
            let iter_position = match keyspace_meta_iterator.position {
                Some(pos) => pos,
                None => return CassConstPtr::null(),
            };

            let view_meta_entry_opt = keyspace_meta_iterator.value.views.iter().nth(iter_position);

            match view_meta_entry_opt {
                Some(view_meta_entry) => RefFFI::as_ptr(view_meta_entry.1.as_ref()),
                None => CassConstPtr::null(),
            }
        }
        CassIterator::CassTableMetaIterator(table_meta_iterator) => {
            let iter_position = match table_meta_iterator.position {
                Some(pos) => pos,
                None => return CassConstPtr::null(),
            };

            let view_meta_entry_opt = table_meta_iterator.value.views.iter().nth(iter_position);

            match view_meta_entry_opt {
                Some(view_meta_entry) => RefFFI::as_ptr(view_meta_entry.1.as_ref()),
                None => CassConstPtr::null(),
            }
        }
        _ => CassConstPtr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_from_result(
    result: CassConstPtr<CassResult>,
) -> CassMutPtr<CassIterator> {
    let result_from_raw = ArcFFI::cloned_from_ptr(result).unwrap();

    let iterator = CassResultIterator {
        result: result_from_raw,
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassResultIterator(iterator)))
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_from_row(
    row: CassConstPtr<CassRow>,
) -> CassMutPtr<CassIterator> {
    let row_from_raw = RefFFI::as_ref(row).unwrap();

    let iterator = CassRowIterator {
        row: row_from_raw,
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassRowIterator(iterator)))
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_from_collection(
    value: CassConstPtr<CassValue>,
) -> CassMutPtr<CassIterator> {
    if value.is_null() || cass_value_is_collection(value) == 0 {
        return CassMutPtr::null_mut();
    }

    let map_iterator = cass_iterator_from_map(value);
    if !map_iterator.is_null() {
        return map_iterator;
    }

    let val = RefFFI::as_ref(value).unwrap();
    let item_count = cass_value_item_count(value);

    let iterator = CassCollectionIterator {
        value: val,
        count: item_count,
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassCollectionIterator(iterator)))
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_from_tuple(
    value: CassConstPtr<CassValue>,
) -> CassMutPtr<CassIterator> {
    let tuple = RefFFI::as_ref(value).unwrap();

    if let Some(Value::CollectionValue(Collection::Tuple(val))) = &tuple.value {
        let item_count = val.len();
        let iterator = CassCollectionIterator {
            value: tuple,
            count: item_count as u64,
            position: None,
        };

        return BoxFFI::into_ptr(Box::new(CassIterator::CassCollectionIterator(iterator)));
    }

    CassMutPtr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_from_map(
    value: CassConstPtr<CassValue>,
) -> CassMutPtr<CassIterator> {
    let map = RefFFI::as_ref(value).unwrap();

    if let Some(Value::CollectionValue(Collection::Map(val))) = &map.value {
        let item_count = val.len();
        let iterator = CassMapIterator {
            value: map,
            count: item_count as u64,
            position: None,
        };

        return BoxFFI::into_ptr(Box::new(CassIterator::CassMapIterator(iterator)));
    }

    CassMutPtr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_fields_from_user_type(
    value: CassConstPtr<CassValue>,
) -> CassMutPtr<CassIterator> {
    let udt = RefFFI::as_ref(value).unwrap();

    if let Some(Value::CollectionValue(Collection::UserDefinedType { fields, .. })) = &udt.value {
        let item_count = fields.len();
        let iterator = CassUdtIterator {
            value: udt,
            count: item_count as u64,
            position: None,
        };

        return BoxFFI::into_ptr(Box::new(CassIterator::CassUdtIterator(iterator)));
    }

    CassMutPtr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_keyspaces_from_schema_meta(
    schema_meta: CassConstPtr<CassSchemaMeta>,
) -> CassMutPtr<CassIterator> {
    let metadata = BoxFFI::as_ref(schema_meta).unwrap();

    let iterator = CassSchemaMetaIterator {
        value: metadata,
        count: metadata.keyspaces.len(),
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassSchemaMetaIterator(iterator)))
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_tables_from_keyspace_meta(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
) -> CassMutPtr<CassIterator> {
    let metadata = RefFFI::as_ref(keyspace_meta).unwrap();

    let iterator = CassKeyspaceMetaIterator {
        value: metadata,
        count: metadata.tables.len(),
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassKeyspaceMetaTableIterator(
        iterator,
    )))
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_materialized_views_from_keyspace_meta(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
) -> CassMutPtr<CassIterator> {
    let metadata = RefFFI::as_ref(keyspace_meta).unwrap();

    let iterator = CassKeyspaceMetaIterator {
        value: metadata,
        count: metadata.views.len(),
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassKeyspaceMetaViewIterator(
        iterator,
    )))
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_user_types_from_keyspace_meta(
    keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
) -> CassMutPtr<CassIterator> {
    let metadata = RefFFI::as_ref(keyspace_meta).unwrap();

    let iterator = CassKeyspaceMetaIterator {
        value: metadata,
        count: metadata.user_defined_type_data_type.len(),
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassKeyspaceMetaUserTypeIterator(
        iterator,
    )))
}

#[no_mangle]
pub unsafe extern "C" fn cass_iterator_columns_from_table_meta(
    table_meta: CassConstPtr<CassTableMeta>,
) -> CassMutPtr<CassIterator> {
    let metadata = RefFFI::as_ref(table_meta).unwrap();

    let iterator = CassTableMetaIterator {
        value: metadata,
        count: metadata.columns_metadata.len(),
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassTableMetaIterator(iterator)))
}

pub unsafe extern "C" fn cass_iterator_materialized_views_from_table_meta(
    table_meta: CassConstPtr<CassTableMeta>,
) -> CassMutPtr<CassIterator> {
    let metadata = RefFFI::as_ref(table_meta).unwrap();

    let iterator = CassTableMetaIterator {
        value: metadata,
        count: metadata.views.len(),
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassTableMetaIterator(iterator)))
}

pub unsafe extern "C" fn cass_iterator_columns_from_materialized_view_meta(
    view_meta: CassConstPtr<CassMaterializedViewMeta>,
) -> CassMutPtr<CassIterator> {
    let metadata = RefFFI::as_ref(view_meta).unwrap();

    let iterator = CassViewMetaIterator {
        value: metadata,
        count: metadata.view_metadata.columns_metadata.len(),
        position: None,
    };

    BoxFFI::into_ptr(Box::new(CassIterator::CassViewMetaIterator(iterator)))
}

#[no_mangle]
pub unsafe extern "C" fn cass_result_free(result_raw: CassConstPtr<CassResult>) {
    ArcFFI::free(result_raw);
}

#[no_mangle]
pub unsafe extern "C" fn cass_result_has_more_pages(
    result: CassConstPtr<CassResult>,
) -> cass_bool_t {
    let result = ArcFFI::as_ref(result).unwrap();
    result.metadata.paging_state.is_some() as cass_bool_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_row_get_column(
    row_raw: CassConstPtr<CassRow>,
    index: size_t,
) -> CassConstPtr<CassValue> {
    let row = RefFFI::as_ref(row_raw).unwrap();

    let index_usize: usize = index.try_into().unwrap();
    let column_value = match row.columns.get(index_usize) {
        Some(val) => val,
        None => return CassConstPtr::null(),
    };

    RefFFI::as_ptr(column_value)
}

#[no_mangle]
pub unsafe extern "C" fn cass_row_get_column_by_name(
    row: CassConstPtr<CassRow>,
    name: *const c_char,
) -> CassConstPtr<CassValue> {
    let name_str = ptr_to_cstr(name).unwrap();
    let name_length = name_str.len();

    cass_row_get_column_by_name_n(row, name, name_length as size_t)
}

#[no_mangle]
pub unsafe extern "C" fn cass_row_get_column_by_name_n(
    row: CassConstPtr<CassRow>,
    name: *const c_char,
    name_length: size_t,
) -> CassConstPtr<CassValue> {
    let row_from_raw = RefFFI::as_ref(row).unwrap();
    let mut name_str = ptr_to_cstr_n(name, name_length).unwrap();
    let mut is_case_sensitive = false;

    if name_str.starts_with('\"') && name_str.ends_with('\"') {
        name_str = name_str.strip_prefix('\"').unwrap();
        name_str = name_str.strip_suffix('\"').unwrap();
        is_case_sensitive = true;
    }

    return row_from_raw
        .result_metadata
        .col_specs
        .iter()
        .enumerate()
        .find(|(_, spec)| {
            is_case_sensitive && spec.name == name_str
                || !is_case_sensitive && spec.name.eq_ignore_ascii_case(name_str)
        })
        .map(|(index, _)| {
            return match row_from_raw.columns.get(index) {
                Some(value) => RefFFI::as_ptr(value),
                None => CassConstPtr::null(),
            };
        })
        .unwrap_or(CassConstPtr::null());
}

#[no_mangle]
pub unsafe extern "C" fn cass_result_column_name(
    result: CassConstPtr<CassResult>,
    index: size_t,
    name: *mut *const c_char,
    name_length: *mut size_t,
) -> CassError {
    let result_from_raw = ArcFFI::as_ref(result).unwrap();
    let index_usize: usize = index.try_into().unwrap();

    if index_usize >= result_from_raw.metadata.col_specs.len() {
        return CassError::CASS_ERROR_LIB_INDEX_OUT_OF_BOUNDS;
    }

    let column_spec: &ColumnSpec = result_from_raw.metadata.col_specs.get(index_usize).unwrap();
    let column_name = column_spec.name.as_str();

    write_str_to_c(column_name, name, name_length);

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_type(value: CassConstPtr<CassValue>) -> CassValueType {
    let value_from_raw = RefFFI::as_ref(value).unwrap();

    cass_data_type_type(ArcFFI::as_ptr(&value_from_raw.value_type))
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_data_type(
    value: CassConstPtr<CassValue>,
) -> CassConstPtr<CassDataType> {
    let value_from_raw = RefFFI::as_ref(value).unwrap();

    ArcFFI::as_ptr(&value_from_raw.value_type)
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_float(
    value: CassConstPtr<CassValue>,
    output: *mut cass_float_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::Float(f))) => std::ptr::write(output, f),
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_double(
    value: CassConstPtr<CassValue>,
    output: *mut cass_double_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::Double(d))) => std::ptr::write(output, d),
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_bool(
    value: CassConstPtr<CassValue>,
    output: *mut cass_bool_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::Boolean(b))) => {
            std::ptr::write(output, b as cass_bool_t)
        }
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_int8(
    value: CassConstPtr<CassValue>,
    output: *mut cass_int8_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::TinyInt(i))) => std::ptr::write(output, i),
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_int16(
    value: CassConstPtr<CassValue>,
    output: *mut cass_int16_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::SmallInt(i))) => std::ptr::write(output, i),
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_uint32(
    value: CassConstPtr<CassValue>,
    output: *mut cass_uint32_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::Date(u))) => std::ptr::write(output, u), // FIXME: hack
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_int32(
    value: CassConstPtr<CassValue>,
    output: *mut cass_int32_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::Int(i))) => std::ptr::write(output, i),
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_int64(
    value: CassConstPtr<CassValue>,
    output: *mut cass_int64_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::BigInt(i))) => std::ptr::write(output, i),
        Some(Value::RegularValue(CqlValue::Counter(i))) => {
            std::ptr::write(output, i.0 as cass_int64_t)
        }
        Some(Value::RegularValue(CqlValue::Time(d))) => match d.num_nanoseconds() {
            Some(nanos) => std::ptr::write(output, nanos as cass_int64_t),
            None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
        },
        Some(Value::RegularValue(CqlValue::Timestamp(d))) => {
            std::ptr::write(output, d.num_milliseconds() as cass_int64_t)
        }
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_uuid(
    value: CassConstPtr<CassValue>,
    output: *mut CassUuid,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::Uuid(uuid))) => std::ptr::write(output, uuid.into()),
        Some(Value::RegularValue(CqlValue::Timeuuid(uuid))) => std::ptr::write(output, uuid.into()),
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_inet(
    value: CassConstPtr<CassValue>,
    output: *mut CassInet,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match val.value {
        Some(Value::RegularValue(CqlValue::Inet(inet))) => std::ptr::write(output, inet.into()),
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_string(
    value: CassConstPtr<CassValue>,
    output: *mut *const c_char,
    output_size: *mut size_t,
) -> CassError {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    match &val.value {
        // It seems that cpp driver doesn't check the type - you can call _get_string
        // on any type and get internal represenation. I don't see how to do it easily in
        // a compatible way in rust, so let's do something sensible - only return result
        // for string values.
        Some(Value::RegularValue(CqlValue::Ascii(s))) => {
            write_str_to_c(s.as_str(), output, output_size)
        }
        Some(Value::RegularValue(CqlValue::Text(s))) => {
            write_str_to_c(s.as_str(), output, output_size)
        }
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    }

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_get_bytes(
    value: CassConstPtr<CassValue>,
    output: *mut *const cass_byte_t,
    output_size: *mut size_t,
) -> CassError {
    let value_from_raw: &CassValue = match RefFFI::as_ref(value) {
        Some(v) => v,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    };

    // FIXME: This should be implemented for all CQL types
    // Note: currently rust driver does not allow to get raw bytes of the CQL value.
    match &value_from_raw.value {
        Some(Value::RegularValue(CqlValue::Blob(bytes))) => {
            *output = bytes.as_ptr() as *const cass_byte_t;
            *output_size = bytes.len() as u64;
        }
        Some(_) => return CassError::CASS_ERROR_LIB_INVALID_VALUE_TYPE,
        None => return CassError::CASS_ERROR_LIB_NULL_VALUE,
    }

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_is_null(value: CassConstPtr<CassValue>) -> cass_bool_t {
    let val: &CassValue = RefFFI::as_ref(value).unwrap();
    val.value.is_none() as cass_bool_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_is_collection(value: CassConstPtr<CassValue>) -> cass_bool_t {
    let val = RefFFI::as_ref(value).unwrap();

    match val.value {
        Some(Value::CollectionValue(Collection::List(_))) => true as cass_bool_t,
        Some(Value::CollectionValue(Collection::Set(_))) => true as cass_bool_t,
        Some(Value::CollectionValue(Collection::Map(_))) => true as cass_bool_t,
        _ => false as cass_bool_t,
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_item_count(collection: CassConstPtr<CassValue>) -> size_t {
    let val = RefFFI::as_ref(collection).unwrap();

    match &val.value {
        Some(Value::CollectionValue(Collection::List(list))) => list.len() as size_t,
        Some(Value::CollectionValue(Collection::Map(map))) => map.len() as size_t,
        Some(Value::CollectionValue(Collection::Set(set))) => set.len() as size_t,
        Some(Value::CollectionValue(Collection::Tuple(tuple))) => tuple.len() as size_t,
        Some(Value::CollectionValue(Collection::UserDefinedType { fields, .. })) => {
            fields.len() as size_t
        }
        _ => 0 as size_t,
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_primary_sub_type(
    collection: CassConstPtr<CassValue>,
) -> CassValueType {
    let val = RefFFI::as_ref(collection).unwrap();

    match val.value_type.as_ref().get_unchecked() {
        CassDataTypeInner::List(Some(list)) => list.get_unchecked().get_value_type(),
        CassDataTypeInner::Set(Some(set)) => set.get_unchecked().get_value_type(),
        CassDataTypeInner::Map(Some(key), _) => key.get_unchecked().get_value_type(),
        _ => CassValueType::CASS_VALUE_TYPE_UNKNOWN,
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_value_secondary_sub_type(
    collection: CassConstPtr<CassValue>,
) -> CassValueType {
    let val = RefFFI::as_ref(collection).unwrap();

    match val.value_type.as_ref().get_unchecked() {
        CassDataTypeInner::Map(_, Some(value)) => value.get_unchecked().get_value_type(),
        _ => CassValueType::CASS_VALUE_TYPE_UNKNOWN,
    }
}

#[no_mangle]
pub unsafe extern "C" fn cass_result_row_count(result_raw: CassConstPtr<CassResult>) -> size_t {
    let result = ArcFFI::as_ref(result_raw).unwrap();

    if result.rows.as_ref().is_none() {
        return 0;
    }

    result.rows.as_ref().unwrap().len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_result_column_count(result_raw: CassConstPtr<CassResult>) -> size_t {
    let result = ArcFFI::as_ref(result_raw).unwrap();

    result.metadata.col_specs.len() as size_t
}

#[no_mangle]
pub unsafe extern "C" fn cass_result_first_row(
    result_raw: CassConstPtr<CassResult>,
) -> CassConstPtr<CassRow> {
    let result = ArcFFI::as_ref(result_raw).unwrap();

    if result.rows.is_some() || result.rows.as_ref().unwrap().is_empty() {
        return RefFFI::as_ptr(result.rows.as_ref().unwrap().first().unwrap());
    }

    CassConstPtr::null()
}

#[no_mangle]
pub unsafe extern "C" fn cass_result_paging_state_token(
    result: CassConstPtr<CassResult>,
    paging_state: *mut *const c_char,
    paging_state_size: *mut size_t,
) -> CassError {
    if cass_result_has_more_pages(result) == cass_false {
        return CassError::CASS_ERROR_LIB_NO_PAGING_STATE;
    }

    let result_from_raw = ArcFFI::as_ref(result).unwrap();

    match &result_from_raw.metadata.paging_state {
        Some(result_paging_state) => {
            *paging_state_size = result_paging_state.len() as u64;
            *paging_state = result_paging_state.as_ptr() as *const c_char;
        }
        None => {
            *paging_state_size = 0;
            *paging_state = CassConstPtr::null();
        }
    }

    CassError::CASS_OK
}

#[no_mangle]
pub unsafe extern "C" fn cass_statement_set_paging_state_token(
    statement: CassMutPtr<CassStatement>,
    paging_state: *const c_char,
    paging_state_size: size_t,
) -> CassError {
    let statement_from_raw = BoxFFI::as_mut_ref(statement).unwrap();

    if paging_state.is_null() {
        statement_from_raw.paging_state = None;
        return CassError::CASS_ERROR_LIB_NULL_VALUE;
    }

    let paging_state_usize: usize = paging_state_size.try_into().unwrap();
    let mut b = BytesMut::with_capacity(paging_state_usize + 1);
    b.put_slice(slice::from_raw_parts(
        paging_state as *const u8,
        paging_state_usize,
    ));
    b.extend_from_slice(b"\0");
    statement_from_raw.paging_state = Some(b.freeze());

    CassError::CASS_OK
}

// CassResult functions:
/*
extern "C" {
    pub fn cass_statement_set_paging_state(
        statement: CassMutPtr<CassStatement>,
        result: CassConstPtr<CassResult>,
    ) -> CassError;
}
extern "C" {
    pub fn cass_result_row_count(result: CassConstPtr<CassResult>) -> size_t;
}
extern "C" {
    pub fn cass_result_column_count(result: CassConstPtr<CassResult>) -> size_t;
}
extern "C" {
    pub fn cass_result_column_name(
        result: CassConstPtr<CassResult>,
        index: size_t,
        name: *mut *const ::std::os::raw::c_char,
        name_length: *mut size_t,
    ) -> CassError;
}
extern "C" {
    pub fn cass_result_column_type(result: CassConstPtr<CassResult>, index: size_t) -> CassValueType;
}
extern "C" {
    pub fn cass_result_column_data_type(
        result: CassConstPtr<CassResult>,
        index: size_t,
    ) -> CassConstPtr<CassDataType>;
}
extern "C" {
    pub fn cass_result_first_row(result: CassConstPtr<CassResult>) -> CassConstPtr<CassRow>;
}
extern "C" {
    pub fn cass_result_has_more_pages(result: CassConstPtr<CassResult>) -> cass_bool_t;
}
extern "C" {
    pub fn cass_result_paging_state_token(
        result: CassConstPtr<CassResult>,
        paging_state: *mut *const ::std::os::raw::c_char,
        paging_state_size: *mut size_t,
    ) -> CassError;
}
*/

// CassIterator functions:
/*
extern "C" {
    pub fn cass_iterator_type(iterator: CassMutPtr<CassIterator>) -> CassIteratorType;
}

extern "C" {
    pub fn cass_iterator_from_row(row: CassConstPtr<CassRow>) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_from_collection(value: CassConstPtr<CassValue>) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_from_map(value: CassConstPtr<CassValue>) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_from_tuple(value: CassConstPtr<CassValue>) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_user_type(value: CassConstPtr<CassValue>) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_keyspaces_from_schema_meta(
        schema_meta: CassConstPtr<CassSchemaMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_tables_from_keyspace_meta(
        keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_materialized_views_from_keyspace_meta(
        keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_user_types_from_keyspace_meta(
        keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_functions_from_keyspace_meta(
        keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_aggregates_from_keyspace_meta(
        keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_keyspace_meta(
        keyspace_meta: CassConstPtr<CassKeyspaceMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_columns_from_table_meta(
        table_meta: CassConstPtr<CassTableMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_indexes_from_table_meta(
        table_meta: CassConstPtr<CassTableMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_materialized_views_from_table_meta(
        table_meta: CassConstPtr<CassTableMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_table_meta(
        table_meta: CassConstPtr<CassTableMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_columns_from_materialized_view_meta(
        view_meta: CassConstPtr<CassMaterializedViewMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_materialized_view_meta(
        view_meta: CassConstPtr<CassMaterializedViewMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_column_meta(
        column_meta: CassConstPtr<CassColumnMeta>,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_index_meta(
        index_meta: *const CassIndexMeta,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_function_meta(
        function_meta: *const CassFunctionMeta,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_fields_from_aggregate_meta(
        aggregate_meta: *const CassAggregateMeta,
    ) -> CassMutPtr<CassIterator>;
}
extern "C" {
    pub fn cass_iterator_get_column(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassValue>;
}
extern "C" {
    pub fn cass_iterator_get_value(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassValue>;
}
extern "C" {
    pub fn cass_iterator_get_map_key(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassValue>;
}
extern "C" {
    pub fn cass_iterator_get_map_value(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassValue>;
}
extern "C" {
    pub fn cass_iterator_get_user_type_field_name(
        iterator: CassConstPtr<CassIterator>,
        name: *mut *const ::std::os::raw::c_char,
        name_length: *mut size_t,
    ) -> CassError;
}
extern "C" {
    pub fn cass_iterator_get_user_type_field_value(
        iterator: CassConstPtr<CassIterator>,
    ) -> CassConstPtr<CassValue>;
}
extern "C" {
    pub fn cass_iterator_get_keyspace_meta(
        iterator: CassConstPtr<CassIterator>,
    ) -> CassConstPtr<CassKeyspaceMeta>;
}
extern "C" {
    pub fn cass_iterator_get_table_meta(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassTableMeta>;
}
extern "C" {
    pub fn cass_iterator_get_materialized_view_meta(
        iterator: CassConstPtr<CassIterator>,
    ) -> CassConstPtr<CassMaterializedViewMeta>;
}
extern "C" {
    pub fn cass_iterator_get_user_type(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassDataType>;
}
extern "C" {
    pub fn cass_iterator_get_function_meta(
        iterator: CassConstPtr<CassIterator>,
    ) -> *const CassFunctionMeta;
}
extern "C" {
    pub fn cass_iterator_get_aggregate_meta(
        iterator: CassConstPtr<CassIterator>,
    ) -> *const CassAggregateMeta;
}
extern "C" {
    pub fn cass_iterator_get_column_meta(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassColumnMeta>;
}
extern "C" {
    pub fn cass_iterator_get_index_meta(iterator: CassConstPtr<CassIterator>) -> *const CassIndexMeta;
}
extern "C" {
    pub fn cass_iterator_get_meta_field_name(
        iterator: CassConstPtr<CassIterator>,
        name: *mut *const ::std::os::raw::c_char,
        name_length: *mut size_t,
    ) -> CassError;
}
extern "C" {
    pub fn cass_iterator_get_meta_field_value(iterator: CassConstPtr<CassIterator>) -> CassConstPtr<CassValue>;
}
*/

// CassRow functions:
/*
extern "C" {
    pub fn cass_row_get_column_by_name(
        row: CassConstPtr<CassRow>,
        name: *const ::std::os::raw::c_char,
    ) -> CassConstPtr<CassValue>;
}
extern "C" {
    pub fn cass_row_get_column_by_name_n(
        row: CassConstPtr<CassRow>,
        name: *const ::std::os::raw::c_char,
        name_length: size_t,
    ) -> CassConstPtr<CassValue>;
}
*/

// CassValue functions:
/*
#[no_mangle]
pub unsafe extern "C" fn cass_value_get_bytes(
    value: CassConstPtr<CassValue>,
    output: *mut *const cass_byte_t,
    output_size: *mut size_t,
) -> CassError {
}
#[no_mangle]
pub unsafe extern "C" fn cass_value_get_decimal(
    value: CassConstPtr<CassValue>,
    varint: *mut *const cass_byte_t,
    varint_size: *mut size_t,
    scale: *mut cass_int32_t,
) -> CassError {
}
#[no_mangle]
pub unsafe extern "C" fn cass_value_get_duration(
    value: CassConstPtr<CassValue>,
    months: *mut cass_int32_t,
    days: *mut cass_int32_t,
    nanos: *mut cass_int64_t,
) -> CassError {
}
extern "C" {
    pub fn cass_value_data_type(value: CassConstPtr<CassValue>) -> CassConstPtr<CassDataType>;
}
extern "C" {
    pub fn cass_value_type(value: CassConstPtr<CassValue>) -> CassValueType;
}
extern "C" {
    pub fn cass_value_is_collection(value: CassConstPtr<CassValue>) -> cass_bool_t;
}
extern "C" {
    pub fn cass_value_is_duration(value: CassConstPtr<CassValue>) -> cass_bool_t;
}
extern "C" {
    pub fn cass_value_item_count(collection: CassConstPtr<CassValue>) -> size_t;
}
extern "C" {
    pub fn cass_value_primary_sub_type(collection: CassConstPtr<CassValue>) -> CassValueType;
}
extern "C" {
    pub fn cass_value_secondary_sub_type(collection: CassConstPtr<CassValue>) -> CassValueType;
}
*/
