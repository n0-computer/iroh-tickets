//! a basic example constructing & consuming the built-in endpoint ticket.
use std::str::FromStr;

use iroh::Endpoint;
use iroh_tickets::{Ticket, endpoint::EndpointTicket};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // build an endpoint that we can dial & be dialed on
    let endpoint = Endpoint::builder().bind().await?;

    // wait until we're online before getting the address of the endpoint
    endpoint.online().await;
    let addr = endpoint.addr();

    // create a ticket from the endpoint address
    let ticket = EndpointTicket::new(addr);

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

    let parsed_ticket = EndpointTicket::from_str(&ticket)?;
    println!("connecting to {:?}", parsed_ticket.endpoint_addr().id);
    let conn = endpoint_b
        .connect(parsed_ticket.endpoint_addr().clone(), b"/hello")
        .await?;

    println!("connected");
    conn.close(400u32.into(), b"all done!");
    println!("done");

    Ok(())
}
