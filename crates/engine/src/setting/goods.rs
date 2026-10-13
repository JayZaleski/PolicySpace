//! Reading `[[good]]` sections (phase 1). The domain types live in [`crate::goods`]; this
//! module only turns setting entries into them.

use std::num::NonZeroU32;

use crate::goods::{Good, GoodKind, Goods, Perishes};

use super::defs::Definitions;
use super::reader::EntryReader;

/// Collects goods during phase 1.
#[derive(Default)]
pub(crate) struct GoodsReader {
    defs: Definitions<GoodKind>,
    /// Indexed by `GoodId::index`; pushed exactly when `defs.define` returns an ID.
    goods: Vec<Good>,
}

impl GoodsReader {
    /// Reads one `[[good]]` entry.
    pub fn read(&mut self, reader: &mut EntryReader<'_, '_>) {
        // Read every key before defining anything, even when an earlier one failed, so
        // that no key is falsely reported as unknown.
        let name = reader.required_name("name");

        let perishes_after = reader.optional_integer("perishes_after");
        let perishes = match perishes_after {
            None => Perishes::Never,
            Some(value) => {
                let after = u32::try_from(value.value).ok();
                match after.and_then(NonZeroU32::new) {
                    Some(n) => Perishes::After(n),
                    None => {
                        let message = if value.value < 1 {
                            "key 'perishes_after' in good must be at least 1"
                        } else {
                            "key 'perishes_after' in good is out of range"
                        };
                        reader.errors().at(&value.at, message);
                        Perishes::Never
                    }
                }
            }
        };

        let movable = reader.optional_bool("movable").is_none_or(|m| m.value);

        // Define the name even if a property was invalid: the load fails anyway, but a
        // later duplicate must still be caught, and a reference to this good must not
        // look like a reference to an unknown good.
        if let Some(name) = name
            && self.defs.define(&name, reader.errors()).is_some()
        {
            self.goods.push(Good { perishes, movable });
        }
    }

    /// The definitions, for resolving references to goods in phase 2.
    // Used by the section types that refer to goods (Step 05 on); until then unused.
    #[allow(dead_code)]
    pub fn defs(&self) -> &Definitions<GoodKind> {
        &self.defs
    }

    /// The finished list.
    pub fn finish(self) -> Goods {
        Goods::from_parts(self.defs.into_registry(), self.goods)
    }
}
