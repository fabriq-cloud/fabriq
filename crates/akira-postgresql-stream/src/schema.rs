table! {
    event_queue (id) {
        id -> Text,
        event_timestamp -> Timestamp,
        consumer_id -> Text,
        operation_id -> Nullable<Text>,
        model_type -> Int4,
        serialized_current_model -> Nullable<Bytea>,
        serialized_previous_model -> Nullable<Bytea>,
        event_type -> Int4,
    }
}
