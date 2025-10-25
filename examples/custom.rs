//! This example shows creating a custom ticket type that implements the Ticket
//! trait. It also uses a Variant, which addsa a version number to tickets for
//! backwards compatibility over time
use std::{
    collections::BTreeSet,
    fmt::{self, Display},
    net::SocketAddr,
    str::FromStr,
};

use iroh::{Endpoint, EndpointAddr, EndpointId, RelayUrl};
use iroh_tickets::{ParseError, Ticket};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomIrohTicket {
    // you can add whatever fields here you'd like
    node: EndpointAddr,
}

impl Display for CustomIrohTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Ticket::serialize(self))
    }
}

#[derive(Serialize, Deserialize)]
struct Variant0NodeAddr {
    endpoint_id: EndpointId,
    info: Variant0AddrInfo,
}

#[derive(Serialize, Deserialize)]
struct Variant0AddrInfo {
    relay_url: Option<RelayUrl>,
    direct_addresses: BTreeSet<SocketAddr>,
}

/// Wire format for [`NodeTicket`].
#[derive(Serialize, Deserialize)]
enum TicketWireFormat {
    Variant0(Variant0NodeTicket),
}

#[derive(Serialize, Deserialize)]
struct Variant0NodeTicket {
    addr: Variant0NodeAddr,
}

impl Ticket for CustomIrohTicket {
    // KIND is the constant that's added to the front of a serialized ticket
    // string. It should be a short, human readble string
    const KIND: &'static str = "zed";

    fn to_bytes(&self) -> Vec<u8> {
        let data = TicketWireFormat::Variant0(Variant0NodeTicket {
            addr: Variant0NodeAddr {
                endpoint_id: self.node.node_id,
                info: Variant0AddrInfo {
                    relay_url: self.node.relay_url.clone(),
                    direct_addresses: self.node.direct_addresses.clone(),
                },
            },
        });
        postcard::to_stdvec(&data).expect("postcard serialization failed")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
        let res: TicketWireFormat = postcard::from_bytes(bytes)?;
        let TicketWireFormat::Variant0(Variant0NodeTicket { addr: node }) = res;
        Ok(Self {
            addr: EndpointAddr {
                id: node.endpoint_id,
                relay_url: node.info.relay_url,
                addrs: node.info.direct_addresses,
            },
        })
    }
}

impl FromStr for CustomIrohTicket {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        iroh_tickets::Ticket::deserialize(s)
    }
}

impl CustomIrohTicket {
    /// Creates a new ticket.
    pub fn new(node: EndpointAddr) -> Self {
        Self { node }
    }

    /// The [`NodeAddr`] of the provider for this ticket.
    pub fn addr(&self) -> &EndpointAddr {
        &self.node
    }
}

impl From<CustomIrohTicket> for NodeAddr {
    /// Returns the addressing info from given ticket.
    fn from(ticket: CustomIrohTicket) -> Self {
        ticket.node
    }
}

impl Serialize for CustomIrohTicket {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            let CustomIrohTicket { node } = self;
            (node).serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for CustomIrohTicket {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Self::from_str(&s).map_err(serde::de::Error::custom)
        } else {
            let peer = Deserialize::deserialize(deserializer)?;
            Ok(Self::new(peer))
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // build an endpoint that we can dial & be dialed on
    let endpoint = Endpoint::builder().bind().await?;

    // wait until we're online before getting the address of the endpoint
    endpoint.online().await;
    let addr = endpoint.addr();

    // create a ticket from the endpoint address
    let ticket = CustomIrohTicket::new(addr);

    println!("ticket: {}", &ticket);
    println!("ticket parts: {:?}", &ticket);

    // convert the ticket to a string, in the real world this would get shared
    // usually the user will copy this string, and send it to anyone they want
    // to connect to them via something like a text message, email, twitter DM,
    // etc.
    let ticket = ticket.serialize();

    // build a second endpoint that will connect
    let endpoint_b = Endpoint::builder().bind().await?;
    endpoint_b.online().await;

    let parsed_ticket = CustomIrohTicket::from_str(&ticket)?;
    println!("connecting to {:?}", parsed_ticket.endpoint_addr().id);
    let conn = endpoint_b
        .connect(parsed_ticket.endpoint_addr().clone(), b"/hello")
        .await?;

    println!("connected");
    conn.close(400u32.into(), b"all done!");
    println!("done");

    Ok(())
}
