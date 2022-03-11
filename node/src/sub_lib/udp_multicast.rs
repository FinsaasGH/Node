// Copyright (c) 2019, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.
#[allow(unused_imports)]
use crossbeam_channel::unbounded;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
#[allow(unused_imports)]
use std::thread;

#[allow(dead_code)]
const MULTICAST_GROUP_ADDRESS: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 2);
const MCAST_PORT: u16 = 8888;
const MCAST_INTERFACE: Ipv4Addr = Ipv4Addr::UNSPECIFIED;

#[allow(dead_code)]
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

#[allow(dead_code)]
fn run_receiver() {
    let socket = create_socket();
    let mut buffer = [0; 64];
    (0..10).for_each(|x| {
        let message = format!("Test message {} for MASQ UDP multicast", x);
        match socket.recv_from(&mut buffer) {
            Ok((len, _remote_addr)) => {
                let data = &buffer[..len];
                let response = std::str::from_utf8(data).unwrap();

                eprintln!("{}: Received on receiver1: {:?}", x, response);
                assert_eq!(response, message)
            }
            Err(err) => {
                println!("client: had a problem: {}", err);
                panic!();
            }
        }
    })
}

#[allow(dead_code)]
fn run_sender() {
    let addr = &SockAddr::from(SocketAddr::new(MULTICAST_GROUP_ADDRESS.into(), MCAST_PORT));
    let socket = create_socket();
    (0..10).for_each(|x| {
        println!("sending multicast message to group");
        let message = format!("Test message {} for MASQ UDP multicast", x);
        socket
            .send_to(message.as_bytes(), &addr.as_socket().unwrap())
            .expect("could not send_to!");
    })
}

#[test]
fn singlecast_udp_test() {
    let (sender, receiver) = unbounded();
    thread::spawn(move || {
        receiver.recv().unwrap();
        run_sender()
    });
    sender.send(()).unwrap();
    run_receiver()
}

#[test]
fn multicast_udp_test() {
    let receiver1 = create_socket();
    let receiver2 = create_socket();
    let receiver3 = create_socket();
    let socket = create_socket();
    let mut buffer1 = [0; 64];
    let mut buffer2 = [0; 64];
    let mut buffer3 = [0; 64];
    let addr = &SockAddr::from(SocketAddr::new(MULTICAST_GROUP_ADDRESS.into(), MCAST_PORT));
    (0..10).for_each(|x| {
        println!("sending multicast message to group");
        let message = format!("Test message {} for MASQ UDP multicast", x);
        socket
            .send_to(message.as_bytes(), &addr.as_socket().unwrap())
            .expect("could not send_to!");
        match receiver1.recv_from(&mut buffer1) {
            Ok((len, _remote_addr)) => {
                let data = &buffer1[..len];
                let response = std::str::from_utf8(data).unwrap();

                eprintln!("{}: Received on receiver1: {:?}", x, response);
                assert_eq!(response, message)
            }
            Err(err) => {
                println!("client: had a problem: {}", err);
                panic!()
            }
        }
        match receiver2.recv_from(&mut buffer2) {
            Ok((len, _remote_addr)) => {
                let data = &buffer2[..len];
                let response = std::str::from_utf8(data).unwrap();

                eprintln!("{}: Received on receiver2: {:?}", x, response);
                assert_eq!(response, message)
            }
            Err(err) => {
                println!("client: had a problem: {}", err);
                panic!();
            }
        }
        match receiver3.recv_from(&mut buffer3) {
            Ok((len, _remote_addr)) => {
                let data = &buffer3[..len];
                let response = std::str::from_utf8(data).unwrap();

                eprintln!("{}: Received on receiver3: {:?}", x, response);
                assert_eq!(response, message)
            }
            Err(err) => {
                println!("client: had a problem: {}", err);
                panic!();
            }
        }
    })
}
