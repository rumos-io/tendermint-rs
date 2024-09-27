//! ABCI application server interface.

use std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    thread,
};

use gancellation_token::{CancellationSource, CancellationToken};
use tracing::{error, info};

use crate::{application::RequestDispatcher, codec::ServerCodec, error::Error, Application};

/// The size of the read buffer for each incoming connection to the ABCI
/// server (1MB).
pub const DEFAULT_SERVER_READ_BUF_SIZE: usize = 1024 * 1024;

/// Allows us to configure and construct an ABCI server.
pub struct ServerBuilder {
    read_buf_size: usize,
}

impl ServerBuilder {
    /// Builder constructor.
    ///
    /// Allows you to specify the read buffer size used when reading chunks of
    /// incoming data from the client. This needs to be tuned for your
    /// application.
    pub fn new(read_buf_size: usize) -> Self {
        Self { read_buf_size }
    }

    /// Constructor for an ABCI server.
    ///
    /// Binds the server to the given address. You must subsequently call the
    /// [`Server::listen`] method in order for incoming connections' requests
    /// to be routed to the specified ABCI application.
    pub fn bind<Addr, App>(self, addr: Addr, app: App) -> Result<Server<App>, Error>
    where
        Addr: ToSocketAddrs,
        App: Application,
    {
        let listener = TcpListener::bind(addr).map_err(Error::io)?;
        let local_addr = listener.local_addr().map_err(Error::io)?.to_string();
        info!("ABCI server running at {}", local_addr);
        Ok(Server {
            app,
            listener,
            local_addr,
            read_buf_size: self.read_buf_size,
            cancellation_source: CancellationSource::new(),
        })
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self {
            read_buf_size: DEFAULT_SERVER_READ_BUF_SIZE,
        }
    }
}

/// A TCP-based server for serving a specific ABCI application.
///
/// Each incoming connection is handled in a separate thread. The ABCI
/// application is cloned for access in each thread. It is up to the
/// application developer to manage shared state across these different
/// threads.
pub struct Server<App> {
    app: App,
    listener: TcpListener,
    local_addr: String,
    read_buf_size: usize,
    cancellation_source: CancellationSource,
}

impl<App: Application> Server<App> {
    pub fn token(&mut self) -> CancellationToken {
        self.cancellation_source.token()
    }

    /// Initiate a blocking listener for incoming connections.
    pub fn listen(mut self) -> Result<(), Error> {
        let mut token = self.cancellation_source.token();

        let _ = thread::spawn(move || {
            while !token.is_cancelled() {
                let connection = self.listener.accept().map_err(Error::io);

                match connection {
                    Ok((stream, addr)) => {
                        let addr = addr.to_string();
                        info!("Incoming connection from: {}", addr);
                        Self::spawn_client_handler(
                            stream,
                            addr,
                            self.app.clone(),
                            self.read_buf_size,
                            token.clone(),
                        );
                    },
                    Err(err) => {
                        error!("Error receiving connection: {err}");
                        token.cancel();
                    },
                }
            }
        });

        while !self.cancellation_source.is_cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(100))
        }

        Ok(())
    }

    /// Getter for this server's local address.
    pub fn local_addr(&self) -> String {
        self.local_addr.clone()
    }

    fn spawn_client_handler(
        stream: TcpStream,
        addr: String,
        app: App,
        read_buf_size: usize,
        token: CancellationToken,
    ) {
        let _ = thread::spawn(move || Self::handle_client(stream, addr, app, read_buf_size, token));
    }

    fn handle_client(
        stream: TcpStream,
        addr: String,
        app: App,
        read_buf_size: usize,
        token: CancellationToken,
    ) {
        let mut codec = ServerCodec::new(stream, read_buf_size);
        info!("Listening for incoming requests from {}", addr);
        while !token.is_cancelled() {
            let request = match codec.next() {
                Some(result) => match result {
                    Ok(r) => r,
                    Err(e) => {
                        error!(
                            "Failed to read incoming request from client {}: {:?}",
                            addr, e
                        );
                        return;
                    },
                },
                None => {
                    info!("Client {} terminated stream", addr);
                    return;
                },
            };
            let response = app.handle(request, &token);
            if let Err(e) = codec.send(response) {
                error!("Failed sending response to client {}: {:?}", addr, e);
                return;
            }
        }
    }
}
