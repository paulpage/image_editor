extern crate sdl2;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Texture, WindowCanvas};
use sdl2::surface::Surface;

extern crate rusttype;
use rusttype::{point, FontCollection, Scale};

extern crate image;
use image::{DynamicImage, ImageBuffer, Rgba};

mod xclip_wrapper;
use xclip_wrapper::get_clipboard_image;

struct Layer {
    texture: Texture,
    rect: Rect,
}

struct TextLayer {
    layer: Option<Layer>,
    text: String,
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Selection {
    layer_index: usize,
    x_offset: i32,
    y_offset: i32,
}

#[derive(Debug)]
enum Mode {
    Normal,
    TextInput,
}

#[derive(Debug)]
struct State {
    mouse: Point,
    w: i32,
    h: i32,
    selection: Option<Selection>,
    mode: Mode,
}

impl State {
    fn new() -> Self {
        Self {
            mouse: Point::new(0, 0),
            w: 0,
            h: 0,
            selection: None,
            mode: Mode::Normal,
        }
    }
}

impl TextLayer {
    fn new(x: i32, y: i32) -> Self {
        Self {
            layer: None,
            text: String::new(),
            x: x,
            y: y,
        }
    }
}

fn layer_from_text(canvas: &mut WindowCanvas, text: &str, x: i32, y: i32) -> Option<Layer> {
    let font_data = include_bytes!("../data/Roboto-Regular.ttf");
    let collection = FontCollection::from_bytes(font_data as &[u8]).unwrap();
    let font = collection.into_font().unwrap();
    let scale = Scale::uniform(32.0);
    let color = (150, 0, 0);
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font
        .layout(text, scale, point(20.0, 20.0 + v_metrics.ascent))
        .collect();
    // work out the layout size
    let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
    let glyphs_width = glyphs
        .iter()
        .rev()
        .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
        .next()
        .unwrap_or(0.0)
        .ceil() as u32;

    // Create a new rgba image with some padding
    let mut image = DynamicImage::new_rgba8(glyphs_width + 40, glyphs_height + 40).to_rgba();

    // Loop through the glyphs in the text, positing each one on a line
    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            // Draw the glyph into the image per-pixel by using the draw closure
            glyph.draw(|x, y, v| {
                image.put_pixel(
                    // Offset the position by the glyph bounding box
                    x + bounding_box.min.x as u32,
                    y + bounding_box.min.y as u32,
                    // Turn the coverage into an alpha value
                    Rgba([color.0, color.1, color.2, (v * 255.0) as u8]),
                )
            });
        }
    }
    layer_from_image(canvas, image, x, y)
}

fn layer_from_clipboard(canvas: &mut WindowCanvas, x: i32, y: i32) -> Option<Layer> {
    if let Some(b) = get_clipboard_image() {
        let image = image::load_from_memory(&b).unwrap().to_rgba();
        return layer_from_image(canvas, image, x, y);
    }
    None
}

fn layer_from_image(
    canvas: &mut WindowCanvas,
    mut image: ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
) -> Option<Layer> {
    let (w, h) = image.dimensions();
    let surface = Surface::from_data(&mut image, w, h, 4 * w, PixelFormatEnum::RGBA32).unwrap();
    let texture = canvas
        .texture_creator()
        .create_texture_from_surface(&surface)
        .unwrap();
    let rect = Rect::new(x, y, w, h);
    Some(Layer { texture, rect })
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let window = video_subsys
        .window("Image Editor", 800, 600)
        .resizable()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    let mut layers: Vec<Layer> = Vec::new();
    let mut text_layers: Vec<TextLayer> = Vec::new();
    let mut active_text_layer_index: Option<usize> = None;
    let mut state = State::new();

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                Event::MouseMotion {
                    x, y, mousestate, ..
                } => {
                    state.mouse = Point::new(x, y);
                    if mousestate.left() {
                        if let Some(selection) = &state.selection {
                            if let Some(layer) = layers.get_mut(selection.layer_index) {
                                layer.rect.x = x - selection.x_offset;
                                layer.rect.y = y - selection.y_offset;
                            }
                        }
                    }
                }
                Event::MouseButtonDown {
                    x, y, mouse_btn, ..
                } => {
                    state.mouse = Point::new(x, y);
                    if mouse_btn == MouseButton::Left {
                        for (i, layer) in &mut layers.iter().enumerate() {
                            if layer.rect.contains_point(state.mouse) {
                                let x_offset = x - layer.rect.x;
                                let y_offset = y - layer.rect.y;
                                state.selection = Some(Selection {
                                    layer_index: i,
                                    x_offset,
                                    y_offset,
                                });
                            }
                        }
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    if mouse_btn == MouseButton::Left {
                        state.selection = None;
                    }
                }
                Event::Window { win_event, .. } => {
                    if let WindowEvent::Resized(w, h) = win_event {
                        state.w = w;
                        state.h = h;
                    }
                }
                Event::TextInput { text, .. } => match state.mode {
                    Mode::TextInput => {
                        if let Some(i) = active_text_layer_index {
                            if let Some(mut text_layer) = text_layers.get_mut(i) {
                                text_layer.text.push_str(&text);
                                text_layer.layer = layer_from_text(
                                    &mut canvas,
                                    &text_layer.text,
                                    text_layer.x,
                                    text_layer.y,
                                );
                            }
                        }
                    }
                    Mode::Normal => {
                        if &text == "t" {
                            text_layers.push(TextLayer::new(state.mouse.x, state.mouse.y));
                            active_text_layer_index = Some(text_layers.len() - 1);
                            state.mode = Mode::TextInput;
                        }
                    }
                },
                Event::KeyDown {
                    keycode: Some(kc),
                    keymod,
                    ..
                } => {
                    if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
                        if kc == Keycode::V {
                            if let Some(layer) =
                                layer_from_clipboard(&mut canvas, state.mouse.x, state.mouse.y)
                            {
                                layers.push(layer);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(200, 180, 100));
        canvas.clear();
        for layer in &layers {
            let src_rect = Rect::new(0, 0, layer.rect.width(), layer.rect.height());
            canvas
                .copy(&layer.texture, Some(src_rect), Some(layer.rect))
                .unwrap();
        }

        for text_layer in &text_layers {
            if let Some(layer) = &text_layer.layer {
                let src_rect = Rect::new(0, 0, layer.rect.width(), layer.rect.height());
                canvas
                    .copy(&layer.texture, Some(src_rect), Some(layer.rect))
                    .unwrap();
            }
        }

        canvas.present();
    }
}
