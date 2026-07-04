use crate::core::error::Result;
use crate::core::transport::TransportData;

pub trait Protocol: Send + Sync + 'static {
    type Command;
    type Event;

    const ENDPOINT: &'static str;

    fn encode(
        &self,
        command: &Self::Command,
    ) -> Result<TransportData>;

    fn decode(
        &self,
        data: TransportData,
    ) -> Result<Self::Event>;
}
