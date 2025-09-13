use core::error::Error;

/// A trait representing an HTTP client.
pub trait HttpClient: Clone + Send {
    /// The type of error that can occur after [`send`](HttpClient::send)ing a
    /// request.
    type Error: Error + Send;

    /// Asynchronously sends an HTTP request and waits for the response.
    fn send(
        &self,
        request: http::Request<String>,
    ) -> impl Future<Output = Result<http::Response<String>, Self::Error>> + Send;
}

impl<T: HttpClient + Sync> HttpClient for &T {
    type Error = T::Error;

    async fn send(
        &self,
        request: http::Request<String>,
    ) -> Result<http::Response<String>, Self::Error> {
        (*self).send(request).await
    }
}

