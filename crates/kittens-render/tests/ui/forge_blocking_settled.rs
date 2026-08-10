use kittens_render::blocking::BlockingSettled;
use kittens_render::sweep::StripeTarget;

fn forge<T, P, E>(
    writer: T,
    pixels: P,
    result: Result<(), E>,
    target: StripeTarget,
) -> BlockingSettled<T, P, E> {
    BlockingSettled {
        writer,
        pixels,
        result,
        target,
    }
}

fn main() {}
