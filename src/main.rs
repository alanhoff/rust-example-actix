extern crate actix;
extern crate bytes;
extern crate tokio;
extern crate tokio_io;

use actix::io::{FramedWrite, WriteHandler};
use actix::prelude::*;
use actix::{Actor, Addr, Context, Handler, Syn};
use bytes::Bytes;
use tokio::io;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::stream::Stream;
use tokio::prelude::*;
use tokio_io::codec::{BytesCodec, FramedRead};

struct Connection {
    writer: FramedWrite<WriteHalf<TcpStream>, BytesCodec>,
}

impl WriteHandler<io::Error> for Connection {}

#[derive(Message)]
struct AttachReadStream {
    stream: ReadHalfStream,
}

impl StreamHandler<Vec<u8>, io::Error> for Connection {
    fn handle(&mut self, buffer: Vec<u8>, _ctx: &mut Context<Self>) {
        println!("I'll write");
        self.writer.write(Bytes::from(buffer));
    }
}

impl Handler<AttachReadStream> for Connection {
    type Result = ();

    fn handle(&mut self, msg: AttachReadStream, ctx: &mut Context<Self>) -> Self::Result {
        ctx.add_stream(msg.stream);
    }
}

impl Actor for Connection {
    type Context = Context<Self>;
}

struct ReadHalfStream {
    socket: ReadHalf<TcpStream>,
}

impl Stream for ReadHalfStream {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let mut buffer: Vec<u8> = Vec::new();

        match self.socket.read_buf(&mut buffer)? {
            Async::Ready(0) => Result::Ok(Async::Ready(None)),
            Async::Ready(_) => Result::Ok(Async::Ready(Some(buffer))),
            Async::NotReady => Result::Ok(Async::NotReady),
        }
    }
}

struct Server;

impl StreamHandler<TcpStream, io::Error> for Server {
    fn handle(&mut self, connection: TcpStream, _ctx: &mut Self::Context) {
        connection
            .set_nodelay(true)
            .expect("Unable to set nodelayx");

        let (read, write) = connection.split();
        let addr: Addr<Syn, _> = Connection::create(move |ctx| {
            let mut writer = FramedWrite::new(write, BytesCodec::new(), ctx);
            writer.set_buffer_capacity(0, 0);

            Connection { writer }
        });

        let stream = ReadHalfStream { socket: read };

        addr.do_send(AttachReadStream { stream });
    }
}

impl Actor for Server {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = "0.0.0.0:8080".parse().expect("Unable to parse address");
        let tcp = TcpListener::bind(&addr).expect("Unable to bind port");
        println!("Server started");
        ctx.add_stream(tcp.incoming());
    }
}

fn main() {
    let system = actix::System::new("system");
    let _: Addr<Syn, _> = Server.start();

    system.run();
}
