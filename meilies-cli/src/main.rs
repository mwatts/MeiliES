use std::net::ToSocketAddrs;

use futures::stream::Stream;
use log::error;
use structopt::StructOpt;
use tokio::prelude::*;

use meilies_client::{sub_connect, paired_connect};
use meilies::command::Command;

#[derive(Debug, StructOpt)]
#[structopt(name = "meilies-cli", about = "A basic cli for MeiliES.")]
struct Opt {
    /// Server hostname.
    #[structopt(short = "h", long = "hostname", default_value = "127.0.0.1")]
    hostname: String,

    /// Server port.
    #[structopt(short = "p", long = "port", default_value = "6480")]
    port: u16,

    /// Command and arguments that will be sent to the server.
    cmd_args: Vec<String>,
}

fn main() {
    let _ = stderrlog::new().color(stderrlog::ColorChoice::Never).verbosity(2).init();

    let opt = Opt::from_args();
    let addr = (opt.hostname.as_str(), opt.port);
    let addr = match addr.to_socket_addrs().map(|addrs| addrs.filter(|a| a.is_ipv4()).next()) {
        Ok(Some(addr)) => addr,
        Ok(None) => return error!("impossible to dns resolve addr; {:?}", addr),
        Err(e) => return error!("error parsing addr; {}", e),
    };

    let args = opt.cmd_args.into_iter().map(Into::into).collect();
    let command = match Command::from_args(args) {
        Ok(command) => command,
        Err(e) => return error!("{}", e),
    };

    let fut = match command {
        Command::Subscribe { streams } => {
            let fut = sub_connect(&addr)
                .map_err(|e| error!("{}", e))
                .and_then(|(mut ctrl, msgs)| {

                    for stream in streams {
                        ctrl.subscribe_to(stream);
                    }

                    msgs.for_each(move |msg| {
                        println!("{:?}", msg);
                        future::ok(())
                    })
                    .map_err(|e| error!("{:?}", e))
                })
                .and_then(|_| {
                    println!("Connection closed by the server");
                    Err(())
                });

            Box::new(fut) as Box<dyn Future<Item=(), Error=()> + Send>
        },
        Command::Publish { stream, event } => {
            let fut = paired_connect(&addr)
                .map_err(|e| error!("{}", e))
                .and_then(|conn| conn.publish(stream, event.0))
                .and_then(|_conn| future::ok(()))
                .map(|()| println!("Event sent to the stream"));

            Box::new(fut) as Box<dyn Future<Item=(), Error=()> + Send>
        }
    };

    tokio::run(fut);
}
