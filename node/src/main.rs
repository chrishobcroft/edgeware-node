// Copyright 2018 Commonwealth Labs, Inc.
// This file is part of Edgeware.

// Edgeware is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Edgeware is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Edgeware.  If not, see <http://www.gnu.org/licenses/>

//! Substrate Node Template CLI library.

#![warn(missing_docs)]

extern crate edgeware_cli as cli;
extern crate ctrlc;
extern crate futures;

use cli::VersionInfo;
use futures::sync::oneshot;
use futures::{future, Future};
use std::cell::RefCell;

// handles ctrl-c
struct Exit;
impl edgeware_cli::IntoExit for Exit {
	type Exit = future::MapErr<oneshot::Receiver<()>, fn(oneshot::Canceled) -> ()>;
	fn into_exit(self) -> Self::Exit {
		// can't use signal directly here because CtrlC takes only `Fn`.
		let (exit_send, exit) = oneshot::channel();

		let exit_send_cell = RefCell::new(Some(exit_send));
		ctrlc::set_handler(move || {
			if let Some(exit_send) = exit_send_cell.try_borrow_mut().expect("signal handler not reentrant; qed").take() {
				exit_send.send(()).expect("Error sending exit notification");
			}
		}).expect("Error setting Ctrl-C handler");

		exit.map_err(drop)
	}
}

fn main() {
	let version = VersionInfo {
		name: "Edgeware",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "edgeware",
		author: "Commonwealth Labs <hello@commonwealth.im>",
		description: "Edgeware Client Node",
		support_url: "https://github.com/hicommonwealth/edgeware-node/issues/new",
	};

	if let Err(e) = edgeware_cli::run(::std::env::args(), Exit, version) {
		eprintln!("Error starting the node: {}\n\n{:?}", e, e);
		std::process::exit(1)
	}
}
