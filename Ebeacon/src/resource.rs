// src/resource.rs
// -------------------------------------------------------------
// Sets up HTTP endpoints (the API for the resource) that process
// transactions. We define two endpoints:
// - "/clientrequest": receives a client request (TxMessage) and responds accordingly.
// - "/forward": receives a forwarded message and responds with a signed TxMessage.
// -------------------------------------------------------------
use warp::Filter;
use crate::handler::{handle_client_request, handle_forward_action};

pub async fn start_server() {
    let client_request_route = warp::post()
         .and(warp::path("clientrequest"))
         .and(warp::body::json())
         .and_then(handle_client_request);

    let forward_route = warp::post()
         .and(warp::path("forward"))
         .and(warp::body::json())
         .and_then(handle_forward_action);

    let routes = client_request_route.or(forward_route);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
