use kittens_render::blocking::{BlockingRegionWrite, BlockingWritePermit};
use kittens_render::geometry::Region;

struct ExternalWriter;

impl BlockingRegionWrite for ExternalWriter {
    type Error = ();

    fn write_region_admitted(
        self,
        _region: Region,
        _pixels: &[u8],
        _permit: BlockingWritePermit<'_>,
    ) -> (Self, Result<(), Self::Error>) {
        (self, Ok(()))
    }
}

fn main() {}
