use crate::{
    framebuffer,
    graphics::{Color, Draw, Point, Rectangle, Size},
    layer::{self, EventSender, Layer, LayerBuffer, LayerId, WindowEvent},
    prelude::*,
    sync::mpsc,
    triple_buffer::{self, Producer},
};
use futures_util::StreamExt;

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
        let screen_info = *framebuffer::info();
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
        self.producer.with_buffer(|buffer| {
            buffer.clone_from(&self.buffer);
        });
        self.producer.store();
        self.event_tx.draw_layer(self.layer_id).await
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
        self.buffer.draw(p, c);
    }

    fn move_area(&mut self, offset: Point<i32>, src: Rectangle<i32>) {
        self.buffer.move_area(offset, src);
    }
}
