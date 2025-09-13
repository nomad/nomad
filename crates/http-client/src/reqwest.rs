use crate::HttpClient;

impl HttpClient for reqwest::Client {
    type Error = reqwest::Error;

    #[inline]
    async fn send(
        &self,
        request: http::Request<String>,
    ) -> Result<http::Response<String>, Self::Error> {
        let (parts, body) = request.into_parts();

        let mut request_builder =
            self.request(parts.method, parts.uri.to_string()).body(body);

        for (maybe_name, value) in parts.headers {
            if let Some(name) = maybe_name {
                request_builder = request_builder.header(name, value);
            }
        }

        let reqwest_response = request_builder.send().await?;

        let headers = reqwest_response.headers().clone();

        let mut http_response = http::Response::builder()
            .status(reqwest_response.status())
            .version(match reqwest_response.version() {
                reqwest::Version::HTTP_09 => http::Version::HTTP_09,
                reqwest::Version::HTTP_10 => http::Version::HTTP_10,
                reqwest::Version::HTTP_11 => http::Version::HTTP_11,
                reqwest::Version::HTTP_2 => http::Version::HTTP_2,
                reqwest::Version::HTTP_3 => http::Version::HTTP_3,
                other => unreachable!("invalid HTTP version: {other:?}"),
            })
            .body(reqwest_response.text().await?)
            .expect("all the fields have been validated by reqwest");

        *http_response.headers_mut() = headers;

        Ok(http_response)
    }
}
