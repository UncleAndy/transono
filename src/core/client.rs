pub struct Client<P: Protocol> {

    ws: WebSocket,

    protocol: P,
}
