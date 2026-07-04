pub struct Client<P, T>
where
    P: Protocol,
    T: Transport,
{
    protocol: P,
    transport: T,
}
