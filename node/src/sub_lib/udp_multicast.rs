// Copyright (c) 2019, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket, SocketAddrV4};
use std::time::Duration;
use std::mem::MaybeUninit;
use std::{thread};
use std::str::FromStr;
use std::sync::mpsc::channel;

const MULTICAST_GROUP_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 2);
const MCAST_PORT: u16 = 8888;
const MCAST_INTERFACE: Ipv4Addr = Ipv4Addr::UNSPECIFIED;

#[test]
fn udp_multicast_receiver() {
    let socket = create_socket();
    let mut buffer = [0; 64];

    match socket.recv_from(&mut buffer) {
        Err(e) => eprintln!("{}: {}", "could not receive from socket", e),
        Ok(res) => println!(
                "{}: Received: {} from {:?}",
                x,
                std::str::from_utf8(&buffer).unwrap(),
                res.1
            ),
        };
        if x == 10 {
            socket
                .leave_multicast_v4(&MULTICAST_GROUP_ADDRESS, &MCAST_INTERFACE)
                .unwrap();
            socket
                .set_read_timeout(Some(Duration::from_millis(1)))
                .unwrap();
            eprintln!("we left the multicast group")
        }
}

fn udp_multicast_sender() {
    let addr = &SockAddr::from(SocketAddr::new(MULTICAST_GROUP_ADDRESS.into(),MCAST_PORT));
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("could not create new socket");
    let socket:UdpSocket = socket.into();
    (0..10).for_each(|x|{
        println!("sending multicast message to group");
        let message = format!("Test message {} for MASQ UDP multicast",x);
        socket.send_to(message.as_bytes(), &addr.as_socket().unwrap()).expect("could not send_to!");
        thread::sleep(Duration::from_millis(10));
    })
}

fn create_socket() -> UdpSocket {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
        .expect("could not create socket!");
    #[cfg(not(target_os = "windows"))]
    socket.set_reuse_port(true).unwrap();
    socket.set_reuse_address(true).unwrap();
    socket
        .join_multicast_v4(&MULTICAST_GROUP_ADDRESS, &MCAST_INTERFACE)
        .unwrap();
    socket
        .bind(&SockAddr::from(SocketAddr::new(
            IpAddr::from(MCAST_INTERFACE),
            MCAST_PORT,
        )))
        .unwrap();
    let socket: UdpSocket = socket.into();
    socket
}

fn run_receiver() {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("could not create socket!");
    #[cfg(not(target_os = "windows"))]
    socket.set_reuse_port(true).unwrap();
    socket.set_reuse_address(true).unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(10))).expect("could not set read timeout");
    socket.join_multicast_v4(&MULTICAST_GROUP_ADDRESS, &MCAST_INTERFACE).expect("could not join multicast group");
    socket.bind(&SockAddr::from(SocketAddr::new(IpAddr::from(MCAST_INTERFACE), MCAST_PORT))).expect("could not bind to address");
    let socket:UdpSocket = socket.into();
    let mut buffer = [0;64];
    (0..10).for_each(|x| {
        socket.recv_from(&mut buffer).expect("could not receive from socket");
        eprintln!("{}: Received: {}", x, std::str::from_utf8(&buffer).unwrap())
    }
    )
}

fn run_sender() {
    let addr = &SockAddr::from(SocketAddr::new(MULTICAST_GROUP_ADDRESS.into(),MCAST_PORT));
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).expect("could not create new socket");
    let socket:UdpSocket = socket.into();
    (0..10).for_each(|x|{
        println!("sending multicast message to group");
        let message = format!("Test message {} for MASQ UDP multicast",x);
        socket.send_to(message.as_bytes(), &addr.as_socket().unwrap()).expect("could not send_to!");
        thread::sleep(Duration::from_secs(1));
    })
}

fn server_simple() {
    let socket = UdpSocket::bind("0.0.0.0:8888").unwrap();
    let mut buf = [0u8; 65535];
    let multi_addr = Ipv4Addr::new(234, 2, 2, 2);
    let inter = Ipv4Addr::new(0,0,0,0);
    socket.join_multicast_v4(&multi_addr,&inter).unwrap();

    let (amt, src) = socket.recv_from(&mut buf).unwrap();
    println!("received {} bytes from {:?}", amt, src);
}

fn client_simple() {
    let socket = UdpSocket::bind("0.0.0.0:9999").unwrap();
    let buf = [1u8; 15000];
    let count = 3;

    loop {
        socket.send_to(&buf[0..count], "234.2.2.2:8888").unwrap();
        thread::sleep(Duration::from_millis(1000));
    }
}

fn server_3() {
    let socket = Socket::new(
        Domain::IPV4,
        Type::DGRAM,
        Some(Protocol::UDP),
    ).unwrap();

    let socket_2 = socket2::SockAddr::from(SocketAddrV4::from_str("0.0.0.0:8888").unwrap());
    socket.bind(&socket_2).unwrap();
    let mut buf = [MaybeUninit::zeroed(); 65535];

    let (amt, src) = socket.recv_from(&mut buf).unwrap();
    println!("received {} bytes from {:?}", amt, src);
}

fn client_3() {
    let socket = UdpSocket::bind("0.0.0.0:9999").unwrap();
    let buf = [1u8; 15000];
    let count = 15;

    loop {
        socket.send_to(&buf[0..count], "224.0.0.251:8888").unwrap();
        thread::sleep(Duration::from_millis(1000));
    }
}

#[test]
fn test(){
    let (sender,receiver) = channel();
    thread::spawn(move ||{
        receiver.recv().unwrap();
        run_sender()
    });
    sender.send(()).unwrap();
    run_receiver()
}

#[test]
fn test_simple(){
    let (sender,receiver) = channel();
    thread::spawn(move ||{
        receiver.recv().unwrap();
        client_simple()
    });
    sender.send(()).unwrap();
    server_simple()
}

#[test]
fn test_3(){
    let (sender,receiver) = channel();
    thread::spawn(move ||{
        receiver.recv().unwrap();
        client_3()
    });
    sender.send(()).unwrap();
    server_3()
}
