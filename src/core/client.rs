use crate::core::protocol::Protocol;
use crate::core::transport::Transport;
use crate::core::error::Result;

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

        let data = self.protocol.encode(command)?;

        self.transport.send(data.into()).await
    }

    pub async fn recv(
        &mut self,
    ) -> Result<P::Event> {
        let data = self.transport.recv().await?;

        self.protocol.decode(data.as_bytes())
    }
}
