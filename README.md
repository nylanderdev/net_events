# net_events
A macro-based system for rapid development of event-based protocols on TCP using Rust enums.

Currently containing trace code from the project it was originally developed for. Very useful, but not currently a high priority so crate availability is TBA.

Example macro usage for creating a protocol:
```rust
protocol! {
    enum Event {
        Awaiting(u8) match as (players),
        Print(u32, u32, u8) match as (x, y, char),
        PrintStr(u32, u32, Vec<u8>) match as (x, y, str),
        Flush,
        Ping,
        KeyUp(u8) match as (key),
        KeyDown(u8) match as (key)
    }
}
```
Note that due to certain limitations of the Rust's current macro system, each non-empty enum needs a `match as` class, naming each field. In addition, only tuple-enums are supported.

The crate also provides a `Connection` type for each protocol, instances of which can be constructed from TcpStream's.

```rust
fn example(your_stream: TcpStream) {
    use SendResult::*;
    let send_res: SendResult = Connection::from_tcp_stream(your_stream)
        .unwrap()
        .send(&Event::Print(32, 56, b'?'));
    
    match send_res {
        Ok => (),
        Disconnected => (), // Other end has disconnected
        Invalid => () // Other end has sent an ill formed message and the connection should be terminated
    }
}
```
