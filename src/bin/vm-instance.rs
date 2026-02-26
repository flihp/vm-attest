// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use clap_verbosity::{InfoLevel, Verbosity};

use log::debug;
use std::{net::TcpListener, path::PathBuf};
use vsock::{VMADDR_CID_HOST, VsockAddr};

use vm_attest_trait::{
    VmInstanceRotBuilder,
    socket::{VmInstanceRotSocketClientBuilder, VmInstanceTcpServer},
    vsock::VmInstanceRotVsockClientBuilder,
};

#[derive(Debug, Subcommand)]
enum SocketType {
    /// Connect to `vm-instance-rot` as a client on a unix domain socket
    Unix {
        // path to unix socket file
        sock: PathBuf,
    },
    /// Connect to `vm-instance-rot` as a client on a vsock
    Vsock {
        // port to listen on
        #[clap(default_value_t = 1024)]
        port: u32,
    },
}

/// This is a tool implements the minimal behavior we expect of the software
/// running in a virtual machine. It connects to the `vm-instance-rot` as a
/// client while accepting challenges from the `appraiser` over TCP.
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    // Dump debug output
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,

    /// Address used for server that listens for challenges
    #[clap(long, default_value_t = String::from("localhost:6666"))]
    address: String,

    #[clap(long, default_value_t = false)]
    retry: bool,

    #[command(subcommand)]
    socket_type: SocketType,
}

fn main() -> Result<()> {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    match args.socket_type {
        SocketType::Unix { sock } => {
            // fail early if the socket file doesn't exist
            if !sock.exists() {
                return Err(anyhow!("socket file missing"));
            }

            let builder = VmInstanceRotSocketClientBuilder::new(sock);
            let vm_instance_rot = builder.build()?;

            let challenge_listener = TcpListener::bind(&args.address)
                .context("bind to TCP socket")?;
            debug!("Listening on TCP address{:?}", &args.address);

            let server =
                VmInstanceTcpServer::new(challenge_listener, vm_instance_rot);
            Ok(server.run()?)
        }
        SocketType::Vsock { port } => {
            debug!("connecting to host vsock on port: {port}");
            let addr = VsockAddr::new(VMADDR_CID_HOST, port);

            let builder = VmInstanceRotVsockClientBuilder::new(addr);
            let vm_instance_rot = builder.build()?;

            debug!("binding to address: {}", &args.address);
            let challenge_listener = TcpListener::bind(&args.address)
                .context("bind to TCP socket")?;
            debug!("Listening on TCP address{:?}", &args.address);

            let server =
                VmInstanceTcpServer::new(challenge_listener, vm_instance_rot);
            Ok(server.run()?)
        }
    }
}
