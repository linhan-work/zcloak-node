// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod currency {
	pub type Balance = u128;
	/// The existential deposit. Set to 1/10 of its parent Relay Chain (v9010).
	pub const EXISTENTIAL_DEPOSIT: Balance = 10 * CENTS;

	pub const UNITS: Balance = 10_000_000_000;
	pub const DOLLARS: Balance = UNITS;
	pub const CENTS: Balance = UNITS / 100; // 100_000_000
	pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		// 1/10 of Polkadot v9010
		(items as Balance * 20 * DOLLARS + (bytes as Balance) * 100 * MILLICENTS) / 10
	}
}
