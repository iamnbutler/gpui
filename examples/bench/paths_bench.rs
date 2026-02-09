use gpui::{
    Application, Background, Bounds, ColorSpace, Context, Path, PathBuilder, Pixels, Point2, Px,
    Render, TitlebarOptions, Window, WindowBounds, WindowOptions, canvas, div, linear_color_stop,
    linear_gradient, prelude::*, px, rgb, size,
};

const DEFAULT_WINDOW_WIDTH: Pixels = px(1024.0);
const DEFAULT_WINDOW_HEIGHT: Pixels = px(768.0);

struct PaintingViewer {
    default_lines: Vec<(Path<Pixels>, Background)>,
    _painting: bool,
}

impl PaintingViewer {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut lines = vec![];

        // draw a lightening bolt ⚡
        for _ in 0..2000 {
            // draw a ⭐
            let mut builder = PathBuilder::fill();
            builder.move_to(Point2::<Px>::new(350., 100.));
            builder.line_to(Point2::<Px>::new(370., 160.));
            builder.line_to(Point2::<Px>::new(430., 160.));
            builder.line_to(Point2::<Px>::new(380., 200.));
            builder.line_to(Point2::<Px>::new(400., 260.));
            builder.line_to(Point2::<Px>::new(350., 220.));
            builder.line_to(Point2::<Px>::new(300., 260.));
            builder.line_to(Point2::<Px>::new(320., 200.));
            builder.line_to(Point2::<Px>::new(270., 160.));
            builder.line_to(Point2::<Px>::new(330., 160.));
            builder.line_to(Point2::<Px>::new(350., 100.));
            let path = builder.build().unwrap();
            lines.push((
                path,
                linear_gradient(
                    180.,
                    linear_color_stop(rgb(0xFACC15), 0.7),
                    linear_color_stop(rgb(0xD56D0C), 1.),
                )
                .color_space(ColorSpace::Oklab),
            ));
        }

        Self {
            default_lines: lines,
            _painting: false,
        }
    }
}

impl Render for PaintingViewer {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        window.request_animation_frame();
        let lines = self.default_lines.clone();
        div().size_full().child(
            canvas(
                move |_, _, _| {},
                move |_, _, window, _| {
                    for (path, color) in lines {
                        window.paint_path(path, color);
                    }
                },
            )
            .size_full(),
        )
    }
}

fn main() {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Vulkan".into()),
                    ..Default::default()
                }),
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| PaintingViewer::new(window, cx)),
        )
        .unwrap();
        cx.activate(true);
    });
}
