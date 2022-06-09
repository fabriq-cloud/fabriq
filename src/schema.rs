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
        model_type -> Int2,
        model_id -> Text,
        key -> Text,
        value -> Text,
    }
}

table! {
    deployments (id) {
        id -> Text,
        workload_id -> Text,
        target_id -> Text,
        replicas -> Int4,
    }
}

table! {
    hosts (id) {
        id -> Text,
        labels -> Array<Text>,
        cpu_capacity -> Int4,
        memory_capacity -> Int8,
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
        workspace_id -> Text,
        template_id -> Text,
    }
}

table! {
    workspaces (id) {
        id -> Text,
    }
}

joinable!(assignments -> deployments (deployment_id));
joinable!(assignments -> hosts (host_id));
joinable!(deployments -> targets (target_id));
joinable!(deployments -> workloads (workload_id));
joinable!(workloads -> templates (template_id));
joinable!(workloads -> workspaces (workspace_id));

allow_tables_to_appear_in_same_query!(
    assignments,
    configs,
    deployments,
    hosts,
    targets,
    templates,
    workloads,
    workspaces,
);
