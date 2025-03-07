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

use crate::{
    env2::{
        call::{
            state,
            CallData,
            Selector,
        },
        errors::CallError,
        Env,
        EnvAccessMut,
        EnvTypes,
    },
    memory::vec::Vec,
};
use core::marker::PhantomData;

/// Represents a return type.
///
/// Used as a marker type to differentiate at compile-time between invoke and evaluate.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ReturnType<T>(PhantomData<fn() -> T>);

/// The final parameters to the cross-contract call.
pub struct CallParams<E, R>
where
    E: EnvTypes,
{
    /// The account ID of the to-be-called smart contract.
    callee: E::AccountId,
    /// The maximum gas costs allowed for the call.
    gas_limit: u64,
    /// The transferred value for the call.
    value: E::Balance,
    /// The expected return type.
    return_type: PhantomData<ReturnType<R>>,
    /// The already encoded call data respecting the ABI.
    call_data: CallData,
}

/// Builds up a call.
pub struct CallBuilder<E, R, Seal>
where
    E: EnvTypes,
{
    /// The current parameters that have been built up so far.
    params: CallParams<E, R>,
    /// Seal state.
    seal: PhantomData<Seal>,
}

impl<E, R> CallParams<E, R>
where
    E: EnvTypes,
{
    /// The code hash of the contract.
    pub fn callee(&self) -> &E::AccountId {
        &self.callee
    }

    /// The gas limit for the contract instantiation.
    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }
    /// The endowment for the instantiated contract.
    pub fn endowment(&self) -> &E::Balance {
        &self.value
    }

    /// The raw encoded input data.
    pub fn input_data(&self) -> &CallData {
        &self.call_data
    }
}

impl<E, R> CallParams<E, R>
where
    E: EnvTypes,
    E::Balance: Default,
{
    /// Creates the default set of parameters for the cross-contract call.
    fn new(callee: E::AccountId, selector: Selector) -> Self {
        Self {
            callee,
            gas_limit: 0,
            value: E::Balance::default(),
            return_type: PhantomData,
            call_data: CallData::new(selector),
        }
    }

    /// Returns a builder for a cross-contract call that might return data.
    pub fn eval(
        callee: E::AccountId,
        selector: Selector,
    ) -> CallBuilder<E, ReturnType<R>, state::Unsealed> {
        CallBuilder {
            params: CallParams::new(callee, selector),
            seal: Default::default(),
        }
    }

    /// Returns a builder for a cross-contract call that cannot return data.
    ///
    /// Prefer this over [`eval`] if possible since it is the more efficient operation.
    pub fn invoke(
        callee: E::AccountId,
        selector: Selector,
    ) -> CallBuilder<E, (), state::Unsealed> {
        CallBuilder {
            params: CallParams::new(callee, selector),
            seal: Default::default(),
        }
    }
}

impl<E, R, Seal> CallBuilder<E, R, Seal>
where
    E: EnvTypes,
{
    /// Sets the maximumly allowed gas costs for the call.
    pub fn gas_limit(mut self, gas_limit: u64) -> Self {
        self.params.gas_limit = gas_limit;
        self
    }

    /// Sets the value transferred upon the execution of the call.
    pub fn value(mut self, value: E::Balance) -> Self {
        self.params.value = value;
        self
    }
}

impl<E, R> CallBuilder<E, R, state::Unsealed>
where
    E: EnvTypes,
{
    /// Pushes an argument to the inputs of the call.
    pub fn push_arg<A>(mut self, arg: &A) -> Self
    where
        A: scale::Encode,
    {
        self.params.call_data.push_arg(arg);
        self
    }

    /// Seals the call builder to prevent further arguments.
    pub fn seal(self) -> CallBuilder<E, R, state::Sealed> {
        CallBuilder {
            params: self.params,
            seal: Default::default(),
        }
    }
}

impl<E, R, Seal> CallBuilder<E, ReturnType<R>, Seal>
where
    E: Env,
    R: scale::Decode,
{
    /// Fires the call to the remote smart contract.
    /// Returns the returned data back to the caller.
    ///
    /// # Note
    ///
    /// Prefer using the [`fire_using`] method whenever possible
    /// since it is more efficient.
    pub fn fire(self) -> Result<R, CallError>
    where
        R: scale::Decode,
    {
        E::eval_contract(&mut Vec::new(), &self.params).map_err(|_| CallError)
    }

    /// Fires the call to the smart contract and returns
    /// the return value of the call back to the caller.
    ///
    /// # Note
    ///
    /// Uses the provided environmental access in order to
    /// dispatch the call which is more efficient than the
    /// [`fire`] method.
    pub fn fire_using(self, env: &mut EnvAccessMut<E>) -> Result<R, CallError>
    where
        R: scale::Decode,
    {
        env.eval_contract(&self.params).map_err(|_| CallError)
    }
}

impl<E, Seal> CallBuilder<E, (), Seal>
where
    E: Env,
{
    /// Fires the cross-call to the smart contract.
    ///
    /// # Note
    ///
    /// Prefer using the [`fire_using`] method whenever possible
    /// since it is more efficient.
    pub fn fire(self) -> Result<(), CallError> {
        E::invoke_contract(&mut Vec::new(), &self.params).map_err(|_| CallError)
    }

    /// Fires the cross-call to the smart contract.
    ///
    /// # Note
    ///
    /// Uses the provided environmental access in order to
    /// dispatch the call which is more efficient than the
    /// [`fire`] method.
    pub fn fire_using(self, env: &mut EnvAccessMut<E>) -> Result<(), CallError> {
        env.invoke_contract(&self.params).map_err(|_| CallError)
    }
}
