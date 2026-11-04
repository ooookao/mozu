//! Anchor adjustment and resolved-placement calculations.

use gpui::{Bounds, Pixels};

use super::PopoverSide;

pub(super) fn resolved_side_from_bounds(
    trigger_bounds: Bounds<Pixels>,
    content_bounds: Bounds<Pixels>,
    requested_side: PopoverSide,
) -> PopoverSide {
    match requested_side {
        PopoverSide::Top | PopoverSide::Bottom => {
            if content_bounds.center().y < trigger_bounds.center().y {
                PopoverSide::Top
            } else {
                PopoverSide::Bottom
            }
        }
        PopoverSide::Left | PopoverSide::InlineStart => {
            if content_bounds.center().x < trigger_bounds.center().x {
                PopoverSide::Left
            } else {
                PopoverSide::Right
            }
        }
        PopoverSide::Right | PopoverSide::InlineEnd => {
            if content_bounds.center().x > trigger_bounds.center().x {
                PopoverSide::Right
            } else {
                PopoverSide::Left
            }
        }
    }
}

pub(super) fn shifted_trigger_bounds(
    mut bounds: Bounds<Pixels>,
    side: PopoverSide,
    offset: Pixels,
) -> Bounds<Pixels> {
    match side {
        PopoverSide::Top | PopoverSide::Bottom => bounds.origin.x += offset,
        PopoverSide::Left
        | PopoverSide::Right
        | PopoverSide::InlineStart
        | PopoverSide::InlineEnd => bounds.origin.y += offset,
    }
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px, size};

    #[test]
    fn align_offset_moves_the_anchor_on_the_alignment_axis() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(30.0), px(40.0)));
        assert_eq!(
            shifted_trigger_bounds(bounds, PopoverSide::Bottom, px(3.0)).origin,
            point(px(13.0), px(20.0))
        );
        assert_eq!(
            shifted_trigger_bounds(bounds, PopoverSide::Right, px(3.0)).origin,
            point(px(10.0), px(23.0))
        );
    }

    #[test]
    fn resolved_side_tracks_the_final_position_after_flipping() {
        let trigger = Bounds::new(point(px(100.0), px(100.0)), size(px(40.0), px(20.0)));
        assert_eq!(
            resolved_side_from_bounds(
                trigger,
                Bounds::new(point(px(80.0), px(40.0)), size(px(80.0), px(50.0))),
                PopoverSide::Top,
            ),
            PopoverSide::Top
        );
        assert_eq!(
            resolved_side_from_bounds(
                trigger,
                Bounds::new(point(px(80.0), px(130.0)), size(px(80.0), px(50.0))),
                PopoverSide::Top,
            ),
            PopoverSide::Bottom
        );
    }

    #[test]
    fn resolved_side_stays_on_the_requested_axis_while_motion_overlaps_the_trigger() {
        let trigger = Bounds::new(point(px(100.0), px(100.0)), size(px(40.0), px(20.0)));

        assert_eq!(
            resolved_side_from_bounds(
                trigger,
                Bounds::new(point(px(138.0), px(100.0)), size(px(80.0), px(60.0))),
                PopoverSide::Right,
            ),
            PopoverSide::Right
        );
    }
}
