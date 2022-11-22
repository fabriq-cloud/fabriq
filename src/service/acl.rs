use tonic::{Request, Status};

pub async fn get_pat_from_headers<T>(req: &Request<T>) -> Result<String, Status> {
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

    let pat = match auth_header.to_str() {
        Ok(pat) => pat,
        Err(_) => {
            return Err(Status::new(
                tonic::Code::InvalidArgument,
                "authorization header malformed",
            ))
        }
    };

    Ok(pat.to_string())
}

#[tracing::instrument(name = "authenticate")]
pub async fn authenticate(req: Request<()>) -> Result<Request<()>, Status> {
    get_pat_from_headers(&req).await?;

    Ok(req)
}
