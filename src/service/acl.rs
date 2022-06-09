use tonic::{Request, Status};

pub fn authorize(req: Request<()>) -> Result<Request<()>, Status> {
    Ok(req)
}
