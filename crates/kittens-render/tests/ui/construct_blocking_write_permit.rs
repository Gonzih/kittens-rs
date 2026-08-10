use kittens_render::blocking::BlockingWritePermit;

fn forge_permit() {
    let mut key = ();
    let _permit = BlockingWritePermit::new(&mut key);
}

fn main() {}
