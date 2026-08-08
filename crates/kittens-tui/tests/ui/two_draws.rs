use std::time::Duration;

use tokio::time::Instant;

use kittens_tui::Presenter;

// Exactly one Draw can exist at a time: the permit holds the presenter's
// exclusive borrow, so a second simultaneous draw is a compile error, not a
// runtime protocol state.
fn main() {
    let mut presenter = Presenter::new(Duration::ZERO);
    let now = Instant::now();
    let first = presenter.try_begin(now);
    let second = presenter.try_begin(now);
    drop((first, second));
}
