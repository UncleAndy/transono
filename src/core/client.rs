pub struct Client<P, T>
where
    P: Protocol,
    T: Transport,
{
    transport: T,
    protocol: P,
}

impl<P, T> Client<P, T>
where
    P: Protocol,
    T: Transport,
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

    pub fn protocol(&self) -> &P {
        &self.protocol
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub async fn send(
        &mut self,
        command: &P::Command,
    ) -> Result<()> {

        let json = self.protocol.encode(command)?;

        self.transport.send(json).await
    }

    pub async fn recv(
        &mut self,
    ) -> Result<P::Event> {

        let json = self.transport.recv().await?;

        self.protocol.decode(&json)
    }
}
