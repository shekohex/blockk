use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Default, PartialEq)]
pub struct ClientRequest {
    pub method: ConnectMethod,
    header: Vec<String>,
    data_block: Vec<String>,
}

#[derive(Debug, Default, PartialEq)]
pub struct ConnectMethod {
    pub name: String,
    pub uri: String,
    version: String,
}

impl ConnectMethod {
    fn new(name: &str, uri: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            uri: uri.to_string(),
            version: version.to_string(),
        }
    }
}

// HTTP response status
// https://developer.mozilla.org/en-US/docs/Web/HTTP/Status
pub enum ServerResponse {
    BadRequest,
    Forbidden,
    MethodNotAllowed,
    Ok,
}

pub async fn send_response<W: AsyncWriteExt + Unpin>(
    mut writer: W,
    response: ServerResponse,
) -> io::Result<()> {
    let (code, message) = match response {
        ServerResponse::BadRequest => (400, "Bad Request"),
        ServerResponse::Forbidden => (403, "Forbidden"),
        ServerResponse::MethodNotAllowed => (405, "Method Not Allowed"),
        ServerResponse::Ok => (200, "OK"),
    };
    let message = format!("HTTP/1.1 {} {}\r\n\r\n", code as u32, message);
    writer.write_all(&message.into_bytes()).await
}

// Support "CONNECT" [1, 2] only.
// [1] https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/CONNECT
// [2] https://tools.ietf.org/html/rfc2817#section-5.2
pub async fn get_request<R: AsyncReadExt + Unpin>(mut reader: R) -> io::Result<ClientRequest> {
    // A reasonable HTTP header size for CONNECT
    const MAX_REQUEST_SIZE: usize = 1024;
    let mut buf = [0; MAX_REQUEST_SIZE];
    let len = reader.read(&mut buf).await?;
    let request = std::str::from_utf8(&buf[..len])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    parse_request(request)
}

// Parse a typically HTTP client request. Works for "CONNECT" only now.
// [1]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Session#sending_a_client_request
fn parse_request(request: &str) -> io::Result<ClientRequest> {
    #[derive(PartialEq)]
    enum ParseState {
        Method,
        Header,
        DataBlock,
    }

    let mut parsed = ClientRequest::default();
    let mut state = ParseState::Method;

    let lines = request.split("\r\n").collect::<Vec<&str>>();
    let mut i = 0;
    while i < lines.len() {
        match state {
            ParseState::Method => {
                parsed.method = parse_method(lines[i])?;
                state = ParseState::Header;
            }
            ParseState::Header => {
                // The HTTP header ends with an "\r\n\r\n".
                // The pattern is "(parsed)\r\n(we are here)(unparsed)" if current line is empty.
                // The (unparsed) part could be a nothing, or a "\r\n(next unparsed)"
                if lines[i].is_empty() {
                    // If next line exists, pattern is "\r\n(next unparsed)". Otherwise, the parsing
                    // is finished (no next line). In this case, the request is incomplete since it
                    // doesn't contain a "\r\n\r\n" mark
                    let has_next_line = i + 1 < lines.len();
                    if has_next_line {
                        state = ParseState::DataBlock;
                    }
                } else {
                    parsed.header.push(lines[i].to_string());
                }
            }
            ParseState::DataBlock => {
                // skip the last line if it's empty
                if !lines[i].is_empty() || i < lines.len() - 1 {
                    parsed.data_block.push(lines[i].to_string());
                }
            }
        }
        i += 1;
    }

    if state == ParseState::DataBlock {
        Ok(parsed)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid request",
        ))
    }
}

fn parse_method(method: &str) -> io::Result<ConnectMethod> {
    let mut items = method.split(' ');
    let name = items.next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "No method in client request")
    })?;
    let uri = items
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No URI in client request"))?;
    let version = items.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "No HTTP protocol version in client request",
        )
    })?;
    Ok(ConnectMethod::new(name, uri, version))
}

