use std::pin::Pin;

use proto::veil::veil_server::{Veil, VeilServer};
use proto::veil::webrtc::*;
use proto::veil::*;
use std::result::Result;
use tokio_stream::Stream;
use tonic::transport::Server as TonicServer;
use tonic::{Request, Response, Status};

pub type TrickleIceStream =
    Pin<Box<dyn Stream<Item = Result<IceCandidate, Status>> + Send + Sync + 'static>>;

pub struct Server {}

#[tonic::async_trait]
impl Veil for Server {
    type TrickleIceStream = TrickleIceStream;

    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        todo!("Implement session creation")
    }
    async fn set_answer(
        &self,
        request: Request<SetAnswerRequest>,
    ) -> Result<Response<SetAnswerResponse>, Status> {
        todo!("Implement setting answer")
    }
    async fn renegotiate(
        &self,
        request: Request<RenegotiateRequest>,
    ) -> Result<Response<RenegotiateResponse>, Status> {
        todo!("Implement renegotiation")
    }

    async fn trickle_ice(
        &self,
        request: Request<tonic::Streaming<IceCandidate>>,
    ) -> Result<Response<Self::TrickleIceStream>, Status> {
        todo!("Implement ICE trickling")
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    let server = Server {};

    println!("Starting gRPC server on {}", addr);

    TonicServer::builder()
        .add_service(VeilServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
