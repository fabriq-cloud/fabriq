use tonic::{Request, Status};

#[tracing::instrument(name = "authorize")]
pub fn authorize(req: Request<()>) -> Result<Request<()>, Status> {
    let headers = req.metadata().clone().into_headers();

    let auth_header = match headers.get("authorization") {
        Some(auth_header) => auth_header,
        None => {
            return Err(Status::new(
                tonic::Code::Unauthenticated,
                "missing authorization header",
            ))
        }
    };

    let _pat = match auth_header.to_str() {
        Ok(pat) => pat,
        Err(_) => {
            return Err(Status::new(
                tonic::Code::InvalidArgument,
                "authorization header malformed",
            ))
        }
    };

    // TODO: validate PAT when https://github.com/hyperium/tonic/pull/910 lands async interceptors

    Ok(req)
}
