use kittens_render::blocking::BlockingWritePermit;

fn escape<'a>(permit: BlockingWritePermit<'a>) -> BlockingWritePermit<'static> {
    permit
}

fn main() {}
