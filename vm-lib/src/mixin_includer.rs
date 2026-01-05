// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use crate::{
    runtime_value::{RuntimeValue, mixin::Mixin},
    symbol::Symbol,
};

#[derive(Default)]
pub struct MixinIncluder {
    mixins: Vec<Mixin>,
}

impl MixinIncluder {
    pub fn load_named_value(&self, name: Symbol) -> Option<RuntimeValue> {
        self.mixins
            .iter()
            .rev()
            .find_map(|mixin| mixin.load_named_value(name))
    }

    pub fn include(&mut self, mixin: Mixin) {
        self.mixins.push(mixin);
    }

    pub fn contains(&self, mixin: &Mixin) -> bool {
        for m in &self.mixins {
            if m.isa_mixin(mixin) {
                return true;
            }
        }
        false
    }

    pub fn list_attributes(&self) -> HashSet<Symbol> {
        let mut attrs = HashSet::new();
        for mixin in &self.mixins {
            attrs.extend(mixin.list_attributes());
        }
        attrs
    }
}
