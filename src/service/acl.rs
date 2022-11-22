use tonic::{Request, Status};

#[tracing::instrument(name = "authenticate")]
pub async fn authenticate(req: Request<()>) -> Result<Request<()>, Status> {
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

    println!("pat: {}", pat);

    let octocrab = match octocrab::OctocrabBuilder::new()
        .personal_token(pat.to_string())
        .build()
    {
        Ok(octocrab) => octocrab,
        Err(_) => {
            return Err(Status::new(
                tonic::Code::Internal,
                "failed to create octocrab instance",
            ));
        }
    };

    match octocrab.current().user().await {
        Ok(user) => {
            tracing::info!("PAT is user with login '{}'", user.login);

            Ok(req)
        }
        Err(err) => {
            let err = format!("failed to authenticate PAT as user: {}", err);

            println!("{}", err);

            match octocrab.current().app().await {
                Ok(app) => {
                    tracing::info!("PAT is app with name '{}'", app.name);

                    Ok(req)
                }
                Err(err) => {
                    let err = format!("failed to authenticate PAT as app: {}", err);

                    println!("{}", err);

                    Err(Status::new(tonic::Code::PermissionDenied, err))
                }
            }
        }
    }
}
