//! Where things are on the screen.
//!
//! A terminal has no widget tree to ask, so drawing records what it put where
//! and pointer events are resolved against that list. Registering during the
//! draw rather than computing regions separately is what keeps the two from
//! disagreeing: if it was not drawn, it cannot be clicked.

use ratatui::layout::Rect;

/// Something the pointer can be over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// A row of the list, by index into `rows`.
    Row(usize),
    /// A count in the status strip.
    Facet(String),
    /// The grouping axis, in the rule.
    Axis,
    /// The sort column, in the rule.
    Sort,
    /// A key in the footer.
    Key(char),
    /// The detail pane.
    Detail,
}

/// What was drawn where, for the frame now on screen.
#[derive(Clone, Debug, Default)]
pub struct Hits {
    regions: Vec<(Rect, Hit)>,
}

impl Hits {
    /// Forget the previous frame.
    pub fn clear(&mut self) {
        self.regions.clear();
    }

    /// Record that something occupies this rectangle.
    ///
    /// A zero-sized rectangle is dropped: a widget that was laid out but had no
    /// room is not on the screen, and making it clickable would give the
    /// pointer a target nobody can see.
    pub fn add(&mut self, area: Rect, what: Hit) {
        if area.width > 0 && area.height > 0 {
            self.regions.push((area, what));
        }
    }

    /// What is under this cell.
    ///
    /// Later registrations win, because they were drawn on top.
    #[must_use]
    pub fn at(&self, column: u16, row: u16) -> Option<&Hit> {
        self.regions
            .iter()
            .rev()
            .find(|(area, _)| {
                column >= area.x
                    && column < area.x + area.width
                    && row >= area.y
                    && row < area.y + area.height
            })
            .map(|(_, what)| what)
    }
}

#[cfg(test)]
mod tests {
    use super::{Hit, Hits};
    use ratatui::layout::Rect;

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn a_cell_inside_a_region_finds_it() {
        let mut hits = Hits::default();
        hits.add(rect(0, 3, 40, 1), Hit::Row(7));
        assert_eq!(hits.at(10, 3), Some(&Hit::Row(7)));
    }

    #[test]
    fn a_cell_outside_every_region_finds_nothing() {
        let mut hits = Hits::default();
        hits.add(rect(0, 3, 40, 1), Hit::Row(7));
        assert_eq!(hits.at(10, 4), None);
        assert_eq!(hits.at(40, 3), None, "the right edge is exclusive");
    }

    #[test]
    fn something_drawn_on_top_wins() {
        let mut hits = Hits::default();
        hits.add(rect(0, 0, 80, 10), Hit::Detail);
        hits.add(rect(0, 0, 20, 1), Hit::Facet("wanted".to_owned()));
        assert_eq!(hits.at(5, 0), Some(&Hit::Facet("wanted".to_owned())));
    }

    #[test]
    fn a_region_with_no_room_is_not_clickable() {
        let mut hits = Hits::default();
        hits.add(rect(0, 0, 0, 1), Hit::Axis);
        hits.add(rect(0, 0, 10, 0), Hit::Sort);
        assert_eq!(hits.at(0, 0), None, "it was laid out and never drawn");
    }

    #[test]
    fn clearing_forgets_the_previous_frame() {
        let mut hits = Hits::default();
        hits.add(rect(0, 0, 10, 1), Hit::Row(1));
        hits.clear();
        assert_eq!(hits.at(0, 0), None, "the screen has moved on");
    }
}
