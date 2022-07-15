table! {
    assignments (id) {
        id -> Text,
        deployment_id -> Text,
        host_id -> Text,
    }
}

table! {
    configs (id) {
        id -> Text,
        owning_model -> Text,
        key -> Text,
        value -> Text,
        value_type -> Int4,
    }
}

table! {
    deployments (id) {
        id -> Text,
        name -> Text,
        workload_id -> Text,
        target_id -> Text,
        template_id -> Nullable<Text>,
        host_count -> Int4,
    }
}

table! {
    hosts (id) {
        id -> Text,
        labels -> Array<Text>,
    }
}

table! {
    targets (id) {
        id -> Text,
        labels -> Array<Text>,
    }
}

table! {
    templates (id) {
        id -> Text,
        repository -> Text,
        branch -> Text,
        path -> Text,
    }
}

table! {
    workloads (id) {
        id -> Text,
        name -> Text,
        team_id -> Text,
        template_id -> Text,
    }
}

joinable!(assignments -> deployments (deployment_id));
joinable!(assignments -> hosts (host_id));
joinable!(deployments -> targets (target_id));
joinable!(deployments -> templates (template_id));
joinable!(deployments -> workloads (workload_id));
joinable!(workloads -> templates (template_id));

allow_tables_to_appear_in_same_query!(
    assignments,
    configs,
    deployments,
    hosts,
    targets,
    templates,
    workloads,
);
