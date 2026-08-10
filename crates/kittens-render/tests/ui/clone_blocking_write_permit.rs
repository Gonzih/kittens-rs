use kittens_render::blocking::BlockingWritePermit;

fn clone_permit(permit: BlockingWritePermit<'_>) {
    let _copy = permit.clone();
}

fn main() {}
