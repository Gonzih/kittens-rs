use kittens_render::blocking::BlockingRegionWrite;
use kittens_render::geometry::Region;

fn write_without_target<W: BlockingRegionWrite>(writer: W, region: Region, pixels: &[u8]) {
    let _ = writer.write_region_admitted(region, pixels);
}

fn main() {}
