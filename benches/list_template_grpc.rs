use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use futures::future::join_all;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use akira_core::template::template_client::TemplateClient;
use akira_core::ListTemplatesRequest;

const PARALLELISM: i32 = 50;
const REQUESTS_PER_ASYNC_TASK: i32 = 1000;

// This construct allows us to have one mutable client per REQUESTS_PER_ASYNC_TASK calls of the GRPC endpoint.
// We add async parallelism on top of this to discover concurrency.
async fn perform_call_block() {
    let channel = Channel::from_static("http://[::1]:50051")
        .connect()
        .await
        .unwrap();

    let mut client =
        TemplateClient::with_interceptor(channel.clone(), move |mut req: Request<()>| {
            let token = MetadataValue::from_str("Bearer 123ABC").unwrap();
            req.metadata_mut().insert("authorization", token);
            Ok(req)
        });

    for _ in 0..REQUESTS_PER_ASYNC_TASK {
        let request = Request::new(ListTemplatesRequest {});
        client.list(request).await.unwrap();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_template_grpc");

    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::from_parameter(PARALLELISM * REQUESTS_PER_ASYNC_TASK),
        &PARALLELISM,
        |b, &_| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    let mut futures = Vec::new();
                    for _ in 0..PARALLELISM {
                        futures.push(perform_call_block());
                    }
                    join_all(futures).await;
                })
        },
    );

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
