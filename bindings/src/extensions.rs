use cooklang::Extensions as OriginalExtensions;

/// Cooklang extensions control optional language features
///
/// Extensions are designed to be backward-compatible - canonical recipes parse
/// the same way regardless of which extensions are enabled. Extensions only
/// activate when you use their specific syntax.
///
/// See https://github.com/cooklang/cooklang-rs/blob/main/extensions.md
#[derive(uniffi::Record, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extensions {
    /// Enables component modifiers: @@recipe, @&reference, @-hidden, @?optional, @+new
    pub component_modifiers: bool,

    /// Enables component aliases: @white wine|wine{}
    pub component_alias: bool,

    /// Allows space instead of % in units (@water{1 L}) and enables unit compatibility checks
    pub advanced_units: bool,

    /// Enables special [mode] metadata to control parsing behavior
    pub modes: bool,

    /// Searches for temperatures and other inline quantities in text
    pub inline_quantities: bool,

    /// Enables range values: @eggs{2-4}, @water{200-300%ml}
    pub range_values: bool,

    /// Makes timers without time values invalid
    pub timer_requires_time: bool,

    /// Enables intermediate preparations: @&(~1)dough{}
    /// Automatically enables component_modifiers
    pub intermediate_preparations: bool,
}

impl From<Extensions> for OriginalExtensions {
    fn from(ext: Extensions) -> Self {
        let mut result = OriginalExtensions::empty();

        if ext.component_modifiers {
            result |= OriginalExtensions::COMPONENT_MODIFIERS;
        }
        if ext.component_alias {
            result |= OriginalExtensions::COMPONENT_ALIAS;
        }
        if ext.advanced_units {
            result |= OriginalExtensions::ADVANCED_UNITS;
        }
        if ext.modes {
            result |= OriginalExtensions::MODES;
        }
        if ext.inline_quantities {
            result |= OriginalExtensions::INLINE_QUANTITIES;
        }
        if ext.range_values {
            result |= OriginalExtensions::RANGE_VALUES;
        }
        if ext.timer_requires_time {
            result |= OriginalExtensions::TIMER_REQUIRES_TIME;
        }
        if ext.intermediate_preparations {
            result |= OriginalExtensions::INTERMEDIATE_PREPARATIONS;
        }

        result
    }
}

impl From<OriginalExtensions> for Extensions {
    fn from(ext: OriginalExtensions) -> Self {
        Extensions {
            component_modifiers: ext.contains(OriginalExtensions::COMPONENT_MODIFIERS),
            component_alias: ext.contains(OriginalExtensions::COMPONENT_ALIAS),
            advanced_units: ext.contains(OriginalExtensions::ADVANCED_UNITS),
            modes: ext.contains(OriginalExtensions::MODES),
            inline_quantities: ext.contains(OriginalExtensions::INLINE_QUANTITIES),
            range_values: ext.contains(OriginalExtensions::RANGE_VALUES),
            timer_requires_time: ext.contains(OriginalExtensions::TIMER_REQUIRES_TIME),
            intermediate_preparations: ext.contains(OriginalExtensions::INTERMEDIATE_PREPARATIONS),
        }
    }
}

/// Create an Extensions configuration with all extensions enabled
///
/// This is the recommended default as extensions are backward-compatible.
/// Canonical recipes parse the same way, and you get extended features when
/// you use their specific syntax.
#[uniffi::export]
pub fn extensions_all() -> Extensions {
    OriginalExtensions::all().into()
}

/// Create an Extensions configuration with no extensions enabled
///
/// This matches the canonical Cooklang spec without any extensions.
/// Only use this if you need strict canonical-only parsing.
#[uniffi::export]
pub fn extensions_empty() -> Extensions {
    OriginalExtensions::empty().into()
}

/// Create an Extensions configuration for maximum compatibility
///
/// Enables all extensions except TIMER_REQUIRES_TIME to maximize
/// compatibility with other cooklang parsers.
#[uniffi::export]
pub fn extensions_compat() -> Extensions {
    OriginalExtensions::COMPAT.into()
}

/// Create a custom Extensions configuration
#[uniffi::export]
pub fn extensions_custom(
    component_modifiers: bool,
    component_alias: bool,
    advanced_units: bool,
    modes: bool,
    inline_quantities: bool,
    range_values: bool,
    timer_requires_time: bool,
    intermediate_preparations: bool,
) -> Extensions {
    Extensions {
        component_modifiers,
        component_alias,
        advanced_units,
        modes,
        inline_quantities,
        range_values,
        timer_requires_time,
        intermediate_preparations,
    }
}
