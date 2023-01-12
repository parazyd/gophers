/* This file is part of gophers (https://github.com/parazyd/gophers)
 *
 * Copyright (C) 2023 parazyd <parazyd@dyne.org>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{
    io,
    io::{Read, Write},
    net::TcpStream,
};

use native_tls::{TlsConnector, TlsStream};
use url::Url;

/// Exported library error types
#[derive(thiserror::Error, Debug)]
pub enum GopherError {
    #[error("Invalid host")]
    InvalidHost,

    #[error("Unsupported protocol")]
    UnsupportedProtocol,

    #[error(transparent)]
    HandshakeError(#[from] native_tls::HandshakeError<TcpStream>),

    #[error(transparent)]
    TlsError(#[from] native_tls::Error),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
}

/// The Gopher struct represents an initialized object ready to connect.
pub struct Gopher {
    host: String,
    port: u16,
    tls: bool,
}

impl Gopher {
    /// Create a new `Gopher` object with the given endpoint.
    ///
    /// # Example
    /// ```
    /// use gophers::Gopher;
    /// let gopher = Gopher::new("gophers://bitreich.org").unwrap();
    /// ```
    pub fn new(endpoint: &str) -> Result<Self, GopherError> {
        let url = Url::parse(endpoint)?;

        if url.host().is_none() {
            return Err(GopherError::InvalidHost);
        }

        let (host, tls) = match url.scheme() {
            "gopher" => (url.host().unwrap(), false),
            "gophers" => (url.host().unwrap(), true),
            _ => return Err(GopherError::UnsupportedProtocol),
        };

        Ok(Self {
            host: host.to_string(),
            port: url.port().unwrap_or(70),
            tls,
        })
    }

    /// Establish a connection with a created Gopher object.
    /// Depending on `tls`, it will establish either a plain TCP or an
    /// encrypted TLS connection.
    ///
    /// # Example
    /// ```
    /// use gophers::Gopher;
    /// let gopher = Gopher::new("gophers://bitreich.org").unwrap();
    /// let mut stream = gopher.connect().unwrap();
    /// ```
    pub fn connect(&self) -> Result<GopherConnection, GopherError> {
        let tcp_conn = TcpStream::connect(format!("{}:{}", self.host, self.port))?;

        if !self.tls {
            return Ok(GopherConnection::Tcp(tcp_conn));
        }

        let tls_conn = TlsConnector::new()?;
        let stream = tls_conn.connect(&self.host, tcp_conn)?;

        Ok(GopherConnection::Tls(stream))
    }
}

/// Abstraction enum over TCP and TLS connections.
/// Implements both `Read` and `Write` traits.
pub enum GopherConnection {
    Tcp(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl Write for GopherConnection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(c) => c.write(buf),
            Self::Tls(c) => c.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tcp(c) => c.flush(),
            Self::Tls(c) => c.flush(),
        }
    }
}

impl Read for GopherConnection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(c) => c.read(buf),
            Self::Tls(c) => c.read(buf),
        }
    }
}

impl GopherConnection {
    /// Fetch a resource given a path from an established Gopher connection.
    ///
    /// # Example
    /// ```
    /// use gophers::Gopher;
    /// let gopher = Gopher::new("gophers://bitreich.org").unwrap();
    /// let mut stream = gopher.connect().unwrap();
    /// let data = stream.fetch("/memecache/index.meme").unwrap();
    /// assert_eq!(&data[..5], b"meme2");
    /// ```
    pub fn fetch(&mut self, path: &str) -> Result<Vec<u8>, io::Error> {
        let req = format!("{}\r\n", path);
        self.write_all(req.as_bytes())?;
        let mut buf = vec![];
        self.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