#[test]
fn test_parse_get_request() {
    let request = "GET / HTTP/1.1\r\nHost: developer.mozilla.org\r\nAccept-Language: fr\r\n\r\n";
    let parsed = parse_request(request).unwrap();
    let expected = ClientRequest {
        method: ConnectMethod::new("GET", "/", "HTTP/1.1"),
        header: [
            "Host: developer.mozilla.org".to_string(),
            "Accept-Language: fr".to_string(),
        ]
        .to_vec(),
        data_block: Vec::default(),
    };
    assert_eq!(parsed, expected);

    let request = "GET / HTTP/1.1\r\n\r\n";
    let parsed = parse_request(request).unwrap();
    let expected = ClientRequest {
        method: ConnectMethod::new("GET", "/", "HTTP/1.1"),
        header: Vec::default(),
        data_block: Vec::default(),
    };
    assert_eq!(parsed, expected);

    // TODO: What is expected result for "GET / HTTP/1.1\r\n\r\n\r\n"?
}

#[test]
fn test_parse_post_request() {
    let request = "POST /contact_form.php HTTP/1.1\r\n\
    Host: developer.mozilla.org\r\n\
    Content-Length: 64\r\n\
    Content-Type: application/x-www-form-urlencoded\r\n\r\n\
    name=Joe%20User&request=Send%20me%20one%20of%20your%20catalogue";
    let parsed = parse_request(request).unwrap();
    let expected = ClientRequest {
        method: ConnectMethod::new("POST", "/contact_form.php", "HTTP/1.1"),
        header: [
            "Host: developer.mozilla.org".to_string(),
            "Content-Length: 64".to_string(),
            "Content-Type: application/x-www-form-urlencoded".to_string(),
        ]
        .to_vec(),
        data_block: ["name=Joe%20User&request=Send%20me%20one%20of%20your%20catalogue".to_string()]
            .to_vec(),
    };
    assert_eq!(parsed, expected);
}

#[test]
fn test_parse_connect_request() {
    let request = "CONNECT en.wikipedia.org:443 HTTP/1.1\r\n\r\n";
    let parsed = parse_request(request).unwrap();
    let expected = ClientRequest {
        method: ConnectMethod::new("CONNECT", "en.wikipedia.org:443", "HTTP/1.1"),
        header: Vec::default(),
        data_block: Vec::default(),
    };
    assert_eq!(parsed, expected);

    let request = "CONNECT en.wikipedia.org:443 HTTP/1.1\r\n\
    Host: en.wikipedia.org:443\r\n\
    User-Agent: curl/7.68.0\r\n\
    Proxy-Connection: Keep-Alive\r\n\r\n";
    let parsed = parse_request(request).unwrap();
    let expected = ClientRequest {
        method: ConnectMethod::new("CONNECT", "en.wikipedia.org:443", "HTTP/1.1"),
        header: [
            "Host: en.wikipedia.org:443".to_string(),
            "User-Agent: curl/7.68.0".to_string(),
            "Proxy-Connection: Keep-Alive".to_string(),
        ]
        .to_vec(),
        data_block: Vec::default(),
    };
    assert_eq!(parsed, expected);
}

#[test]
fn test_parse_invalid_request_without_emptyline() {
    // The request should be ended with "\r\n\r\n"
    let request = "CONNECT en.wikipedia.org:443 HTTP/1.1\r\n";
    assert!(parse_request(request).is_err());
    let request = "CONNECT en.wikipedia.org:443 HTTP/1.1\r\nHost: developer.mozilla.org";
    assert!(parse_request(request).is_err());
    let request = "GET / HTTP/1.1\r\nHost: developer.mozilla.org\r\nAccept-Language: fr";
    assert!(parse_request(request).is_err());
    let request = "GET / HTTP/1.1\r\nHost: developer.mozilla.org\r\nAccept-Language: fr\r\n";
    assert!(parse_request(request).is_err());
}
