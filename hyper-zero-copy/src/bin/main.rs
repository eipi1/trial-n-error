use std::borrow::Cow;
use std::env;
use std::str::FromStr;
use axum::{
    Json,
    extract::{Extension, Path},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use axum::extract::State;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use hyper::{Client, Uri};
use hyper::client::HttpConnector;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use yoke::Yoke;

struct AppState {
    // ...
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(Client::new());

    let uri = Uri::from_str(
        format!(
            "http://{}:1080/hello",
            env::var("host").unwrap_or("localhost".to_string())
        )
            .as_str(),
    ).unwrap();

    let app = Router::new()
        .route(
            "/zc",
            get(zero_copy),
        )
        .with_state((shared_state.clone(), uri.clone()))
        .route(
            "/serde",
            get(serde_val),
        )
        .with_state((shared_state.clone(), uri.clone()))
        ;


    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:2000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct SerializableYok(Yoke<serde_zero_copy::Value<'static>, Arc<Bytes>>);

// impl Serialize for SerializableYok {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//         self.0.get().serialize(serializer)
//     }
// }

impl IntoResponse for SerializableYok {
    fn into_response(self) -> Response {

        // Use a small initial capacity of 128 bytes like serde_json::to_vec
        // https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189
        let mut buf = BytesMut::with_capacity(128).writer();
        match serde_json_nostr::to_writer(&mut buf, &self.0.get()) {
            Ok(()) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                )],
                buf.into_inner().freeze(),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}

// async fn root_agg(State(client): State<Arc<Client<HttpConnector>>>, State(uri): State<Uri>) -> Bytes {
// #[axum_macros::debug_handler]
async fn zero_copy(State((client, uri)): State<(Arc<Client<HttpConnector>>, Uri)>) -> SerializableYok {
    let res = client.get(uri).await.unwrap();
    // let buf = hyper::body::aggregate(res).await.unwrap();
    let buf = hyper::body::to_bytes(res).await.unwrap();
    // let val: Value = serde_json::from_slice(buf.as_ref()).unwrap();
    // let val: serde_zero_copy::Value = serde_json_nostr::from_slice(&buf).unwrap();
    let buf = Arc::new(buf);
    let yoked = yoke::Yoke::<serde_zero_copy::Value<'static>, Arc<Bytes>>::attach_to_cart(buf, |b| {
        let val = serde_json_nostr::from_slice(b).unwrap();
        val
    });
    SerializableYok(yoked)
    // buf
    // return to_opaque(buf).unwrap();
}

// #[axum_macros::debug_handler]
async fn serde_val(State((client, uri)): State<(Arc<Client<HttpConnector>>, Uri)>) -> Json<Value> {
    let res = client.get(uri).await.unwrap();
    let buf = hyper::body::to_bytes(res).await.unwrap();
    let val: Value = serde_json::from_slice(buf.as_ref()).unwrap();
    Json(val)
}



async fn get_user(state: Arc<Client<HttpConnector>>) {
    // ...
}

async fn create_user(Json(payload): Json<CreateUserPayload>, state: Arc<AppState>) {
    // ...
}

#[derive(Deserialize)]
struct CreateUserPayload {
    // ...
}


/*
use hyper::{Client, Uri};
use std::env;
use std::io::Write;
use std::os::unix::raw::mode_t;
use std::str::FromStr;
// use std::sync::Arc;
use std::time::Duration;
// use axum::Router;
// use axum::routing::get;
use bytes::Buf;
use hyper::client::HttpConnector;

#[tokio::main]
async fn main() {
    // for _ in env::var("iter").map(|t| parse::<usize>(t)).unwrap_or_default() {
    //
    // }
    let client = Arc::new(Client::new());
    let uri = Uri::from_str(
        format!(
            "http://{}:1080/hello",
            env::var("host").unwrap_or("localhost".to_string())
        )
            .as_str(),
    ).unwrap();

    let path = env::var("aggregate").unwrap_or("0".to_string())
        .parse::<usize>().unwrap_or(0);

    let app = Router::new()
        .route("/", get({
            let temp_path=path.clone();
            move|| root(temp_path)
        }))
        // `GET /` goes to `root`
        // .route("/agg", get({
        //     let cloned = client.clone();
        //     let uri1 = uri.clone();
        //     move || root_agg(path, cloned, uri1)
        // }))
        // .route("/bytes", get({
        //     let cloned = client.clone();
        //     let uri1 = uri.clone();
        //     move || root_agg(cloned, uri1)
        // }))
    ;

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// #[axum_macros::debug_handler]
async fn root_agg(client: Arc<Client<HttpConnector>>, uri: Uri) -> impl Buf {
    let res = client.get(uri.clone()).await.unwrap();
    let buf = hyper::body::aggregate(res).await.unwrap();
    buf
    // return to_opaque(buf).unwrap();
}
#[axum_macros::debug_handler]
async fn root(path: usize) {}

// #[axum_macros::debug_handler]
// async fn root_bytes(path: usize, client: Arc<Client<HttpConnector>>, uri: Uri) -> impl Buf {
//     let res = client.get(uri.clone()).await.unwrap();
//     let buf = hyper::body::to_bytes(res).await.unwrap();
//     buf
//     // return to_opaque(buf).unwrap();
// }

fn to_opaque(buf: impl Buf) -> Option<impl Buf> {
    Some(buf)
}
*/