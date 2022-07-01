use tonic::{
    metadata::{Ascii, MetadataValue},
    service::Interceptor,
    Status,
};

pub struct ClientInterceptor {
    pub token: MetadataValue<Ascii>,
}

impl Interceptor for ClientInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        request
            .metadata_mut()
            .insert("authorization", self.token.clone());
        Ok(request)
    }
}
