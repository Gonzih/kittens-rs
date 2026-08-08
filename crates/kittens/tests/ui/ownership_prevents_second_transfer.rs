struct Display;
struct Framebuffer;

fn submit(_display: Display, _framebuffer: Framebuffer) {}

fn main() {
    let display = Display;
    let framebuffer = Framebuffer;
    submit(display, framebuffer);
    submit(display, Framebuffer);
}
