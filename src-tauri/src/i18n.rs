//! Language of user-facing backend messages (errors, notes warnings).
//!
//! The frontend owns the language setting and reports it via the
//! `set_language` command; until then messages are in Russian.

#[cfg(not(test))]
static ENGLISH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_english(english: bool) {
    #[cfg(not(test))]
    ENGLISH.store(english, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    TEST_ENGLISH.set(english);
}

pub fn english() -> bool {
    #[cfg(not(test))]
    return ENGLISH.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    TEST_ENGLISH.get()
}

// Tests run in parallel threads; a per-thread language keeps them independent.
#[cfg(test)]
thread_local! {
    static TEST_ENGLISH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `tr!("Слайд {n}", "Slide {n}")`: formats the message in the current language.
macro_rules! tr {
    ($ru:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::english() { format!($en $(, $arg)*) } else { format!($ru $(, $arg)*) }
    };
}
pub(crate) use tr;
