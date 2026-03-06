use std::io::{BufReader, BufWriter};

use dap::prelude::*;

fn main() {
    let input = BufReader::new(std::io::stdin());
    let output = BufWriter::new(std::io::stdout());
    let server = Server::new(input, output);
    let mut dap_server = writ_dap::server::DapServer::new(server);
    dap_server.run();
}
