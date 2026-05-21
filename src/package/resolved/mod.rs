mod cask;
mod formula;

use std::{borrow::Cow, iter, sync::Arc};

use either::Either::{Left, Right};
use enum_dispatch::enum_dispatch;

pub(crate) use self::{cask::ResolvedCask, formula::ResolvedFormula};

use super::Packageable;

#[enum_dispatch]
pub(crate) enum ResolvedPackage {
    Formula(Arc<ResolvedFormula>),
    Cask(Arc<ResolvedCask>),
}

impl ResolvedPackage {
    pub(crate) fn iter(&self) -> impl Iterator<Item = Self> + use<> {
        match self {
            Self::Formula(formula) => {
                let formulae = formula.iter().map(Self::Formula);

                Left(formulae)
            },

            Self::Cask(cask) => {
                let cask = Arc::clone(cask);

                let casks = iter::once(cask).map(Self::Cask);

                Right(casks)
            },
        }
    }
}

#[cfg_attr(debug_assertions, expect(shadowing_supertrait_items))]
#[enum_dispatch(ResolvedPackage)]
pub(super) trait ResolvedPackageable: Packageable {
    fn version(&self) -> Cow<'_, str>;
}

impl<ResolvedPackage: ResolvedPackageable> ResolvedPackageable for Arc<ResolvedPackage> {
    #[cfg_attr(not(debug_assertions), expect(unconditional_recursion))]
    fn version(&self) -> Cow<'_, str> {
        #[cfg_attr(not(debug_assertions), expect(unused_variables))]
        #[expect(clippy::use_self)]
        let this = Arc::as_ref(self);

        #[cfg(debug_assertions)]
        #[cfg_attr(
            debug_assertions,
            expect(resolving_to_items_shadowing_supertrait_items)
        )]
        let version = this.version();

        #[cfg(not(debug_assertions))]
        let version = ResolvedPackageable::version(self);

        version
    }
}
