// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of ink!.
//
// ink! is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// ink! is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with ink!.  If not, see <http://www.gnu.org/licenses/>.

/// Trait implemented by contracts to make them testable.
///
/// The testability comes from converting `#[ink(constructor)]`
/// functions from `&mut self` methods into making them actual
/// Rust constructors: e.g. with signatures like `fn new() -> Self`.
pub trait InstantiateTestable: Sized {
    /// The test wrapper for the contract.
    type Wrapped: core::ops::Deref<Target = Self> + core::ops::DerefMut<Target = Self>;

    /// Creates a testable instantiation of the contract.
    fn instantiate() -> Self::Wrapped;
}
