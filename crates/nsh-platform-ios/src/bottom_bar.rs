//! Independent hiding reasons for the system tab bar. Navigation owns the
//! visible stack's choice; the app owns `TabBar.hidden` across transitions.

use std::cell::Cell;

#[derive(Default)]
pub(crate) struct Visibility {
    navigation: Cell<bool>,
    app: Cell<bool>,
}

impl Visibility {
    pub(crate) fn set_navigation(&self, hidden: bool) {
        self.navigation.set(hidden);
    }

    pub(crate) fn set_app(&self, hidden: bool) {
        self.app.set(hidden);
    }

    pub(crate) fn hidden(&self) -> bool {
        self.navigation.get() || self.app.get()
    }
}

/// Whether a stack whose screens ask `hides` for the bar off screen (the
/// root first) keeps it there while the screen at `shown` is the one on
/// screen: UIKit's rule counts that screen and those below it. Screens above
/// it are leaving — `navigationController:willShowViewController:animated:`
/// can still list them for a pop — and do not count; `None` counts them all.
pub(crate) fn hidden_through(hides: &[bool], shown: Option<usize>) -> bool {
    let end = shown.map_or(hides.len(), |at| (at + 1).min(hides.len()));
    hides[..end].iter().any(|hides| *hides)
}

#[cfg(test)]
mod tests {
    use super::{Visibility, hidden_through};

    #[test]
    fn a_pop_counts_only_the_screens_up_to_the_one_shown() {
        // A hiding page over a root that does not hide, popped while the
        // stack still lists it.
        assert!(!hidden_through(&[false, true], Some(0)));
        assert!(hidden_through(&[false, true], Some(1)));
        // A hiding screen below the one shown still keeps the bar away.
        assert!(hidden_through(&[true, false], Some(1)));
        // Not in the stack: every screen counts.
        assert!(hidden_through(&[false, true], None));
        assert!(!hidden_through(&[], None));
    }

    #[test]
    fn pushes_and_pops_preserve_the_apps_hidden_prop() {
        let bar = Visibility::default();
        bar.set_app(true);
        // Ordinary push, hiding page pushed, then popped back to root.
        for hides in [false, true, false] {
            bar.set_navigation(hides);
            assert!(bar.hidden());
        }
        bar.set_app(false);
        assert!(!bar.hidden());
    }

    #[test]
    fn app_updates_preserve_the_visible_stacks_choice() {
        let bar = Visibility::default();
        bar.set_navigation(true);
        bar.set_app(true);
        bar.set_app(false);
        assert!(bar.hidden());
        // Switching to a retained root stack replaces the navigation reason.
        bar.set_navigation(false);
        assert!(!bar.hidden());
        bar.set_navigation(true);
        assert!(bar.hidden());
    }
}
