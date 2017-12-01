#[macro_use] extern crate log;
extern crate env_logger;
extern crate futures;
extern crate hyper;
extern crate tokio_core;

use futures::future::Future;
use futures::Stream;
use hyper::{Body, Client, StatusCode};
use hyper::client::HttpConnector;
use hyper::header::{ContentLength, ContentType};
use hyper::server::{Http, Request, Response, Service};
use tokio_core::reactor::Core;

struct Prox {
    client: Client<HttpConnector, Body>
}

impl Prox {
    fn new(client: Client<HttpConnector, Body>) -> Prox {
        Prox { client: client }
    }
}

impl Service for Prox {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        info!("Request: {:?}", req);
        let work = self.client.request(req)
            .and_then(|res| {
                info!("Response: {:?}", res);
                futures::future::ok(res)
            })
            .or_else(|err| {
                error!("Error: {:?}", err);
                let body = format!("{}\n", err);
                futures::future::ok(Response::new()
                    .with_status(StatusCode::BadRequest)
                    .with_header(ContentType::plaintext())
                    .with_header(ContentLength(body.len() as u64))
                    .with_body(body))
            });
        Box::new(work)
    }
}

fn main() {
    env_logger::init().unwrap();

    let addr = "127.0.0.1:3000".parse().unwrap();
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let handle1 = handle.clone();
    let server = Http::new().serve_addr_handle(&addr, &handle, move || {
        Ok(Prox::new(Client::new(&handle1)))
    }).unwrap();
    let handle2 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        handle2.spawn(conn.map(|_| ()).map_err(|err| error!("{:?}", err)));
        Ok(())
    }).map_err(|_| ()));
    core.run(futures::future::empty::<(), ()>()).unwrap();
}
