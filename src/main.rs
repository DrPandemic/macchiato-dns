use std::convert::From;
use std::net::{SocketAddr, UdpSocket};
use std::str;

const MAX_DATAGRAM_SIZE: usize = 512;
const DEFAULT_DNS_RESOLVER: &str = "1.1.1.1:53";
const DEFAULT_PORT: u16 = 4053;

fn main() -> std::io::Result<()> {
    {
        let socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)))
            .expect("tried to bind an UDP port");

        // Receives a single datagram message on the socket. If `buf` is too small to hold
        // the message, it will be cut off.
        // TODO: Detect overflow. Longer messages are truncated and the TC bit is set in the header.
        let mut buf = [0; MAX_DATAGRAM_SIZE];
        let (amt, src) = socket.recv_from(&mut buf)?;

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        let buf = &buf[..amt];
        println!("stub {:?}", buf);

        {
            let remote_socket = UdpSocket::bind("127.0.0.1:34254")
                .expect("couldn't bind remote resolver to address");
            remote_socket
                .send_to(&[0; 10], "8.8.8.8:53")
                .expect("couldn't send data to remote");
            let mut remote_buf = [0; MAX_DATAGRAM_SIZE];
            let (remote_amt, _) = socket.recv_from(&mut remote_buf)?;
            let remote_buf = &remote_buf[..remote_amt];
            println!("remote {:?}", remote_buf);
        }

        socket.send_to(buf, &src)?;
    }
    Ok(())
}
