#[macro_use]
extern crate log;

extern crate hyper;

pub mod err {
    
    macro_rules! from {
        ($t: ty) => {
            impl ::std::convert::From<$t> for Error {
                fn from(e: $t) -> Self {
                    Error(e.into())
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct Error(pub Box<::std::error::Error + Send + Sync>);

    impl ::std::error::Error for Error {
        fn description(&self) -> &str {
            self.0.description()
        }
    }

    impl ::std::fmt::Display for Error {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            ::std::error::Error::description(self).fmt(f)
        }
    }

    from!(::std::io::Error);
    from!(::hyper::Error);
    from!(&'static str);
    from!(String);

    pub type Result<T> = ::std::result::Result<T, Error>;

}

pub use hyper::method::Method;

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Connection {
    client: Arc<hyper::Client>,
    headers: Option<hyper::header::Headers>,
    url: hyper::Url,
}

impl Connection {

    pub fn new<U: hyper::client::IntoUrl>(url: U) -> err::Result<Self> {
        if let Ok(url) = url.into_url() {
            let out = Connection {
                url: url,
                client: Arc::new(hyper::Client::new()),
                headers: None
            };
            Ok(out)
        } else {
            Err("Could not parse URL".into())
        }        
    }

    pub fn with_headers(mut self, headers: hyper::header::Headers) -> Self {        
        self.headers = Some(headers);
        self
    }

    pub fn request(&self, method: ::Method) -> Request {
        let Connection { client, headers, url } = self.clone();
        Request {
            client: client,
            headers: headers,
            url: url,
            method: method,
            body: None
        }
    }

}

#[derive(Debug, Clone)]
pub struct Request<'a> {
    client: Arc<hyper::Client>,
    headers: Option<hyper::header::Headers>,
    url: hyper::Url,
    method: hyper::method::Method,
    body: Option<&'a [u8]>
}

impl<'a> Request<'a> {

    pub fn with_path(mut self, path: &str) -> Self {
        self.url.set_path(path);
        self
    }

    pub fn with_query(mut self, pairs: &[(&str, &str)]) -> Self {        
        {
            let mut ser = self.url.query_pairs_mut();

            for &(k, v) in pairs {
                ser.append_pair(k, v);
            }
        }

        self
    }

    pub fn with_body(mut self, body: &'a [u8]) -> Self {
        self.body = Some(body);
        self
    }


    pub fn send(self) -> err::Result<Vec<u8>> {

        use std::io::Read;

        let mut builder = self.client.request(self.method, self.url);
        
        if let Some(headers) = self.headers {
            builder = builder.headers(headers);
        }

        if let Some(body) = self.body {
            builder = builder.body(body);
        }

        let mut response = try!(builder.send());

        if !response.status.is_success() {
            let mut e = String::new();
            let _ = response.read_to_string(&mut e);
            if e.is_empty() { e.push_str("Unknown error"); }

            return Err(e.into());
        }

        let mut buffer = {
            if let Some(&hyper::header::ContentLength(len)) = response.headers.get() {
                Vec::with_capacity(len as usize)
            } else {
                Vec::new()
            }
        };

        let _ = try!(response.read_to_end(&mut buffer));

        Ok(buffer)

    }

}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn it_works() {

        let conn = Connection::new("http://localhost:3000").unwrap();

        let req = conn.request(Method::Get)
            .with_path("/foo/bar")
            .with_query(&[("yes", "no")]);

        println!("{:?}", &req.url);

    }
}
