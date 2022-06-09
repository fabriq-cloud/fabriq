fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/common.proto")?;

    tonic_build::compile_protos("proto/assignment.proto")?;
    tonic_build::compile_protos("proto/deployment.proto")?;
    tonic_build::compile_protos("proto/event.proto")?;
    tonic_build::compile_protos("proto/health.proto")?;
    tonic_build::compile_protos("proto/host.proto")?;
    tonic_build::compile_protos("proto/target.proto")?;
    tonic_build::compile_protos("proto/template.proto")?;
    tonic_build::compile_protos("proto/workload.proto")?;
    tonic_build::compile_protos("proto/workspace.proto")?;

    Ok(())
}
