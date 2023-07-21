#[cfg(test)]
mod tests {
    use hyper::{Body, Uri};
    use hyper::client::Client;

    #[tokio::test]
    async fn hyper_getting_started() {
        let client = Client::new();
        let res = client.get(Uri::from_static("http://localhost:1080/hello")).await.unwrap();
        let buf = hyper::body::to_bytes(res).await.unwrap();
        println!("body: {:?}", buf);
    }
}
