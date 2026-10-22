// Reference rust implementation of AluVM (arithmetic logic unit virtual machine).
// To find more on AluVM please check <https://aluvm.org>
//
// SPDX-License-Identifier: Apache-2.0
//
// Written in 2021-2024 by
//     Dr Maxim Orlovsky <orlovsky@ubideco.org>
//
// Copyright (C) 2021-2022 LNP/BP Standards Association. All rights reserved.
// Copyright (C) 2023-2024 UBIDECO Labs,
//     Institute for Distributed and Cognitive Computing, Switzerland.
//     All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Alu virtual machine

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::boxed::Box;
use core::marker::PhantomData;

use crate::isa::{Instr, InstructionSet, ReservedOp};
use crate::library::{Lib, LibId, LibSite};
use crate::reg::CoreRegs;

/// Alu virtual machine providing single-core execution environment
#[derive(Clone, Debug, Default)]
pub struct Vm<Isa = Instr<ReservedOp>>
where Isa: InstructionSet
{
    /// A set of registers
    pub registers: Box<CoreRegs>,

    phantom: PhantomData<Isa>,
}

/// Runtime for program execution.
impl<Isa> Vm<Isa>
where Isa: InstructionSet
{
    /// Constructs new virtual machine instance.
    pub fn new() -> Self {
        Self {
            registers: Box::default(),
            phantom: Default::default(),
        }
    }

    /// Executes the program starting from the provided entry point.
    ///
    /// # Returns
    ///
    /// Value of the `st0` register at the end of the program execution, which is `false` if the
    /// program enters a library `lib_resolver` does not provide.
    pub fn exec<'prog>(
        &mut self,
        entry_point: LibSite,
        lib_resolver: impl Fn(LibId) -> Option<&'prog Lib>,
        context: &Isa::Context<'_>,
    ) -> bool {
        let mut call = Some(entry_point);
        while let Some(site) = call {
            if let Some(lib) = lib_resolver(site.lib) {
                call = lib.exec::<Isa>(site.pos, &mut self.registers, context);
            } else {
                self.registers.st0 = false;
                call = None;
            };
        }
        self.registers.st0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::isa::ControlFlowOp::{self, *};

    fn lib(code: impl IntoIterator<Item = ControlFlowOp>) -> Lib {
        let code = code.into_iter().map(Instr::<ReservedOp>::ControlFlow).collect::<Vec<_>>();
        Lib::assemble(&code).unwrap()
    }

    fn run(entry: &Lib, libs: &[&Lib]) -> bool {
        let mut vm = Vm::<Instr>::new();
        let resolver = |id| libs.iter().copied().chain([entry]).find(|lib| lib.id() == id);
        vm.exec(LibSite::with(0, entry.id()), resolver, &())
    }

    #[test]
    fn routine_returns_after_itself() {
        assert!(!run(&lib([Routine(4), Fail, Ret]), &[]));
        assert!(run(&lib([Routine(4), Ret, Ret]), &[]));
    }

    #[test]
    fn call_returns_after_itself() {
        let helper = lib([Ret]);
        let site = LibSite::with(0, helper.id());
        assert!(!run(&lib([Call(site), Fail, Ret]), &[&helper]));
        assert!(run(&lib([Call(site), Ret, Ret]), &[&helper]));
    }

    #[test]
    fn call_into_unresolved_lib_fails() {
        let helper = lib([Ret]);
        let site = LibSite::with(0, helper.id());
        assert!(!run(&lib([Call(site), Ret]), &[]));
        assert!(!run(&lib([Exec(site)]), &[]));
    }
}
