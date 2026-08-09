//! Negative control: `TouchReader` is intentionally open during this slice.
//! Rust can require one snapshot-returning call, but it cannot prove that an
//! implementation obtained all fields in one physical bus transaction.

use core::convert::Infallible;

use kittens_render::touch::{TouchPoint, TouchReader, TouchReport};

struct TwoTransactionReader {
    x_register: u16,
    y_register: u16,
}

impl TwoTransactionReader {
    fn read_x_transaction(&self) -> u16 {
        self.x_register
    }

    fn read_y_transaction(&self) -> u16 {
        self.y_register
    }
}

impl TouchReader for TwoTransactionReader {
    type Error = Infallible;

    fn read_snapshot(&mut self) -> Result<TouchReport, Self::Error> {
        Ok(TouchReport {
            points: [
                Some(TouchPoint {
                    id: 0,
                    x: self.read_x_transaction(),
                    y: self.read_y_transaction(),
                }),
                None,
            ],
        })
    }

    fn int_asserted(&self) -> bool {
        false
    }
}

fn main() {
    let mut reader = TwoTransactionReader {
        x_register: 1,
        y_register: 2,
    };
    let _accepted = reader.read_snapshot();
}
