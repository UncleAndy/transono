pub struct Client<P, T>
where
    P: Protocol,
    T: Transport,
{
    protocol: P,
    transport: T,
}

impl<P, T> Client<P, T>
where
    P: Protocol,
    T: Transport,
{
    pub fn new(
        protocol: P,
        transport: T,
    ) -> Self {
        Self {
            protocol,
            transport,
        }
    }
}
