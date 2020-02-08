extern crate sdl2;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::{Point, Rect};
use sdl2::render::{Texture, WindowCanvas};
use sdl2::surface::Surface;

extern crate image;

mod xclip_wrapper;
use xclip_wrapper::get_clipboard_image;

struct Layer {
    texture: Texture,
    rect: Rect,
}

#[derive(Debug)]
struct Selection {
    layer_index: usize,
    x_offset: i32,
    y_offset: i32,
}

#[derive(Debug)]
struct State {
    mouse: Point,
    w: i32,
    h: i32,
    selection: Option<Selection>,
}

impl State {
    fn new() -> Self {
        Self {
            mouse: Point::new(0, 0),
            w: 0,
            h: 0,
            selection: None,
        }
    }
}

fn layer_from_clipboard(canvas: &mut WindowCanvas, x: i32, y: i32) -> Option<Layer> {
    if let Some(b) = get_clipboard_image() {
        let mut img = image::load_from_memory(&b).unwrap().into_rgba();
        let (w, h) = img.dimensions();
        let surface = Surface::from_data(&mut img, w, h, 4 * w, PixelFormatEnum::RGBA32).unwrap();
        let texture = canvas
            .texture_creator()
            .create_texture_from_surface(&surface)
            .unwrap();
        let rect = Rect::new(x, y, w, h);
        return Some(Layer { texture, rect });
    }
    None
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let window = video_subsys
        .window("Neovim", 800, 600)
        .resizable()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();

    let mut layers: Vec<Layer> = Vec::new();
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

        canvas.present();
    }
}
