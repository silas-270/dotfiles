fn main() {
    let mut dest = tiny_skia::Pixmap::new(200, 200).unwrap();
    let mut src = tiny_skia::Pixmap::new(50, 50).unwrap();
    src.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    
    let transform = tiny_skia::Transform::from_scale(1.0, 1.0).post_translate(100.0, 100.0);
    dest.draw_pixmap(0, 0, src.as_ref(), &tiny_skia::PixmapPaint::default(), transform, None);
    
    dest.save_png("test.png").unwrap();
}
