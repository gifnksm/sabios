use super::{font, Color, Offset, Point, Rectangle, Size};

pub(crate) trait Draw {
    fn size(&self) -> Size<i32>;
    fn draw(&mut self, p: Point<i32>, c: Color);
    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>);

    fn area(&self) -> Rectangle<i32> {
        Rectangle::new(Point::new(0, 0), self.size())
    }

    fn fill_rect(&mut self, rect: Rectangle<i32>, c: Color) {
        for p in rect.points() {
            self.draw(p, c);
        }
    }

    fn draw_rect(&mut self, rect: Rectangle<i32>, c: Color) {
        if rect.size.x == 0 || rect.size.y == 0 {
            return;
        }

        for x in rect.x_range() {
            self.draw(Point::new(x, rect.y_start()), c);
            self.draw(Point::new(x, rect.y_end() - 1), c);
        }
        for y in rect.y_range() {
            self.draw(Point::new(rect.x_start(), y), c);
            self.draw(Point::new(rect.x_end() - 1, y), c);
        }
    }

    fn draw_byte_char(&mut self, pos: Point<i32>, byte: u8, color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        font::draw_byte_char(self, pos, byte, color)
    }

    fn draw_byte_str(&mut self, pos: Point<i32>, bytes: &[u8], color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        font::draw_byte_str(self, pos, bytes, color)
    }

    fn draw_char(&mut self, pos: Point<i32>, ch: char, color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        font::draw_char(self, pos, ch, color)
    }

    fn draw_str(&mut self, pos: Point<i32>, s: &str, color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        font::draw_str(self, pos, s, color)
    }

    fn draw_box(
        &mut self,
        area: Rectangle<i32>,
        background: Color,
        border_top_left: Color,
        border_bottom_right: Color,
    ) {
        // fill main box
        self.fill_rect(
            Rectangle::new(area.pos + Offset::new(1, 1), area.size - Offset::new(2, 2)),
            background,
        );

        // draw border lines
        self.fill_rect(
            Rectangle::new(area.pos, Size::new(area.size.x, 1)),
            border_top_left,
        );
        self.fill_rect(
            Rectangle::new(area.pos, Size::new(1, area.size.y)),
            border_top_left,
        );
        self.fill_rect(
            Rectangle::new(
                area.pos + Offset::new(0, area.size.y),
                Size::new(area.size.x, 1),
            ),
            border_bottom_right,
        );
        self.fill_rect(
            Rectangle::new(
                area.pos + Offset::new(area.size.x, 0),
                Size::new(1, area.size.y),
            ),
            border_bottom_right,
        );
    }
}
static_assertions::assert_obj_safe!(Draw);
