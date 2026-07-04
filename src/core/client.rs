pub struct Client<P, T>
where
    T: Transport,
    P: Protocol,
{
    transport: T,
    protocol: P,
}

impl<T, P> Client<T, P>
where
    T: Transport,
    P: Protocol,
{
    pub fn new(
        transport: T,
        protocol: P,
    ) -> Self {
        Self {
            transport,
            protocol,
        }
    }

    pub async fn send_protocol(
        &mut self,
        command: &P::Command,
    ) -> Result<()> {

        let json = self.protocol.encode(command)?;

        self.transport.send(json).await
    }

    pub async fn recv_protocol(
        &mut self,
    ) -> Result<P::Event> {

        let json = self.transport.recv().await?;

        self.protocol.decode(&json)
    }
}
