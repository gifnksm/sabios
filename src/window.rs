use crate::{
    graphics::{Color, Draw, Point, Rectangle, ScreenInfo, Size},
    keyboard::KeyboardEvent,
    layer::{self, EventSender, Layer, LayerBuffer, LayerId},
    prelude::*,
    sync::mpsc,
    triple_buffer::{self, Producer},
};

#[derive(Debug)]
pub(crate) enum WindowEvent {
    Activated,
    Deactivated,
    Keyboard(KeyboardEvent),
}

#[derive(Debug, Clone)]
pub(crate) struct Builder {
    pos: Option<Point<i32>>,
    size: Size<i32>,
    transparent_color: Option<Color>,
    height: Option<usize>,
    draggable: Option<bool>,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Self {
            pos: None,
            size: Size::new(0, 0),
            transparent_color: None,
            height: None,
            draggable: None,
        }
    }

    pub(crate) fn pos(&mut self, pos: Point<i32>) -> &mut Self {
        self.pos = Some(pos);
        self
    }

    pub(crate) fn size(&mut self, size: Size<i32>) -> &mut Self {
        self.size = size;
        self
    }

    pub(crate) fn transparent_color(&mut self, tc: Option<Color>) -> &mut Self {
        self.transparent_color = tc;
        self
    }

    pub(crate) fn height(&mut self, height: usize) -> &mut Self {
        self.height = Some(height);
        self
    }

    pub(crate) fn draggable(&mut self, draggable: bool) -> &mut Self {
        self.draggable = Some(draggable);
        self
    }

    pub(crate) fn build(&mut self) -> Result<Window> {
        let screen_info = ScreenInfo::get();
        let mut buffer = LayerBuffer::new(self.size, screen_info)?;
        buffer.set_transparent_color(self.transparent_color);

        let (producer, consumer) = triple_buffer::new(buffer.clone());
        let (tx, rx) = mpsc::channel(100);
        let mut layer = Layer::new(consumer, tx);
        let layer_id = layer.id();
        let event_tx = layer::event_tx();

        if let Some(pos) = self.pos {
            layer.move_to(pos);
        }

        if let Some(draggable) = self.draggable {
            layer.set_draggable(draggable);
        }

        event_tx.register(layer)?;

        if let Some(height) = self.height {
            event_tx.set_height(layer_id, height)?;
        }

        Ok(Window {
            layer_id,
            event_tx,
            buffer,
            producer,
            rx,
            redraw_area: RedrawArea::new(self.size),
        })
    }
}

#[derive(Debug)]
pub(crate) struct Window {
    layer_id: LayerId,
    event_tx: EventSender,
    buffer: LayerBuffer,
    producer: Producer<LayerBuffer>,
    rx: mpsc::Receiver<WindowEvent>,
    redraw_area: RedrawArea,
}

impl Window {
    pub(crate) fn builder() -> Builder {
        Builder::new()
    }

    pub(crate) fn layer_id(&self) -> LayerId {
        self.layer_id
    }

    pub(crate) async fn move_to(&self, pos: Point<i32>) -> Result<()> {
        self.event_tx.move_to(self.layer_id, pos).await
    }

    pub(crate) async fn flush(&mut self) -> Result<()> {
        if let Some(redraw_area) = self.redraw_area.take() {
            self.producer.with_buffer(|buffer| {
                buffer.clone_from(&self.buffer);
            });
            self.producer.store();
            self.event_tx.draw_layer(self.layer_id, redraw_area).await?;
        }
        Ok(())
    }

    pub(crate) async fn recv_event(&mut self) -> Option<WindowEvent> {
        self.rx.next().await
    }
}

impl Draw for Window {
    fn size(&self) -> Size<i32> {
        self.buffer.size()
    }

    fn draw(&mut self, p: Point<i32>, c: Color) {
        self.redraw_area.add_point(p);
        self.buffer.draw(p, c);
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        self.redraw_area.add_rect(src + offset);
        self.buffer.move_area(offset, src);
    }

    // implement some default methods for faster redraw area computation
    fn fill_rect(&mut self, rect: Rectangle<i32>, c: Color) {
        self.redraw_area.add_rect(rect);
        self.buffer.fill_rect(rect, c)
    }

    fn draw_rect(&mut self, rect: Rectangle<i32>, c: Color) {
        self.redraw_area.add_rect(rect);
        self.buffer.draw_rect(rect, c)
    }

    fn draw_byte_char(&mut self, pos: Point<i32>, byte: u8, color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        let rect = self.buffer.draw_byte_char(pos, byte, color);
        self.redraw_area.add_rect(rect);
        rect
    }

    fn draw_byte_str(&mut self, pos: Point<i32>, bytes: &[u8], color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        let rect = self.buffer.draw_byte_str(pos, bytes, color);
        self.redraw_area.add_rect(rect);
        rect
    }

    fn draw_char(&mut self, pos: Point<i32>, ch: char, color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        let rect = self.buffer.draw_char(pos, ch, color);
        self.redraw_area.add_rect(rect);
        rect
    }

    fn draw_str(&mut self, pos: Point<i32>, s: &str, color: Color) -> Rectangle<i32>
    where
        Self: Sized,
    {
        let rect = self.buffer.draw_str(pos, s, color);
        self.redraw_area.add_rect(rect);
        rect
    }

    fn draw_box(
        &mut self,
        area: Rectangle<i32>,
        background: Color,
        border_top_left: Color,
        border_bottom_right: Color,
    ) {
        self.redraw_area.add_rect(area);
        self.buffer
            .draw_box(area, background, border_top_left, border_bottom_right);
    }
}

#[derive(Debug)]
struct RedrawArea {
    redraw_area: Option<Rectangle<i32>>,
    draw_area: Rectangle<i32>,
}

impl RedrawArea {
    fn new(size: Size<i32>) -> Self {
        Self {
            redraw_area: None,
            draw_area: Rectangle::new(Point::new(0, 0), size),
        }
    }

    fn take(&mut self) -> Option<Rectangle<i32>> {
        self.redraw_area.take()
    }

    fn add_rect(&mut self, area: Rectangle<i32>) {
        if let Some(area) = self.draw_area & area {
            match &mut self.redraw_area {
                Some(redraw_area) => *redraw_area = *redraw_area | area,
                None => self.redraw_area = Some(area),
            }
        }
    }

    fn add_point(&mut self, p: Point<i32>) {
        self.add_rect(Rectangle::new(p, Size::new(1, 1)));
    }
}
